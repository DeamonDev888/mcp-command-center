use crate::types::ToolInfo;
use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};
use std::net::{SocketAddr, TcpStream};
#[cfg_attr(not(windows), allow(unused_imports))] // Stdio : chemins cfg(windows)
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

/// En-têtes utiles d'une réponse HTTP (inférence du langage serveur + session MCP).
#[derive(Default, Clone)]
pub struct RespHead {
    pub content_type: String,
    pub server: String,
    pub powered_by: String,
    pub session_id: String,
}

impl RespHead {
    pub fn sse(&self) -> bool {
        self.content_type.contains("text/event-stream")
    }
}

pub struct McpProbe {
    pub mcp_up: bool,
    #[allow(dead_code)] // réservé : distinguer port fermé vs MCP muet côté UI
    pub port_open: bool,
    pub pid: Option<u32>,
    pub host: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub protocol: Option<String>,
    /// Réponses JSON-RPC encodées SSE (Content-Type: text/event-stream sur
    /// l'initialize). NE suffit PAS à qualifier un transport SSE : FastMCP
    /// httpStream répond par défaut ainsi (enableJsonResponse=false) tout en
    /// restant du streamable HTTP.
    pub sse: bool,
    /// Endpoint SSE legacy co-servé (GET /sse → 200 + `event:`). Vrai sur un
    /// FastMCP httpStream (compat 2024-11-05) ET sur un vrai serveur SSE.
    pub sse_endpoint: bool,
    /// Le serveur accepte une upgrade WebSocket (101 à un GET avec
    /// `Upgrade: websocket`) — badge de transport « WS » dans l'UI.
    pub ws: bool,
    /// Langage inféré (stdio : la commande ; HTTP : en-têtes Server/X-Powered-By).
    /// La spec MCP n'expose PAS le langage dans serverInfo.
    pub lang: Option<String>,
    /// Framework détecté par fingerprint — "litemcp" si le serveur répond
    /// initialize avec la version 2026-07-28 SANS négocier (le probe demande
    /// 2024-11-05, litemcp ignore la version demandée) ET que sa sonde
    /// GET /health répond 200 « ok » (deux signatures documentées de litemcp).
    pub framework: Option<String>,
    pub tools: Vec<ToolInfo>,
    pub latency_ms: u64,
    pub error: Option<String>,
}

impl McpProbe {
    pub fn fail(err: String, port_open: bool) -> McpProbe {
        McpProbe {
            mcp_up: false,
            port_open,
            pid: None,
            host: None,
            name: None,
            version: None,
            protocol: None,
            sse: false,
            sse_endpoint: false,
            ws: false,
            lang: None,
            framework: None,
            tools: vec![],
            latency_ms: 0,
            error: Some(err),
        }
    }
}

/// Handle d'un serveur stdio maintenu vivant (mode "start").
pub struct StdioHandle {
    pub child: Child,
    #[allow(dead_code)] // réservé : tools/call sur stdio dans une future version
    pub stdin: Option<ChildStdin>,
}

fn resolve_windows_cmd(program: &str) -> String {
    // Windows : CreateProcess ne résout que les .exe — npx/uvx sont des .cmd → introuvables.
    // Retourne <prog>.cmd si présent dans le PATH, sinon le programme tel quel.
    if !program.contains('.') {
        let path_var = std::env::var("PATH").unwrap_or_default();
        for dir in path_var.split(';') {
            if dir.is_empty() {
                continue;
            }
            let cand = std::path::Path::new(dir).join(format!("{}.cmd", program));
            if cand.exists() {
                return cand.to_string_lossy().to_string();
            }
        }
    }
    program.to_string()
}

/// Applique CREATE_NO_WINDOW : aucune fenêtre console ne s'ouvre, même quand le
/// parent est une app GUI (Tauri). À utiliser pour TOUT process console lancé par l'app.
pub fn hidden(cmd: &mut std::process::Command) -> &mut std::process::Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

/// Tue l'arbre complet d'un PID (cmd.exe → node.exe). `Child::kill()` ne tue que
/// le lanceur .cmd : les serveurs détachés survivent et gardent leur console.
pub fn kill_tree(pid: u32) {
    #[cfg(windows)]
    {
        let mut c = std::process::Command::new("taskkill");
        let _ = hidden(&mut c)
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
    }
}

/// Tue le process qui ÉCOUTE réellement sur un port local (méthode du launcher
/// MCP MONITOR v7.3) : `child.kill()` ne tue que le lanceur, pas le serveur
/// détaché qu'il a spawné. netstat -ano → PIDs LISTENING → taskkill /F /PID.
/// PIDs écoutant EXACTEMENT sur `port` (parse par colonne : ":3141" ne matche plus
/// ":31410"), en excluant notre propre PID (on ne se suicide pas si l'id vise notre port).
pub fn pids_listening_on(netstat_text: &str, port: u16) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for line in netstat_text.lines() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        if toks.len() < 5 || !toks.contains(&"LISTENING") {
            continue;
        }
        let Some(addr) = toks.get(1) else { continue };
        if addr.rsplit(':').next().and_then(|p| p.parse::<u16>().ok()) != Some(port) {
            continue;
        }
        if let Some(pid) = toks.last().and_then(|t| t.parse::<u32>().ok()) {
            if pid != std::process::id() && !out.contains(&pid) {
                out.push(pid);
            }
        }
    }
    out
}

#[cfg(windows)]
pub fn kill_port(port: u16) {
    let mut c = Command::new("netstat");
    if let Ok(out) = hidden(&mut c).args(["-ano"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for pid in pids_listening_on(&text, port) {
            let mut t = Command::new("taskkill");
            let _ = hidden(&mut t)
                .args(["/F", "/PID", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

/// Parse la sortie `ss -tlnp` : PIDs (notre propre PID exclu) des listeners sur
/// `port` EXACT. Format : `LISTEN 0 128 0.0.0.0:8080 0.0.0.0:* users:(("node",pid=1234,fd=20))`
pub fn pids_listening_on_ss(text: &str, port: u16) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for line in text.lines() {
        if !line.contains("LISTEN") {
            continue;
        }
        let on_port = line.split_whitespace().any(|t| {
            t.rsplit_once(':')
                .map(|(_, p)| p.parse::<u16>() == Ok(port))
                .unwrap_or(false)
        });
        if !on_port {
            continue;
        }
        for tok in line.split_whitespace() {
            for seg in tok.split(',') {
                if let Some(pid) = seg.strip_prefix("pid=") {
                    if let Ok(pid) = pid.trim_end_matches(')').parse::<u32>() {
                        if pid != std::process::id() && !out.contains(&pid) {
                            out.push(pid);
                        }
                    }
                }
            }
        }
    }
    out
}

/// Unix : tue le process qui ÉCOUTE réellement sur un port (parité Windows).
///
/// 1) `ss -tlnp` (iproute2) — sans privilèges, seuls les process de
///    l'utilisateur sont visibles ;
/// 2) repli `lsof -ti :PORT -sTCP:LISTEN` (macOS).
#[cfg(not(windows))]
pub fn kill_port(port: u16) {
    let mut c = Command::new("ss");
    probe_hidden_cmd(&mut c);
    if let Ok(out) = c.args(["-tlnp"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        let pids = pids_listening_on_ss(&text, port);
        if !pids.is_empty() {
            for pid in pids {
                kill_pid_unix(pid);
            }
            return;
        }
    }
    let mut c = Command::new("lsof");
    probe_hidden_cmd(&mut c);
    if let Ok(out) = c
        .args(["-ti", &format!(":{}", port), "-sTCP:LISTEN"])
        .output()
    {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    if pid != std::process::id() {
                        kill_pid_unix(pid);
                    }
                }
            }
        }
    }
}

#[cfg(not(windows))]
fn kill_pid_unix(pid: u32) {
    let mut c = Command::new("kill");
    probe_hidden_cmd(&mut c);
    let _ = c.arg("-9").arg(pid.to_string()).status();
}

/// Spawn un process serveur (HTTP/HTTPS lancé par MCP Control) sans handshake
/// JSON-RPC : le probe HTTP classique fera le handshake. stdout/stderr nullés
/// (pas de pipe = pas de risque de blocage buffer plein).
pub fn spawn_process(
    program: &str,
    args: &[String],
    workdir: Option<&str>,
) -> Result<StdioHandle, String> {
    let program = resolve_windows_cmd(program);
    let mut cmd = Command::new(&program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS :
        // l'enfant survit au shell/lanceur et à un Ctrl+C dans la console de l'app.
        cmd.creation_flags(0x0800_0000 | 0x0000_0200 | 0x0000_0008);
    }
    if let Some(wd) = workdir {
        cmd.current_dir(wd);
    }
    let child = cmd
        .spawn()
        .map_err(|e| format!("spawn {}: {}", program, e))?;
    Ok(StdioHandle { child, stdin: None })
}

/// Langage inféré d'un serveur stdio depuis sa commande de lancement.
/// (La spec MCP n'expose pas le langage : serverInfo = name/title/version only.)
pub fn detect_lang_stdio(program: &str, args: &[String]) -> Option<String> {
    let p = program
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(program)
        .to_lowercase();
    let p = p
        .trim_end_matches(".exe")
        .trim_end_matches(".cmd")
        .trim_end_matches(".bat");
    let lang = match p {
        "node" | "npx" | "bun" | "deno" | "tsx" | "ts-node" => "node",
        "python" | "python3" | "py" | "uv" | "uvx" | "pipx" | "poetry" | "pdm" => "python",
        "cargo" | "rustc" => "rust",
        "go" => "go",
        "java" | "javaw" | "mvn" | "gradle" => "java",
        "dotnet" => "dotnet",
        "ruby" | "bundle" => "ruby",
        "php" => "php",
        _ => {
            // binaire inconnu : indices dans les args (script .py/.js/.ts)
            let joined = args.join(" ").to_lowercase();
            if joined.contains(".py") {
                "python"
            } else if joined.contains(".js") || joined.contains(".ts") || joined.contains(".mjs") {
                "node"
            } else {
                return None;
            }
        }
    };
    Some(lang.to_string())
}

/// Langage inféré d'un serveur HTTP depuis les en-têtes de réponse
/// (Server / X-Powered-By) ou, à défaut, le nom du serveur MCP.
pub fn detect_lang_http(head: &RespHead, name: Option<&str>) -> Option<String> {
    let h = format!("{} {}", head.server, head.powered_by).to_lowercase();
    let lang = if [
        "uvicorn",
        "hypercorn",
        "gunicorn",
        "werkzeug",
        "aiohttp",
        "starlette",
        "fastapi",
        "quart",
    ]
    .iter()
    .any(|k| h.contains(k))
    {
        "python"
    } else if ["express", "next.js", "fastify", "node"]
        .iter()
        .any(|k| h.contains(k))
    {
        "node"
    } else if ["hyper", "axum", "actix", "rocket", "warp", "salvo"]
        .iter()
        .any(|k| h.contains(k))
    {
        "rust"
    } else if ["kestrel", "asp.net"].iter().any(|k| h.contains(k)) {
        "dotnet"
    } else if ["jetty", "tomcat", "undertow", "netty"]
        .iter()
        .any(|k| h.contains(k))
    {
        "java"
    } else if ["net/http", "gofiber", "fasthttp"]
        .iter()
        .any(|k| h.contains(k))
    {
        "go"
    } else {
        // pas d'en-tête exploitable : indices dans le nom du serveur
        let n = name.unwrap_or("").to_lowercase();
        if n.contains("rust") {
            "rust"
        } else if n.contains("python") {
            "python"
        } else {
            return None;
        }
    };
    Some(lang.to_string())
}

/// Langage inféré du process qui écoute réellement un port local
/// (Windows : netstat -ano → PID → tasklist → image). Cache 10 s.
#[cfg(windows)]
pub fn port_owner_lang(port: u16) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};
    static CACHE: OnceLock<Mutex<(Instant, HashMap<u16, String>)>> = OnceLock::new();
    let cache = CACHE
        .get_or_init(|| Mutex::new((Instant::now() - Duration::from_secs(60), HashMap::new())));
    if let Ok(g) = cache.lock() {
        let (t, map) = &*g;
        if t.elapsed() < Duration::from_secs(10) {
            return map.get(&port).and_then(|img| detect_lang_from_image(img));
        }
    }
    let map = build_port_owner_map();
    let lang = map.get(&port).and_then(|img| detect_lang_from_image(img));
    if let Ok(mut g) = cache.lock() {
        *g = (Instant::now(), map);
    }
    lang
}

#[cfg(not(windows))]
pub fn port_owner_lang(_port: u16) -> Option<String> {
    None
}

/// port → nom d'image du process propriétaire (Windows).
#[cfg(windows)]
fn build_port_owner_map() -> std::collections::HashMap<u16, String> {
    use std::collections::HashMap;
    use std::process::Command;
    let mut ports: Vec<(u16, u32)> = Vec::new();
    let mut c = Command::new("netstat");
    if let Ok(out) = hidden(&mut c).args(["-ano"]).output() {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let toks: Vec<&str> = line.split_whitespace().collect();
            if toks.len() >= 5 && toks.contains(&"LISTENING") {
                if let (Some(addr), Some(pid)) = (toks.get(1), toks.last()) {
                    if let (Some(p), Ok(pid)) = (
                        addr.rsplit(':').next().and_then(|p| p.parse::<u16>().ok()),
                        pid.parse::<u32>(),
                    ) {
                        ports.push((p, pid));
                    }
                }
            }
        }
    }
    let mut names: HashMap<u32, String> = HashMap::new();
    let mut t = Command::new("tasklist");
    if let Ok(out) = hidden(&mut t).args(["/FO", "CSV", "/NH"]).output() {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let cols: Vec<&str> = line.split("\",\"").collect();
            if cols.len() > 1 {
                let name = cols[0].trim_matches('"').to_lowercase();
                if let Ok(pid) = cols[1].trim_matches('"').parse::<u32>() {
                    names.insert(pid, name);
                }
            }
        }
    }
    ports
        .into_iter()
        .filter_map(|(p, pid)| names.get(&pid).map(|n| (p, n.clone())))
        .collect()
}

/// Image de process → langage (node.exe, python.exe, java.exe…).
pub fn detect_lang_from_image(image: &str) -> Option<String> {
    let i = image.to_lowercase();
    let i = i.trim_end_matches(".exe");
    let lang = if ["node", "deno", "bun"].contains(&i) {
        "node"
    } else if i.starts_with("python") {
        "python"
    } else if ["java", "javaw"].contains(&i) {
        "java"
    } else if i == "dotnet" {
        "dotnet"
    } else if i == "ruby" {
        "ruby"
    } else if i == "php" {
        "php"
    } else {
        return None;
    };
    Some(lang.to_string())
}

fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > hay.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

pub fn socket_addr(host: &str, port: u16) -> Result<SocketAddr, std::net::AddrParseError> {
    // crochets uniquement pour IPv6 ("[::1]:p") ; "127.0.0.1:p" tel quel
    let s = if host.contains(':') {
        format!("[{}]:{}", host, port)
    } else {
        format!("{}:{}", host, port)
    };
    s.parse()
}

pub fn port_open(host: &str, port: u16, timeout: Duration) -> bool {
    match socket_addr(host, port) {
        Ok(addr) => TcpStream::connect_timeout(&addr, timeout).is_ok(),
        Err(_) => false,
    }
}

/// En-tête `Host` conforme RFC 7230 §5.4 : crochets obligatoires sur un littéral IPv6.
fn http_host_header(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{}]:{}", host, port)
    } else {
        format!("{}:{}", host, port)
    }
}

/// Sanitize un chemin HTTP avant insertion dans la request-line (anti-CRLF / request splitting).
fn sanitize_http_path<'a>(path: &'a str, fallback: &'a str) -> &'a str {
    if !path.is_empty()
        && path.starts_with('/')
        && path.chars().all(|c| !c.is_control() && c != ' ')
    {
        path
    } else {
        fallback
    }
}

/// Le endpoint SSE legacy (GET, généralement /sse) sert-il un flux SSE ?
/// Un 200 dont le début de réponse contient `event:` = flux SSE ouvert.
/// Seule signature fiable du transport SSE legacy : un FastMCP httpStream
/// co-sert /sse (compat 2024-11-05), donc ce flag n'a de sens qu'associé au
/// résultat du POST initialize (répondu = streamable HTTP, quelle que soit
/// l'encodage de ses réponses).
pub fn sse_get_host(host: &str, port: u16, path: &str, timeout_ms: u64) -> bool {
    let addr = match socket_addr(host, port) {
        Ok(a) => a,
        Err(_) => return false,
    };
    let timeout = Duration::from_millis(timeout_ms);
    let mut stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(s) => s,
        Err(_) => return false,
    };
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
    let path = sanitize_http_path(path, "/sse");
    let host_hdr = http_host_header(host, port);
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nAccept: text/event-stream\r\nConnection: close\r\n\r\n",
        path, host_hdr
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut data = Vec::new();
    let mut tmp = [0u8; 4096];
    let deadline = Instant::now() + timeout;
    loop {
        let remain = deadline.saturating_duration_since(Instant::now());
        if remain.is_zero() {
            break;
        }
        stream.set_read_timeout(Some(remain)).ok();
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                data.extend_from_slice(&tmp[..n]);
                // premier événement reçu : suffit à qualifier le flux
                if find_sub(&data, b"event:").is_some() {
                    break;
                }
                if data.len() > MAX_PROBE_READ_SMALL {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if data.is_empty() {
        return false;
    }
    let head = String::from_utf8_lossy(&data).to_lowercase();
    (head.starts_with("http/1.1 2") || head.starts_with("http/1.0 2"))
        && find_sub(&data, b"event:").is_some()
}

/// sse_get_host sur ::1 puis 127.0.0.1 (binds FastMCP = IPv6-only).
pub fn sse_get(port: u16, path: &str, timeout_ms: u64) -> bool {
    sse_get_host("::1", port, path, timeout_ms) || sse_get_host("127.0.0.1", port, path, timeout_ms)
}

/// Le serveur accepte-t-il une upgrade WebSocket sur `path` ? Envoie un GET
/// avec les en-têtes `Upgrade: websocket` et regarde si la réponse est un
/// `101 Switching Protocols`. Détection seule : aucun frame WS n'est échangé.
/// Sert au badge de transport « WS » de l'UI (les trois modes affichés sont
/// stdio, http et ws).
pub fn ws_upgrade_host(host: &str, port: u16, path: &str, timeout_ms: u64) -> bool {
    let addr = match socket_addr(host, port) {
        Ok(a) => a,
        Err(_) => return false,
    };
    let timeout = Duration::from_millis(timeout_ms.clamp(150, 1500));
    let mut stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(s) => s,
        Err(_) => return false,
    };
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
    let path = sanitize_http_path(path, "/mcp");
    let host_hdr = http_host_header(host, port);
    // clé factice RFC 6455 (16 octets base64) — le serveur peut répondre 101
    // sans qu'on poursuive le protocole.
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n",
        path, host_hdr
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut data = Vec::new();
    let mut tmp = [0u8; 2048];
    // une seule lecture suffit : la ligne de statut arrive en tête de réponse
    match stream.read(&mut tmp) {
        Ok(n) if n > 0 => data.extend_from_slice(&tmp[..n]),
        _ => return false,
    }
    let head = String::from_utf8_lossy(&data).to_lowercase();
    head.starts_with("http/1.1 101") || head.starts_with("http/1.0 101")
}

/// Passerelles par défaut IPv4 de la machine (0 à 2 entrées).
/// Sert à mesurer si le réseau LOCAL est joignable : quand un VPN capture le
/// trafic (kill switch), la passerelle LAN ne répond plus alors que la machine
/// est bien connectée — les serveurs distants paraissent alors « en panne ».
/// Ajoute `cand` à `out` si c'est une IPv4 valide non nulle et pas déjà vue.
fn push_gateway(out: &mut Vec<String>, cand: &str) {
    let ip = cand.trim().trim_matches(|c: char| c == '(' || c == ')');
    let ok = ip.split('.').count() == 4
        && ip.split('.').all(|o| o.parse::<u8>().is_ok())
        && ip != "0.0.0.0";
    if ok && !out.iter().any(|x| x == ip) {
        out.push(ip.to_string());
    }
}

pub fn default_gateways() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if cfg!(windows) {
        let mut c = Command::new("ipconfig");
        hidden(&mut c);
        if let Ok(o) = c.output() {
            let text = String::from_utf8_lossy(&o.stdout).to_string();
            for line in text.lines() {
                let low = line.to_lowercase();
                if (low.contains("passerelle") || low.contains("default gateway"))
                    && line.contains(':')
                {
                    if let Some(v) = line.split(':').nth(1) {
                        push_gateway(&mut out, v);
                    }
                }
            }
        }
    } else {
        let mut c = Command::new("ip");
        probe_hidden_cmd(&mut c);
        if let Ok(o) = c.args(["route", "show", "default"]).output() {
            let text = String::from_utf8_lossy(&o.stdout).to_string();
            for line in text.lines() {
                let toks: Vec<&str> = line.split_whitespace().collect();
                if let Some(i) = toks.iter().position(|t| *t == "via") {
                    if let Some(v) = toks.get(i + 1) {
                        push_gateway(&mut out, v);
                    }
                }
            }
        }
        if out.is_empty() {
            // macOS
            let mut c2 = Command::new("netstat");
            probe_hidden_cmd(&mut c2);
            if let Ok(o) = c2.args(["-rn"]).output() {
                let text = String::from_utf8_lossy(&o.stdout).to_string();
                for line in text.lines() {
                    let low = line.to_lowercase();
                    if low.starts_with("default") {
                        if let Some(v) = line.split_whitespace().nth(1) {
                            push_gateway(&mut out, v);
                        }
                    }
                }
            }
        }
    }
    out
}

/// Ping ICMP court (1 paquet) — vrai si la cible répond. Utilisé pour vérifier
/// la passerelle LAN ; jamais bloquant au-delà de `timeout_ms`.
pub fn ping_ok(host: &str, timeout_ms: u64) -> bool {
    let ms = timeout_ms.clamp(200, 3000);
    let mut c = Command::new("ping");
    hidden(&mut c);
    let status = if cfg!(windows) {
        c.args(["-n", "1", "-w", &ms.to_string(), host]).output()
    } else {
        // -W : secondes sous Linux, millisecondes sous macOS — 1 reste sûr.
        c.args(["-c", "1", "-W", "1", host]).output()
    };
    matches!(status, Ok(o) if o.status.success())
}

/// Le réseau local est-il joignable ? (au moins une passerelle répond)
/// `None` = aucune passerelle détectée (pas de réseau local à surveiller).
pub fn lan_reachable() -> Option<bool> {
    let gws = default_gateways();
    if gws.is_empty() {
        return None;
    }
    Some(gws.iter().any(|g| ping_ok(g, 800)))
}

/// GET /health sur un hôte précis — vrai si 2xx avec un corps contenant « ok ».
/// Sonde de vie documentée de litemcp ; second signal du fingerprint du framework.
pub fn health_ok_host(host: &str, port: u16, timeout_ms: u64) -> bool {
    let addr = match socket_addr(host, port) {
        Ok(a) => a,
        Err(_) => return false,
    };
    let timeout = Duration::from_millis(timeout_ms);
    let mut stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(s) => s,
        Err(_) => return false,
    };
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
    let host_hdr = http_host_header(host, port);
    let req = format!(
        "GET /health HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        host_hdr
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut data = Vec::new();
    let mut tmp = [0u8; 2048];
    let deadline = Instant::now() + timeout;
    loop {
        let remain = deadline.saturating_duration_since(Instant::now());
        if remain.is_zero() {
            break;
        }
        stream.set_read_timeout(Some(remain)).ok();
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                data.extend_from_slice(&tmp[..n]);
                if data.len() > MAX_PROBE_READ_SMALL {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let sep = match find_sub(&data, b"\r\n\r\n") {
        Some(p) => p,
        None => return false,
    };
    let head = String::from_utf8_lossy(&data[..sep]).to_lowercase();
    (head.starts_with("http/1.1 2") || head.starts_with("http/1.0 2"))
        && String::from_utf8_lossy(&data[sep + 4..])
            .to_lowercase()
            .contains("ok")
}

/// Fingerprint du framework serveur (pur, testé) : litemcp répond initialize
/// avec la version 2026-07-28 sans négocier la version demandée par le client
/// ET expose GET /health → 200 « ok ». Les deux signatures à la fois écartent
/// les serveurs qui partagent l'une ou l'autre (specs à jour, health endpoints
/// génériques).
/// Ancienne API conservée pour les tests : signature comportementale seule.
#[cfg(test)]
fn detect_framework(
    protocol: Option<&str>,
    health: bool,
    sse: bool,
    sse_endpoint: bool,
) -> Option<&'static str> {
    crate::frameworks::detect_signature(protocol, health, sse, sse_endpoint).map(|f| f.id)
}

/// Sonde un port sur les chemins MCP usuels, dans l'ordre : /mcp, /, /api/mcp.
/// Retourne le premier chemin qui répond + sa sonde complète — un serveur
/// monté à la racine ou sous /api/mcp est découvert, pas seulement /mcp.
pub fn probe_mcp_any_path(port: u16, timeout_ms: u64) -> Option<(&'static str, McpProbe)> {
    for path in ["/mcp", "/", "/api/mcp"] {
        let p = probe_http(port, path, timeout_ms);
        if p.mcp_up {
            return Some((path, p));
        }
    }
    None
}

// ---------- découverte SSH d'une machine distante ----------

/// Sanitize d'un hôte/utilisateur pour la ligne de commande ssh (anti-injection :
/// ces valeurs arrivent de l'UI). Hôte : lettres/chiffres/point/tiret/deux-points
/// (IPv6) ; utilisateur : lettres/chiffres/point/tiret/underscore.
fn ssh_safe(s: &str) -> Option<String> {
    let ok = !s.is_empty()
        && s.len() <= 253
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'));
    if ok {
        Some(s.to_string())
    } else {
        None
    }
}

/// Ports système écartés de la découverte (écouteurs Windows/Linux sans MCP).
const SSH_PORTS_EXCLUDES: &[u16] = &[
    22, 53, 88, 135, 139, 389, 445, 464, 636, 2179, 3268, 3269, 3389, 5040, 5353, 5355, 5357, 5985,
    47001,
];

/// Parse la sortie `netstat -ano | findstr LISTENING` (Windows) ou `ss -tln` /
/// `netstat -tln` (Linux) en liste de ports en écoute, ports système écartés.
pub fn parse_listening_ports(text: &str) -> Vec<u16> {
    let mut ports = std::collections::BTreeSet::new();
    for line in text.lines() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        // Windows : "TCP    0.0.0.0:8080    0.0.0.0:0    LISTENING    1234"
        if toks.len() >= 4 && toks.contains(&"LISTENING") {
            if let Some(p) = toks[1]
                .rsplit(':')
                .next()
                .and_then(|x| x.parse::<u16>().ok())
            {
                ports.insert(p);
            }
            continue;
        }
        // Linux (ss / netstat -tln) : "LISTEN 0 128 0.0.0.0:8080 ..."
        //  — séparateur ':' ;
        // BSD/macOS (netstat -an -p tcp) : "tcp4 0 0 127.0.0.1.8080 *.* LISTEN"
        //  — séparateur '.', y compris l'IPv6 imprimée "::1.8080" (zone "%lo0"
        //  possible avant le point).
        if toks.contains(&"LISTEN") {
            for t in &toks {
                if let Some((addr, port)) = t.rsplit_once(':') {
                    if (addr.contains('.') || addr == "*" || addr.starts_with('['))
                        && port.parse::<u16>().map(|p| p > 0).unwrap_or(false)
                    {
                        ports.insert(port.parse::<u16>().unwrap_or(0));
                        break;
                    }
                }
                // forme BSD : le port suit le DERNIER point, l'adresse avant
                // ("127.0.0.1.8080", "*.8080", "::1.8080", "fe80::1%lo0.22")
                if let Some((addr, port)) = t.rsplit_once('.') {
                    if (addr.contains('.') || addr.contains('*') || addr.contains(':'))
                        && port.parse::<u16>().map(|p| p > 0).unwrap_or(false)
                    {
                        ports.insert(port.parse::<u16>().unwrap_or(0));
                        break;
                    }
                }
            }
        }
    }
    ports
        .into_iter()
        .filter(|p| !SSH_PORTS_EXCLUDES.contains(p))
        .collect()
}

/// CREATE_NO_WINDOW pour tout process utilitaire (ssh, powershell…) : aucune
/// console ne doit s'ouvrir depuis une app GUI.
pub fn probe_hidden_cmd(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

/// Indice diagnostic sur une erreur `ssh` (pur, testable).
/// NB : « connect to host … port 22: Permission denied » (blocage socket/pare-feu
/// local) doit être classé comme machine injoignable AVANT le test « permission
/// denied » d'un refus d'authentification par clé publique.
fn ssh_error_hint(stderr: &str) -> &'static str {
    let low = stderr.to_lowercase();
    if low.contains("could not resolve")
        || low.contains("timed out")
        || low.contains("connection refused")
        || low.contains("no route to host")
        || low.contains("network is unreachable")
        || low.contains("connect to host")
    {
        " — machine injoignable (LAN coupé / VPN actif, ou machine éteinte)"
    } else if low.contains("permission denied") || low.contains("publickey") {
        " — clé SSH refusée (ajoute ta clé publique dans authorized_keys de la machine ; \
le mot de passe n'est pas possible en non-interactif)"
    } else {
        ""
    }
}

/// Exécute une commande sur la machine distante via le client ssh système
/// (aucune dépendance : ssh.exe/OpenSSH livré avec Windows). Auth par clé
/// obligatoire : BatchMode interdit toute invite. Retourne stdout, ou l'erreur
/// ssh en clair (avec indication) pour l'UI.
fn ssh_exec(
    host: &str,
    user: &str,
    remote_cmd: &str,
    connect_timeout: u64,
) -> Result<String, String> {
    let host = ssh_safe(host).ok_or_else(|| "hôte invalide".to_string())?;
    let user = ssh_safe(user).ok_or_else(|| "utilisateur SSH invalide".to_string())?;
    let mut cmd = Command::new("ssh");
    cmd.args([
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-o",
        &format!("ConnectTimeout={}", connect_timeout.clamp(2, 15)),
        &format!("{user}@{host}"),
        remote_cmd,
    ]);
    probe_hidden_cmd(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("client ssh introuvable ou injoignable : {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() && stdout.trim().is_empty() {
        let hint = ssh_error_hint(&stderr);
        return Err(format!("ssh {user}@{host} : {}{}", stderr.trim(), hint));
    }
    Ok(stdout)
}

/// Health-check SSH : la machine est-elle joignable et l'authentification par
/// clé fonctionne-t-elle ? Vrai contrôle en une commande (pas de scan) —
/// à appeler AVANT toute découverte pour éviter un scan de 30 s sur une
/// machine injoignable, et pour distinguer "machine éteinte" de "clé absente".
pub fn ssh_health(host: &str, user: &str, timeout_secs: u64) -> Result<(), String> {
    let out = ssh_exec(host, user, "echo __MCC_OK__", timeout_secs)?;
    if out.contains("__MCC_OK__") {
        Ok(())
    } else {
        Err(format!(
            "ssh {user}@{host} : connexion établie mais la commande distante n'a pas répondu"
        ))
    }
}

/// Liste les ports en écoute d'une machine distante (netstat/ss exécutés sur place).
pub fn ssh_netstat(host: &str, user: &str, timeout_secs: u64) -> Result<Vec<u16>, String> {
    let stdout = ssh_exec(
        host,
        user,
        "netstat -ano | findstr LISTENING || ss -tln",
        timeout_secs,
    )?;
    Ok(parse_listening_ports(&stdout))
}

/// Ports TCP en écoute SUR CETTE machine (toutes interfaces), ports système
/// écartés — la source de la découverte locale : l'OS sait déjà ce qui écoute,
/// le scan au boot n'a plus à deviner une plage (un serveur sur 8787 restait
/// invisible avec scan_from/scan_to = 3000-3200).
pub fn local_listening_ports() -> Vec<u16> {
    let out = if cfg!(windows) {
        let mut c = Command::new("netstat");
        hidden(&mut c).args(["-ano"]).output()
    } else if cfg!(target_os = "macos") {
        // BSD netstat : ni `ss` ni `-tln` — `-an -p tcp` liste les sockets TCP
        // et le parseur filtre sur l'état LISTEN (format à séparateur '.').
        let mut c = Command::new("netstat");
        probe_hidden_cmd(&mut c);
        c.args(["-an", "-p", "tcp"]).output()
    } else {
        // ss (iproute2) d'abord, netstat -tln en repli (même format addr:port)
        let mut c = Command::new("ss");
        probe_hidden_cmd(&mut c);
        match c.args(["-tln"]).output() {
            Ok(o) if o.status.success() => Ok(o),
            _ => {
                let mut c2 = Command::new("netstat");
                probe_hidden_cmd(&mut c2);
                c2.args(["-tln"]).output()
            }
        }
    };
    match out {
        Ok(o) => parse_listening_ports(&String::from_utf8_lossy(&o.stdout)),
        Err(_) => Vec::new(),
    }
}

/// Chemin MCP sanitisé (arrive de la config) : commence par /, aucun espace
/// ni quote — il est injecté dans une commande distante.
fn mcp_path_safe(path: &str) -> String {
    let p = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    if !p.is_empty()
        && p.len() <= 128
        && p.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.' | '~'))
    {
        p
    } else {
        "/mcp".to_string()
    }
}

/// Payload JSON-RPC d'un curl interne (aucune single-quote : protégé par '...').
fn mcp_remote_payload(method: &str, id: u64) -> String {
    let params = if method == "tools/list" {
        json!({})
    } else {
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "mcp-control", "version": env!("CARGO_PKG_VERSION")}
        })
    };
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string()
}

/// Commande curl interne d'un appel MCP sur 127.0.0.1 (à exécuter SUR la machine).
/// `hostport` peut contenir une expression shell SÛRE (${p} pour le batch) :
/// l'URL est en quotes doubles, aucun $ autre que celui voulu (path sanitizé).
fn mcp_remote_curl(hostport: &str, path: &str, payload: &str, curl_timeout: u64) -> String {
    format!(
        "curl -s -m {t} -X POST \"http://127.0.0.1:{hostport}{path}\" \
         -H 'Content-Type: application/json' \
         -H 'Accept: application/json, text/event-stream' \
         -H 'MCP-Protocol-Version: 2024-11-05' \
         -d '{payload}'",
        t = curl_timeout.clamp(1, 15),
        hostport = hostport,
        path = mcp_path_safe(path),
        payload = payload
    )
}

pub const MCC_PORT_MARKER: &str = "__MCC_PORT_";

/// Sonde par lot DEPUIS la machine distante : un seul exec ssh boucle sur les
/// ports avec curl interne sur 127.0.0.1 (passe les binds localhost et ufw).
/// Retourne les ports qui répondent à initialize avec un serverInfo + le résultat.
pub fn ssh_probe_mcp_batch(
    host: &str,
    user: &str,
    ports: &[u16],
    path: &str,
    curl_timeout: u64,
) -> Result<Vec<(u16, Value)>, String> {
    if ports.is_empty() {
        return Ok(vec![]);
    }
    let path = mcp_path_safe(path);
    let list = ports
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let payload = mcp_remote_payload("initialize", 1);
    let remote = format!(
        "for p in {list}; do echo \"{MCC_PORT_MARKER}${{p}}__\"; {curl}; echo; done",
        curl = mcp_remote_curl("${p}", &path, &payload, curl_timeout)
    );
    let out = ssh_exec(host, user, &remote, 8)?;
    let mut results = Vec::new();
    for seg in out.split(MCC_PORT_MARKER).skip(1) {
        let Some((pstr, body)) = seg.split_once("__") else {
            continue;
        };
        let Ok(port) = pstr.trim().parse::<u16>() else {
            continue;
        };
        if let Some(v) = extract_json(body.as_bytes()) {
            // VALIDATION STRICTE (même règle que la sonde locale/réseau) :
            // serverInfo.name, protocolVersion ou capabilities. Un port qui
            // répond `{"result":{}}` n'est PAS un serveur MCP.
            if crate::validate::is_mcp(&v) {
                results.push((port, v));
            }
        }
    }
    Ok(results)
}

/// Sonde COMPLÈTE d'un serveur MCP (initialize + tools/list) exécutée DEPUIS la
/// machine distante : un seul exec ssh, les deux réponses séparées par un
/// marqueur. Utilisé pour le poll des serveurs via_ssh — cf. Server.via_ssh.
pub fn ssh_probe_server(
    host: &str,
    user: &str,
    port: u16,
    path: &str,
    curl_timeout: u64,
) -> McpProbe {
    let t0 = Instant::now();
    let path = mcp_path_safe(path);
    let init = mcp_remote_payload("initialize", 1);
    let tools = mcp_remote_payload("tools/list", 2);
    // Certains serveurs (SDK officiels) attribuent un Mcp-Session-Id à
    // l'initialize et REJETENT tools/list sans lui : en-têtes capturés (-D),
    // session sanitisée (uuid) puis rejouée. AUCUNE double-quote dans la
    // commande distante (l'échappement Windows ssh les mutilerait) : l'en-tête
    // est toujours ajouté — vide quand le serveur n'en émet pas (ignoré).
    let tmp = "/tmp/.mcc_session_$$";
    let remote = format!(
        "{init_curl} -D {tmp}; sid=$(grep -i mcp-session-id {tmp} | head -n 1 | tr -d '\\r' | sed 's/^[^:]*: *//' | tr -cd 'A-Za-z0-9-_' | head -c 100); \
         rm -f {tmp}; echo __MCC_SEP__; {tools_curl} -H 'Mcp-Session-Id:'$sid; echo",
        init_curl = mcp_remote_curl(&port.to_string(), &path, &init, curl_timeout),
        tools_curl = mcp_remote_curl(&port.to_string(), &path, &tools, curl_timeout),
        tmp = tmp,
    );
    let out = match ssh_exec(host, user, &remote, 8) {
        Ok(o) => o,
        Err(e) => return McpProbe::fail(e, false),
    };
    let latency = t0.elapsed().as_millis() as u64;
    let (init_body, tools_body) = out.split_once("__MCC_SEP__").unwrap_or((out.as_str(), ""));
    let remote_open = !init_body.trim().is_empty();
    let Some(v) = extract_json(init_body.as_bytes()) else {
        return McpProbe::fail(
            "via ssh : pas de réponse initialize".to_string(),
            remote_open,
        );
    };
    let Some(res) = v.get("result") else {
        return McpProbe::fail("via ssh : initialize rejeté".to_string(), true);
    };
    let si = res.get("serverInfo");
    let name = si
        .and_then(|s| s.get("name"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let version = si
        .and_then(|s| s.get("version"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let protocol = res
        .get("protocolVersion")
        .and_then(|x| x.as_str())
        .map(String::from);
    let lang_hint = detect_lang_http(&RespHead::default(), name.as_deref());
    let fw = crate::frameworks::detect(
        name.as_deref(),
        protocol.as_deref(),
        false,
        false,
        false,
        lang_hint.as_deref(),
    );
    let framework = fw.map(|f| f.id.to_string());
    let lang = lang_hint.or_else(|| fw.map(|f| f.lang.to_string()));
    let mut tools = Vec::new();
    if let Some(tv) = extract_json(tools_body.as_bytes()) {
        if let Some(arr) = tv
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(|t| t.as_array())
        {
            for t in arr {
                tools.push(ToolInfo {
                    name: t
                        .get("name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("?")
                        .to_string(),
                    desc: t
                        .get("description")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .chars()
                        .take(160)
                        .collect(),
                });
            }
        }
    }
    McpProbe {
        mcp_up: true,
        port_open: true,
        pid: None,
        host: Some(host.to_string()),
        name,
        version,
        protocol,
        sse: false,
        sse_endpoint: false,
        ws: false,
        lang,
        framework,
        tools,
        latency_ms: latency,
        error: None,
    }
}

/// POST JSON vers localhost (essaye ::1 puis 127.0.0.1), lit toute la réponse (gère chunked).
/// Renvoie (host_utilisé, corps, transport SSE détecté).
pub fn http_post(
    port: u16,
    path: &str,
    body: &str,
    timeout_ms: u64,
) -> Result<(String, Vec<u8>, RespHead), String> {
    let mut last_err = String::new();
    for host in ["::1", "127.0.0.1"] {
        match http_post_host(host, port, path, body, timeout_ms, &[]) {
            Ok((b, h)) => return Ok((host.to_string(), b, h)),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

/// Cap mémoire d'une réponse de sonde : un serveur LAN rapide ne doit pas pouvoir
/// gonfler la RAM pendant la fenêtre de timeout (audit 2026-10-24 — le parseur
/// d'entrée HTTP était capé, pas le client).
const MAX_PROBE_READ: usize = 2 * 1024 * 1024;
/// Cap des sondes qui ne lisent que le début d'un flux (SSE, /health).
const MAX_PROBE_READ_SMALL: usize = 256 * 1024;

/// POST JSON vers un hôte précis (localhost OU hôte du réseau local).
/// `extra_headers` : en-têtes additionnels (preuve de pair pour /api/state).
pub fn http_post_host(
    host: &str,
    port: u16,
    path: &str,
    body: &str,
    timeout_ms: u64,
    extra_headers: &[(&str, &str)],
) -> Result<(Vec<u8>, RespHead), String> {
    http_request(
        "POST",
        host,
        port,
        path,
        Some(body),
        timeout_ms,
        extra_headers,
    )
}

/// GET HTTP vers un hôte précis — préauthentification des pairs (/api/hello).
pub fn http_get_host(
    host: &str,
    port: u16,
    path: &str,
    timeout_ms: u64,
) -> Result<(Vec<u8>, RespHead), String> {
    http_request("GET", host, port, path, None, timeout_ms, &[])
}

/// POST/PUT HTTP générique (webhook de notification) : pas d'en-tête MCP,
/// pas de negotiated protocol — un simple appel HTTP avec un corps JSON.
/// `extra_headers` : paires (nom, valeur) ajoutées à la requête.
pub fn http_post_raw(
    host: &str,
    port: u16,
    path: &str,
    body: &str,
    extra_headers: &[(&str, &str)],
    method: &str,
    timeout_ms: u64,
) -> Result<(), String> {
    let (bytes, _head) = http_request_trusted(
        method,
        host,
        port,
        path,
        Some(body),
        timeout_ms,
        extra_headers,
    )?;
    // code HTTP : 2e token de la ligne de statut ("HTTP/1.1 200 OK" → 200).
    let status_line = String::from_utf8_lossy(&bytes)
        .lines()
        .next()
        .unwrap_or("")
        .to_string();
    let code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse::<u16>().ok())
        .unwrap_or(0);
    if (200..400).contains(&code) {
        Ok(())
    } else {
        Err(format!("HTTP {code} ({status_line})"))
    }
}
/// Variante de [`http_request`] pour les en-têtes de CONFIANCE (webhook
/// configuré par l'opérateur) : la valeur peut contenir des espaces
/// (« Authorization: Bearer xxx ») — on exige seulement l'absence de
/// CR/LF et une longueur bornée (le request splitting reste impossible).
fn http_request_trusted(
    method: &str,
    host: &str,
    port: u16,
    path: &str,
    body: Option<&str>,
    timeout_ms: u64,
    trusted_headers: &[(&str, &str)],
) -> Result<(Vec<u8>, RespHead), String> {
    let timeout = Duration::from_millis(timeout_ms);
    let addr: SocketAddr = match socket_addr(host, port) {
        Ok(a) => a,
        Err(e) => return Err(format!("addr {}: {}", host, e)),
    };
    let mut last_err = String::new();
    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(mut stream) => {
            stream.set_read_timeout(Some(timeout)).ok();
            stream.set_write_timeout(Some(timeout)).ok();
            let host_hdr = http_host_header(host, port);
            let path = sanitize_http_path(path, "/");
            let mut req = format!("{} {} HTTP/1.1\r\nHost: {}\r\n", method, path, host_hdr);
            for (n, v) in trusted_headers {
                // SÉCURITÉ : un CR/LF Allow-Injection d'en-têtes (request
                // splitting) reste interdit — seuls les espaces et la
                // ponctuation sont tolérés.
                if v.is_empty() || v.len() > 512 || v.contains('\r') || v.contains('\n') {
                    continue;
                }
                if n.contains('\r') || n.contains('\n') || n.contains(':') {
                    continue;
                }
                req.push_str(&format!("{}: {}\r\n", n, v));
            }
            if body.is_some() {
                req.push_str("Content-Type: application/json\r\n");
            }
            match body {
                Some(b) => req.push_str(&format!(
                    "Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    b.len(),
                    b
                )),
                None => req.push_str("Connection: close\r\n\r\n"),
            }
            if let Err(e) = stream.write_all(req.as_bytes()) {
                last_err = format!("écriture: {e}");
            } else {
                let mut data = Vec::new();
                let mut tmp = [0u8; 16384];
                let deadline = Instant::now() + timeout;
                let mut header_len: Option<usize> = None;
                let mut status_ok = false;
                loop {
                    let remain = deadline.saturating_duration_since(Instant::now());
                    if remain.is_zero() {
                        break;
                    }
                    if header_len.is_none() {
                        if let Some(p) = find_sub(&data, b"\r\n\r\n") {
                            header_len = Some(p + 4);
                            let head = String::from_utf8_lossy(&data[..p]).to_lowercase();
                            status_ok =
                                head.starts_with("http/1.1 2") || head.starts_with("http/1.0 2");
                        }
                    }
                    if let Some(hl) = header_len {
                        let head = String::from_utf8_lossy(&data[..hl]).to_lowercase();
                        let body_bytes = &data[hl..];
                        let complete = if head.contains("transfer-encoding: chunked") {
                            find_sub(body_bytes, b"\r\n0\r\n").is_some()
                        } else if let Some(cl) = head.lines().find_map(|l| {
                            l.strip_prefix("content-length:")
                                .map(|v| v.trim().to_string())
                        }) {
                            body_bytes.len() >= cl.parse::<usize>().unwrap_or(usize::MAX)
                        } else {
                            false
                        };
                        if complete {
                            break;
                        }
                    }
                    match stream.read(&mut tmp) {
                        Ok(0) => break,
                        Ok(n) => data.extend_from_slice(&tmp[..n]),
                        Err(e) => {
                            last_err = format!("lecture: {e}");
                            break;
                        }
                    }
                }
                let head_end = find_sub(&data, b"\r\n\r\n").map(|p| p + 4).unwrap_or(0);
                let head = String::from_utf8_lossy(&data[..head_end]).to_string();
                let mut content_type = String::new();
                let mut server = String::new();
                let mut powered_by = String::new();
                let mut session_id = String::new();
                for line in head.lines() {
                    let lower = line.to_lowercase();
                    if let Some(v) = lower.strip_prefix("content-type:") {
                        content_type = v.trim().to_string();
                    } else if let Some(v) = lower.strip_prefix("server:") {
                        server = v.trim().to_string();
                    } else if let Some(v) = lower.strip_prefix("x-powered-by:") {
                        powered_by = v.trim().to_string();
                    } else if let Some(v) = lower.strip_prefix("mcp-session-id:") {
                        session_id = v.trim().to_string();
                    }
                }
                if status_ok {
                    return Ok((
                        data,
                        RespHead {
                            content_type,
                            server,
                            powered_by,
                            session_id,
                        },
                    ));
                }
                // une erreur d'E/S diagnostiquée (« lecture: … ») est plus
                // parlante qu'un « réponse vide » générique : on la garde
                if last_err.is_empty() {
                    last_err = if data.is_empty() {
                        "réponse vide".to_string()
                    } else {
                        format!(
                            "HTTP non 2xx : {}",
                            String::from_utf8_lossy(&data).lines().next().unwrap_or("?")
                        )
                    };
                }
            }
        }
        Err(e) => last_err = format!("{e}"),
    }
    Err(last_err)
}
fn http_request(
    method: &str,
    host: &str,
    port: u16,
    path: &str,
    body: Option<&str>,
    timeout_ms: u64,
    extra_headers: &[(&str, &str)],
) -> Result<(Vec<u8>, RespHead), String> {
    let timeout = Duration::from_millis(timeout_ms);
    let mut last_err = String::new();
    {
        let addr: SocketAddr = match socket_addr(host, port) {
            Ok(a) => a,
            Err(e) => return Err(format!("addr {}: {}", host, e)),
        };
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(mut stream) => {
                stream.set_read_timeout(Some(timeout)).ok();
                stream.set_write_timeout(Some(timeout)).ok();
                // Host exact (le "localhost" historique casse les vhosts sur hôte distant)
                let host_hdr = http_host_header(host, port);
                // path sans CR/LF ni espace : il est placé tel quel dans la ligne de
                // requête — un path configuré ne doit pas pouvoir injecter d'en-têtes
                let path = sanitize_http_path(path, "/mcp");
                let mut req = format!(
                    "{} {} HTTP/1.1\r\nHost: {}\r\nAccept: application/json, text/event-stream\r\n",
                    method, path, host_hdr
                );
                if body.is_some() {
                    req.push_str("Content-Type: application/json\r\n");
                    // Le protocole par défaut est 2024-11-05. Un appelant qui
                    // sonde le protocole stateless fournit SON en-tête
                    // (2026-07-28) : envoyer les deux valeurs dans la même
                    // requête fait répondre 400 aux serveurs stricts.
                    let has_proto = extra_headers
                        .iter()
                        .any(|(n, _)| n.eq_ignore_ascii_case("MCP-Protocol-Version"));
                    if !has_proto {
                        req.push_str("MCP-Protocol-Version: 2024-11-05\r\n");
                    }
                }
                for (n, v) in extra_headers {
                    // SÉCURITÉ (audit 2026-10-25) : la VALEUR d'un en-tête
                    // additionnel arrive de sources distantes (le nonce renvoyé
                    // par /api/hello d'un pair). Écrite telle quelle, une valeur
                    // contenant \r\n permet le request splitting (injecter des
                    // en-têtes, voire une seconde requête). On n'écrit que les
                    // valeurs du jeu sûr (hex/base64/jeton) — le nonce est hex,
                    // la preuve est hex.
                    let ok = !v.is_empty()
                        && v.len() <= 256
                        && v.chars().all(|c| {
                            c.is_ascii_alphanumeric()
                                || matches!(c, '/' | '+' | '=' | '.' | '_' | '-' | '~' | ':')
                        });
                    if ok {
                        req.push_str(&format!("{}: {}\r\n", n, v));
                    }
                }
                match body {
                    Some(b) => req.push_str(&format!(
                        "Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                        b.len(),
                        b
                    )),
                    None => req.push_str("Connection: close\r\n\r\n"),
                }
                if let Err(e) = stream.write_all(req.as_bytes()) {
                    return Err(format!("{} write: {}", host, e));
                }
                let mut data = Vec::new();
                let mut tmp = [0u8; 16384];
                let deadline = Instant::now() + timeout;
                // lit l'en-tête puis s'arrête dès que le corps est complet (Content-Length / 0-chunk)
                let mut header_len: Option<usize> = None;
                let mut status_ok = false;
                loop {
                    let remain = deadline.saturating_duration_since(Instant::now());
                    if remain.is_zero() {
                        break;
                    }
                    if header_len.is_none() {
                        if let Some(p) = find_sub(&data, b"\r\n\r\n") {
                            header_len = Some(p + 4);
                            let head = String::from_utf8_lossy(&data[..p]).to_lowercase();
                            status_ok =
                                head.starts_with("http/1.1 2") || head.starts_with("http/1.0 2");
                        }
                    }
                    if let Some(hl) = header_len {
                        let head = String::from_utf8_lossy(&data[..hl]).to_lowercase();
                        let body_bytes = &data[hl..];
                        let complete = if head.contains("transfer-encoding: chunked") {
                            find_sub(body_bytes, b"\r\n0\r\n").is_some()
                        } else if let Some(cl) = head.lines().find_map(|l| {
                            l.strip_prefix("content-length:")
                                .map(|v| v.trim().to_string())
                        }) {
                            body_bytes.len() >= cl.parse::<usize>().unwrap_or(usize::MAX)
                        } else {
                            false
                        };
                        if complete {
                            break;
                        }
                    }
                    stream.set_read_timeout(Some(remain)).ok();
                    match stream.read(&mut tmp) {
                        Ok(0) => break,
                        Ok(n) => {
                            data.extend_from_slice(&tmp[..n]);
                            if data.len() > MAX_PROBE_READ {
                                break; // cap mémoire, le parse des octets reçus décidera
                            }
                        }
                        Err(e) => {
                            if data.is_empty() {
                                last_err = format!("{} read: {}", host, e);
                            }
                            break;
                        }
                    }
                }
                if data.is_empty() {
                    if last_err.is_empty() {
                        last_err = format!("{}: réponse vide", host);
                    }
                    return Err(last_err);
                }
                if header_len.is_some() && !status_ok {
                    return Err(format!(
                        "{}: réponse HTTP non-2xx (port occupé par un non-MCP ?)",
                        host
                    ));
                }
                // ROBUSTESSE (audit 2026-10-25) : une réponse sans séparateur
                // CRLFCRLF (serveur qui répond 1-3 octets puis ferme) faisait
                // panicer sur `data[sep + 4..]` (sep=0, len<4) → le thread du
                // POLLER mourait en silence (fenêtre desktop : aucune console)
                // et la supervision des serveurs s'arrêtait définitivement.
                // On borne l'extraction du corps au contenu réellement reçu.
                let sep = find_sub(&data, b"\r\n\r\n").unwrap_or(0);
                let raw_head = String::from_utf8_lossy(&data[..sep]);
                let head = raw_head.to_lowercase();
                let header = |name: &str| -> String {
                    head.lines()
                        .find_map(|l| l.strip_prefix(name))
                        .map(|v| v.trim().to_string())
                        .unwrap_or_default()
                };
                // Mcp-Session-Id : nom d'en-tête insensible à la casse, mais la
                // VALEUR du jeton de session doit conserver sa casse exacte.
                let session_id = raw_head
                    .lines()
                    .find_map(|l| {
                        let (k, v) = l.split_once(':')?;
                        if k.trim().eq_ignore_ascii_case("mcp-session-id") {
                            Some(v.trim().to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                let resp_head = RespHead {
                    content_type: header("content-type:"),
                    server: header("server:"),
                    powered_by: header("x-powered-by:"),
                    session_id,
                };
                let body_start = sep.saturating_add(4).min(data.len());
                let mut body_bytes = data[body_start..].to_vec();
                if head.contains("transfer-encoding: chunked") {
                    body_bytes = dechunk(&body_bytes);
                }
                return Ok((body_bytes, resp_head));
            }
            Err(e) => {
                last_err = format!("{}: {}", host, e);
            }
        }
    }
    Err(last_err)
}

fn dechunk(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < data.len() {
        // ROBUSTESSE (audit 2026-10-25) : deux panics sur réponse chunked
        // tronquée ou à taille absurde —
        //  (1) `&data[i..]` quand i > len (slice hors bornes) ;
        //  (2) `start + size` qui déborde (usize overflow sur un size héxa
        //      énorme, ex. ffffffffffffffff).
        // Les deux tuaient le thread du POLLER en silence (fenêtre desktop :
        // aucune console) et arrêtaient la supervision des serveurs.
        let Some(p) = find_sub(&data[i..], b"\r\n") else {
            break;
        };
        let line_end = i + p;
        let size_str = String::from_utf8_lossy(&data[i..line_end]);
        let size = usize::from_str_radix(size_str.trim().split(';').next().unwrap_or("0"), 16)
            .unwrap_or(0);
        if size == 0 {
            break;
        }
        let start = line_end + 2;
        // saturating_add : jamais de débordement, et la borne `end > data.len()`
        // rejette proprement un chunk tronqué.
        let end = start.saturating_add(size);
        if start > data.len() || end > data.len() {
            break;
        }
        out.extend_from_slice(&data[start..end]);
        // après le chunk, on saute le CRLF final ; s'il manque, on s'arrête
        let next = end.saturating_add(2);
        if next > data.len() {
            break;
        }
        i = next;
    }
    out
}

/// Extrait le JSON-RPC d'une réponse (JSON brut ou flux SSE avec lignes "data:").
pub fn extract_json(body: &[u8]) -> Option<Value> {
    let text = String::from_utf8_lossy(body);
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("data:") {
            let rest = rest.trim();
            if let Ok(v) = serde_json::from_str::<Value>(rest) {
                if v.get("result").is_some() || v.get("error").is_some() {
                    return Some(v);
                }
            }
        }
    }
    if let Some(start) = text.find('{') {
        if let Ok(v) = serde_json::from_str::<Value>(text[start..].trim()) {
            return Some(v);
        }
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with('{') {
                if let Ok(v) = serde_json::from_str::<Value>(t) {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Un appel JSON-RPC HTTP complet (POST + parse).
pub fn rpc_call(
    port: u16,
    path: &str,
    method: &str,
    params: Value,
    id: u64,
    timeout_ms: u64,
) -> Result<(Value, String, RespHead), String> {
    let body = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string();
    let (host, bytes, head) = http_post(port, path, &body, timeout_ms)?;
    let v =
        extract_json(&bytes).ok_or_else(|| format!("réponse non JSON ({} octets)", bytes.len()))?;
    if let Some(err) = v.get("error") {
        return Err(format!("rpc error: {}", err));
    }
    // Même validation stricte que le chemin par hôte : un port local qui
    // n'est pas un serveur MCP doit être rejeté (faux positifs du scan local).
    if method == "initialize" {
        match crate::validate::validate_initialize(&v) {
            crate::validate::McpValid::Valid { .. } => {}
            crate::validate::McpValid::NotMcp => {
                return Err("initialize sans serverInfo : ce port ne parle pas MCP".to_string())
            }
            crate::validate::McpValid::NotJsonRpc => {
                return Err("réponse non JSON-RPC : ce port ne parle pas MCP".to_string())
            }
        }
    }
    Ok((v.get("result").cloned().unwrap_or(Value::Null), host, head))
}

/// Probe MCP streamable-HTTP complète : initialize + tools/list.
pub fn probe_http(port: u16, path: &str, timeout_ms: u64) -> McpProbe {
    let quick = Duration::from_millis(timeout_ms.min(1500));
    let open_host = if port_open("::1", port, quick) {
        "::1"
    } else if port_open("127.0.0.1", port, quick) {
        "127.0.0.1"
    } else {
        return McpProbe::fail(
            "port fermé (rien n'écoute sur ::1 ni 127.0.0.1)".to_string(),
            false,
        );
    };
    let client_info = json!({
        "name": "mcp-control",
        "version": env!("CARGO_PKG_VERSION")
    });
    let init_params = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": client_info.clone()
    });
    // Protocole 2026-07-28 (stateless) : certains serveurs (rust-mcp-sdk 2.x)
    // REFUSENT un initialize nu — il leur faut `Mcp-Method` + les clés `_meta`
    // io.modelcontextprotocol/*. On sonde donc en 2 temps : legacy d'abord
    // (majorité des serveurs), stateless en repli.
    let strict_meta = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": client_info.clone(),
        "io.modelcontextprotocol/clientCapabilities": {}
    });
    let init_strict = json!({
        "protocolVersion": "2026-07-28",
        "capabilities": {},
        "clientInfo": client_info,
        "_meta": strict_meta.clone()
    });
    let strict_init_hdrs: [(&str, &str); 2] = [
        ("MCP-Protocol-Version", "2026-07-28"),
        ("Mcp-Method", "initialize"),
    ];
    let other_host = if open_host == "::1" {
        "127.0.0.1"
    } else {
        "::1"
    };
    let t0 = Instant::now();
    let mut attempt: Option<(serde_json::Value, String, RespHead, u64, bool)> = None;
    let mut last_err = String::from("pas de réponse initialize");
    'probe: for (params, hdrs, stateless) in [
        (&init_params, &[][..], false),
        (&init_strict, &strict_init_hdrs[..], true),
    ] {
        for host in [open_host, other_host] {
            match rpc_call_host_headers(
                host,
                port,
                path,
                "initialize",
                params.clone(),
                1,
                timeout_ms,
                hdrs,
            ) {
                Ok((r, h)) => {
                    attempt = Some((
                        r,
                        host.to_string(),
                        h,
                        t0.elapsed().as_millis() as u64,
                        stateless,
                    ));
                    break 'probe;
                }
                Err(e) => last_err = e,
            }
        }
    }
    // 3e forme : serveurs stateless 2026-07-28 qui n'implémentent PAS
    // `initialize` (rust-mcp-sdk 2.x répond « No handler is implemented for
    // custom requests »). La découverte passe par `server/discover`, et le
    // serverInfo vit sous result._meta["io.modelcontextprotocol/serverInfo"].
    let mut discovered: Option<serde_json::Value> = None;
    if attempt.is_none() {
        let discover_hdrs: [(&str, &str); 2] = [
            ("MCP-Protocol-Version", "2026-07-28"),
            ("Mcp-Method", "server/discover"),
        ];
        for host in [open_host, other_host] {
            if let Ok((r, h)) = rpc_call_host_headers(
                host,
                port,
                path,
                "server/discover",
                json!({ "_meta": strict_meta.clone() }),
                1,
                timeout_ms,
                &discover_hdrs,
            ) {
                // VALIDATION STRICTE du repli stateless : `server/discover`
                // doit prouver qu'il est un serveur MCP, sinon un port qui
                // répond {"result":{}} à n'importe quoi passe (faux positif).
                // La preuve est serverInfo sous _meta (spec 2026-07-28).
                let proof = r
                    .get("_meta")
                    .and_then(|m| m.get("io.modelcontextprotocol/serverInfo"))
                    .and_then(|si| si.get("name"))
                    .and_then(|n| n.as_str())
                    .map(|n| !n.trim().is_empty())
                    .unwrap_or(false);
                if !proof {
                    last_err =
                        "server/discover sans serverInfo : ce port ne parle pas MCP".to_string();
                    continue;
                }
                discovered = Some(r);
                attempt = Some((
                    serde_json::Value::Null,
                    host.to_string(),
                    h,
                    t0.elapsed().as_millis() as u64,
                    true,
                ));
                break;
            }
        }
    }
    let (res, host, head, latency, stateless_mode) = match attempt {
        Some(x) => x,
        None => return McpProbe::fail(last_err, true),
    };
    // serverInfo : à la racine (initialize) ou sous _meta (server/discover).
    let si = discovered
        .as_ref()
        .and_then(|d| {
            d.get("_meta")
                .and_then(|m| m.get("io.modelcontextprotocol/serverInfo"))
        })
        .or_else(|| res.get("serverInfo"));
    let name = si
        .and_then(|s| s.get("name"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let version = si
        .and_then(|s| s.get("version"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let protocol = res
        .get("protocolVersion")
        .and_then(|x| x.as_str())
        .map(String::from)
        .or_else(|| {
            discovered
                .as_ref()
                .and_then(|d| d.get("supportedVersions"))
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|x| x.as_str())
                .map(String::from)
        });

    // Classification transport — le Content-Type de l'initialize NE SUFFIT PAS :
    // FastMCP httpStream répond en text/event-stream (mcp-proxy encode les
    // réponses JSON-RPC en événements SSE, enableJsonResponse=false par défaut)
    // tout en restant du streamable HTTP, et co-sert /sse (compat 2024-11-05).
    // sse = encodage SSE des réponses ; sse_endpoint = /sse legacy co-servi.
    // Le transport réel est streamable HTTP dans tous les cas ici (le POST
    // initialize a été répondu) — un vrai serveur SSE-only échouerait avant.
    let sse = head.sse();
    let sse_path = if path == "/sse" { path } else { "/sse" };
    let sse_endpoint = sse_get_host(&host, port, sse_path, timeout_ms.clamp(500, 1200));
    // Transport WS : upgrade WebSocket acceptée (101) → badge « WS » dans l'UI.
    let ws = ws_upgrade_host(&host, port, path, timeout_ms.clamp(300, 800));
    // Langage : 1) en-têtes Server/X-Powered-By ; 2) process qui écoute le port.
    // Il sert aussi à arbitrer FastMCP Python vs TypeScript.
    let lang_hint = detect_lang_http(&head, name.as_deref()).or_else(|| port_owner_lang(port));
    // Fingerprint : la sonde /health ne part que si l'initialize a répondu
    // 2026-07-28 (requête émise : 2024-11-05) — aucune requête de plus pour les
    // serveurs qui négocient normalement. Les signatures fortes (litemcp,
    // FastMCP) passent avant la détection par nom du framework.
    let health = protocol.as_deref() == Some(crate::LITEMCP_PROTOCOL)
        && health_ok_host(&host, port, timeout_ms.clamp(400, 1200));
    let fw = crate::frameworks::detect(
        name.as_deref(),
        protocol.as_deref(),
        health,
        sse,
        sse_endpoint,
        lang_hint.as_deref(),
    );
    let framework = fw.map(|f| f.id.to_string());
    // 3) sinon le framework (litemcp = lib Rust par définition, idem rmcp…)
    let lang = lang_hint.or_else(|| fw.map(|f| f.lang.to_string()));
    let mut session_hdr: Vec<(&str, &str)> = Vec::new();
    if !head.session_id.is_empty() {
        session_hdr.push(("Mcp-Session-Id", head.session_id.as_str()));
    }
    // Un serveur stateless 2026-07-28 attend les en-têtes `Mcp-Method` et le
    // bloc `_meta` sur CHAQUE requête, pas seulement sur initialize.
    if stateless_mode {
        session_hdr.push(("MCP-Protocol-Version", "2026-07-28"));
        session_hdr.push(("Mcp-Method", "tools/list"));
    }
    let tools_params = if stateless_mode {
        json!({ "_meta": strict_meta })
    } else {
        json!({})
    };
    let mut tools = Vec::new();
    if let Ok((tr, _)) = rpc_call_host_headers(
        &host,
        port,
        path,
        "tools/list",
        tools_params,
        2,
        timeout_ms,
        &session_hdr,
    ) {
        if let Some(arr) = tr.get("tools").and_then(|t| t.as_array()) {
            for t in arr {
                tools.push(ToolInfo {
                    name: t
                        .get("name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("?")
                        .to_string(),
                    desc: t
                        .get("description")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .chars()
                        .take(160)
                        .collect(),
                });
            }
        }
    }
    McpProbe {
        mcp_up: true,
        port_open: true,
        pid: None,
        host: Some(host),
        name,
        version,
        protocol,
        sse,
        sse_endpoint,
        ws,
        lang,
        framework,
        tools,
        latency_ms: latency,
        error: None,
    }
}

/// Lit au plus `MAX_PROBE_READ` octets jusqu'à `\n` sans jamais allouer une
/// ligne géante en mémoire AVANT troncature (audit 2026-10-25 — `read_until`
/// nu lisait les 300 Mo en RAM avant de rendre la main).
fn read_bounded_line<R: std::io::BufRead>(
    reader: &mut R,
    buf: &mut Vec<u8>,
) -> std::io::Result<usize> {
    buf.clear();
    let n = reader
        .take((MAX_PROBE_READ as u64) + 1)
        .read_until(b'\n', buf)?;
    if buf.len() > MAX_PROBE_READ {
        buf.truncate(MAX_PROBE_READ);
    }
    Ok(n)
}

/// Probe/handshake MCP stdio (JSON délimité par nouvelles lignes sur stdin/stdout).
/// keep_alive=true → ne tue pas le process, renvoie un StdioHandle.
pub fn probe_stdio(
    program: &str,
    args: &[String],
    workdir: Option<&str>,
    timeout_ms: u64,
    keep_alive: bool,
) -> Result<(McpProbe, Option<StdioHandle>), String> {
    let t0 = Instant::now();
    let program = resolve_windows_cmd(program);
    let mut cmd = Command::new(&program);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    if let Some(wd) = workdir {
        cmd.current_dir(wd);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("spawn {}: {}", program, e))?;
    let pid = child.id();
    let mut stdin = child.stdin.take().ok_or("stdin indisponible")?;
    let stdout = child.stdout.take().ok_or("stdout indisponible")?;

    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stdout);
        // ROBUSTESSE (audit 2026-10-25) : `BufRead::lines()` et `read_until` nu
        // allouent la ligne ENTIÈRE en mémoire, sans plafond — un serveur stdio
        // qui écrit 300 Mo sur une seule ligne (sans \n) ferait exploser la RAM
        // du moteur. `read_bounded_line` borne la lecture via `Read::take`
        // AVANT allocation.
        let mut buf: Vec<u8> = Vec::new();
        loop {
            match read_bounded_line(&mut reader, &mut buf) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let l = String::from_utf8_lossy(&buf).to_string();
                    if tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let init = json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05", "capabilities": {},
            "clientInfo": {"name": "mcp-control", "version": env!("CARGO_PKG_VERSION")}
        }
    });
    if let Err(e) = writeln!(stdin, "{}", init) {
        kill_tree(pid);
        return Err(format!("write init: {}", e));
    }
    let _ = stdin.flush();

    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut got_init = false;
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut protocol: Option<String> = None;
    let mut tools: Vec<ToolInfo> = Vec::new();
    let mut init_err: Option<String> = None;

    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        let wait = deadline - now;
        match rx.recv_timeout(wait) {
            Ok(line) => {
                let t = line.trim();
                if !t.starts_with('{') {
                    continue;
                } // logs/bruits
                let v: Value = match serde_json::from_str(t) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let id = v.get("id").and_then(|x| x.as_u64());
                if id == Some(1) && !got_init {
                    if let Some(err) = v.get("error") {
                        init_err = Some(format!("initialize rejeté: {}", err));
                        break;
                    }
                    let res = v.get("result").cloned().unwrap_or(Value::Null);
                    if let Some(si) = res.get("serverInfo") {
                        name = si.get("name").and_then(|x| x.as_str()).map(String::from);
                        version = si.get("version").and_then(|x| x.as_str()).map(String::from);
                    }
                    protocol = res
                        .get("protocolVersion")
                        .and_then(|x| x.as_str())
                        .map(String::from);
                    got_init = true;
                    let _ = writeln!(
                        stdin,
                        "{}",
                        json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
                    );
                    let _ = writeln!(
                        stdin,
                        "{}",
                        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
                    );
                    let _ = stdin.flush();
                } else if id == Some(2) {
                    if let Some(res) = v.get("result") {
                        if let Some(arr) = res.get("tools").and_then(|x| x.as_array()) {
                            for t in arr {
                                tools.push(ToolInfo {
                                    name: t
                                        .get("name")
                                        .and_then(|x| x.as_str())
                                        .unwrap_or("?")
                                        .to_string(),
                                    desc: t
                                        .get("description")
                                        .and_then(|x| x.as_str())
                                        .unwrap_or("")
                                        .chars()
                                        .take(160)
                                        .collect(),
                                });
                            }
                        }
                    }
                    break;
                }
            }
            Err(_) => break, // timeout
        }
    }

    let latency = t0.elapsed().as_millis() as u64;
    if init_err.is_some() || !got_init {
        kill_tree(pid);
        let _ = child.wait();
        let msg = init_err.unwrap_or_else(|| {
            format!(
                "timeout {}: pas de réponse initialize en {} ms",
                program, timeout_ms
            )
        });
        return Err(msg);
    }
    // Framework d'un serveur stdio : nom annoncé ou paquet lancé (npx/uvx).
    let lang = detect_lang_stdio(&program, args);
    let args_join = args.join(" ").to_ascii_lowercase();
    let fw = crate::frameworks::detect(
        name.as_deref(),
        protocol.as_deref(),
        false,
        false,
        false,
        lang.as_deref(),
    )
    .or_else(|| crate::frameworks::detect_by_name(&args_join));
    let framework = fw.map(|f| f.id.to_string());
    let lang = lang.or_else(|| fw.map(|f| f.lang.to_string()));
    if keep_alive {
        Ok((
            McpProbe {
                mcp_up: true,
                port_open: true,
                pid: Some(pid),
                host: None,
                name,
                version,
                protocol,
                sse: false,
                sse_endpoint: false,
                ws: false,
                lang,
                framework,
                tools,
                latency_ms: latency,
                error: None,
            },
            Some(StdioHandle {
                child,
                stdin: Some(stdin),
            }),
        ))
    } else {
        kill_tree(pid);
        let _ = child.wait();
        Ok((
            McpProbe {
                mcp_up: true,
                port_open: true,
                pid: Some(pid),
                host: None,
                name,
                version,
                protocol,
                sse: false,
                sse_endpoint: false,
                ws: false,
                lang,
                framework,
                tools,
                latency_ms: latency,
                error: None,
            },
            None,
        ))
    }
}

/// Un appel JSON-RPC HTTP vers un hôte précis (POST + parse).
pub fn rpc_call_host(
    host: &str,
    port: u16,
    path: &str,
    method: &str,
    params: Value,
    id: u64,
    timeout_ms: u64,
) -> Result<(Value, RespHead), String> {
    rpc_call_host_headers(host, port, path, method, params, id, timeout_ms, &[])
}

/// Un appel JSON-RPC HTTP vers un hôte précis avec en-têtes additionnels (ex. `Mcp-Session-Id`).
#[allow(clippy::too_many_arguments)]
pub fn rpc_call_host_headers(
    host: &str,
    port: u16,
    path: &str,
    method: &str,
    params: Value,
    id: u64,
    timeout_ms: u64,
    extra_headers: &[(&str, &str)],
) -> Result<(Value, RespHead), String> {
    let body = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string();
    let (bytes, head) = http_post_host(host, port, path, &body, timeout_ms, extra_headers)?;
    let v =
        extract_json(&bytes).ok_or_else(|| format!("réponse non JSON ({} octets)", bytes.len()))?;
    if let Some(err) = v.get("error") {
        return Err(format!("rpc error: {}", err));
    }
    // VALIDATION STRICTE sur `initialize` : c'est LA preuve qu'un port ouvert
    // parle MCP. Un port qui répond autre chose (API REST, page HTML, 200
    // vide) est rejeté ici — sinon le scan réseau invente des serveurs.
    // Les autres méthodes (tools/list, notifications…) ne sont PAS validées :
    // elles n'ont pas serverInfo par nature.
    if method == "initialize" {
        match crate::validate::validate_initialize(&v) {
            crate::validate::McpValid::Valid { .. } => {}
            crate::validate::McpValid::NotMcp => {
                return Err("initialize sans serverInfo : ce port ne parle pas MCP".to_string())
            }
            crate::validate::McpValid::NotJsonRpc => {
                return Err("réponse non JSON-RPC : ce port ne parle pas MCP".to_string())
            }
        }
    }
    Ok((v.get("result").cloned().unwrap_or(Value::Null), head))
}

/// Probe MCP streamable-HTTP sur un hôte précis : initialize + tools/list.
pub fn probe_http_host(host: &str, port: u16, path: &str, timeout_ms: u64) -> McpProbe {
    if !port_open(host, port, Duration::from_millis(timeout_ms.min(1500))) {
        return McpProbe::fail(format!("{}:{} injoignable", host, port), false);
    }
    let t0 = Instant::now();
    let init_params = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "mcp-control", "version": env!("CARGO_PKG_VERSION")}
    });
    let (res, head) =
        match rpc_call_host(host, port, path, "initialize", init_params, 1, timeout_ms) {
            Ok(x) => x,
            Err(e) => return McpProbe::fail(e, true),
        };
    let latency = t0.elapsed().as_millis() as u64;
    let si = res.get("serverInfo");
    let name = si
        .and_then(|s| s.get("name"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let version = si
        .and_then(|s| s.get("version"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let protocol = res
        .get("protocolVersion")
        .and_then(|x| x.as_str())
        .map(String::from);

    // Même classification que probe_http (voir le commentaire là-bas) :
    // POST initialize répondu = streamable HTTP ; sse = encodage SSE des
    // réponses ; sse_endpoint = /sse legacy co-servi (FastMCP).
    let sse = head.sse();
    let sse_path = if path == "/sse" { path } else { "/sse" };
    let sse_endpoint = sse_get_host(host, port, sse_path, timeout_ms.clamp(500, 1200));
    // Transport WS : upgrade WebSocket acceptée (101) → badge « WS » dans l'UI.
    let ws = ws_upgrade_host(host, port, path, timeout_ms.clamp(300, 800));
    let lang_hint = detect_lang_http(&head, name.as_deref());
    let health = protocol.as_deref() == Some(crate::LITEMCP_PROTOCOL)
        && health_ok_host(host, port, timeout_ms.clamp(400, 1200));
    let fw = crate::frameworks::detect(
        name.as_deref(),
        protocol.as_deref(),
        health,
        sse,
        sse_endpoint,
        lang_hint.as_deref(),
    );
    let framework = fw.map(|f| f.id.to_string());
    let lang = lang_hint.or_else(|| fw.map(|f| f.lang.to_string()));
    let mut session_hdr: Vec<(&str, &str)> = Vec::new();
    if !head.session_id.is_empty() {
        session_hdr.push(("Mcp-Session-Id", head.session_id.as_str()));
    }
    let mut tools = Vec::new();
    if let Ok((tr, _)) = rpc_call_host_headers(
        host,
        port,
        path,
        "tools/list",
        json!({}),
        2,
        timeout_ms,
        &session_hdr,
    ) {
        if let Some(arr) = tr.get("tools").and_then(|t| t.as_array()) {
            for t in arr {
                tools.push(ToolInfo {
                    name: t
                        .get("name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("?")
                        .to_string(),
                    desc: t
                        .get("description")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .chars()
                        .take(160)
                        .collect(),
                });
            }
        }
    }
    McpProbe {
        mcp_up: true,
        port_open: true,
        pid: None,
        host: Some(host.to_string()),
        name,
        version,
        protocol,
        sse,
        sse_endpoint,
        ws,
        lang,
        framework,
        tools,
        latency_ms: latency,
        error: None,
    }
}

// ---------- découverte de pairs MCP Control sur le LAN ----------

pub struct PeerProbe {
    pub alive: bool,
    pub servers: Option<Value>,
}

/// Sonde /api/state d'un moteur MCP Control distant (cache de pairs).
/// `access` : code d'accès API partagé (même valeur des deux côtés).
///
/// SÉCURITÉ (audit 2026-10-24) : le code n'est plus envoyé en clair à la cible —
/// il était diffusé à toutes les cibles sondées du LAN. Déroulé en deux phases :
/// 1. GET /api/hello (préauth) → un moteur MCP Control répond avec un nonce à usage unique ;
/// 2. POST /api/state avec X-MCC-Nonce + X-MCC-Proof = HMAC-SHA256(code, nonce)
///    (cf. crypto.rs) — un observateur réseau ne récupère qu'une preuve morte.
///
/// Un pair en bind loopback accepte aussi la sonde sans preuve (pas de garde).
pub fn probe_peer(ip: &str, port: u16, access: Option<&str>) -> PeerProbe {
    let nonce: Option<String> = match http_get_host(ip, port, "/api/hello", 1200) {
        Ok((bytes, _)) => match serde_json::from_slice::<Value>(&bytes) {
            Ok(v) if v.get("app").and_then(|x| x.as_str()) == Some("mcp-control") => {
                v.get("nonce").and_then(|x| x.as_str()).map(String::from)
            }
            _ => {
                return PeerProbe {
                    alive: false,
                    servers: None,
                }
            }
        },
        Err(_) => {
            return PeerProbe {
                alive: false,
                servers: None,
            }
        }
    };
    let extras: Vec<(String, String)> = match (access, nonce) {
        (Some(code), Some(n)) if !code.is_empty() => vec![
            ("X-MCC-Nonce".to_string(), n.clone()),
            (
                "X-MCC-Proof".to_string(),
                crate::crypto::hmac_sha256_hex(
                    code.as_bytes(),
                    format!("mcc-proof-v1:{}", n).as_bytes(),
                ),
            ),
        ],
        _ => Vec::new(),
    };
    let extra_refs: Vec<(&str, &str)> = extras
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let body = http_post_host(ip, port, "/api/state", "", 1200, &extra_refs).map(|(b, _)| b);
    match body {
        Ok(bytes) => {
            // /api/state répond à tout POST : valider que c'est du JSON avec "servers"
            match serde_json::from_slice::<Value>(&bytes) {
                Ok(v) if v.get("servers").is_some() && v.get("stats").is_some() => PeerProbe {
                    alive: true,
                    servers: Some(v),
                },
                _ => PeerProbe {
                    alive: false,
                    servers: None,
                },
            }
        }
        Err(_) => PeerProbe {
            alive: false,
            servers: None,
        },
    }
}

/// IPs voisines depuis le cache ARP local (Windows: arp -a, Linux/mac: arp -a).
/// Filtrées sur le /24 demandé, sans self.
pub fn arp_hosts(prefix: &str, self_ip: &str) -> Vec<String> {
    let mut c = Command::new("arp");
    let out = hidden(&mut c).arg("-a").output();
    let stdout = match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return Vec::new(),
    };
    let mut ips = std::collections::BTreeSet::new();
    for line in stdout.lines() {
        for tok in line.split_whitespace() {
            let tok = tok.trim_matches(|c| c == '(' || c == ')');
            if looks_like_ip(tok) && tok.starts_with(prefix) && tok != self_ip {
                // écarter réseau (x.x.x.0) et broadcast (x.x.x.255)
                if tok
                    .rsplit_once('.')
                    .map(|(_, last)| last == "0" || last == "255")
                    .unwrap_or(true)
                {
                    continue;
                }
                ips.insert(tok.to_string());
            }
        }
    }
    ips.into_iter().collect()
}

pub fn looks_like_ip(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|p| {
        !p.is_empty()
            && p.len() <= 3
            && p.chars().all(|c| c.is_ascii_digit())
            && p.parse::<u8>().is_ok()
    })
}

/// Un moteur MCP Control tourne-t-il déjà sur CE port local (127.0.0.1/::1) ?
/// Retourne l'URL de son UI. Utilisé par run_headless pour se RECONNECTER à un
/// moteur existant au lieu d'en lancer un second — deux moteurs sur la même
/// config se disputeraient les spawns et réécriraient le config.json en boucle.
pub fn control_ui_at(port: u16) -> Option<String> {
    control_ui_at_hosts(port, &["127.0.0.1", "::1"])
}

/// Cherche un moteur MCP Control sur le port donné, en interrogeant les
/// hôtes indiqués (le bind configuré en premier : un moteur en bind LAN
/// n'est PAS joignable sur 127.0.0.1 si le bind est une IP précise).
/// Timeout large : sous forte charge (scan réseau), 800 ms ne suffisaient pas
/// et la détection échouait — la nouvelle instance prenait alors le port
/// suivant et fuyait un moteur de plus à chaque lancement.
pub fn control_ui_at_hosts(port: u16, hosts: &[&str]) -> Option<String> {
    for host in hosts.iter().copied() {
        if let Ok((bytes, _)) = http_get_host(host, port, "/api/hello", 2000) {
            if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
                if v.get("app").and_then(|x| x.as_str()) == Some("mcp-control") {
                    let h = if host.contains(':') {
                        format!("[{}]:{}", host, port)
                    } else {
                        format!("{}:{}", host, port)
                    };
                    return Some(format!("http://{}", h));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    /// RÉGRESSION faux positifs : un port HTTP qui répond du JSON-RPC SANS
    /// serverInfo (`{"result":{}}`) ne doit JAMAIS être découvert comme
    /// serveur MCP — ni par `initialize`, ni par le repli `server/discover`
    /// (stateless 2026-07-28). Avant la validation stricte, ce port était
    /// accepté et le scan réseau inventait un serveur « port-XXXX ».
    #[test]
    fn port_non_mcp_avec_result_vide_est_rejete() {
        // RÉGRESSION faux positifs : un port HTTP qui répond du JSON-RPC SANS
        // serverInfo ne doit JAMAIS être découvert comme serveur MCP — ni par
        // initialize, ni par le repli server/discover (stateless 2026-07-28).
        // Avant la validation stricte, ce port était accepté et le scan réseau
        // inventait un serveur « port-XXXX ».
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        thread::spawn(move || {
            for stream in l.incoming() {
                let Ok(mut s) = stream else { continue };
                use std::io::Read;
                let mut buf = [0u8; 16384];
                let n = s.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                // ne répondre qu'aux vraies requêtes JSON-RPC : sinon cette
                // fixture empoisonne les sondes /health, /sse et WS des autres
                // tests parallèles (tests flaky).
                if !req.contains("\"method\"") {
                    continue;
                }
                use std::io::Write;
                let body = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = s.write_all(resp.as_bytes());
                let _ = s.flush();
            }
        });
        std::thread::sleep(std::time::Duration::from_millis(150));
        // aucune des trois routes ne doit produire un serveur MCP
        assert!(
            probe_mcp_any_path(port, 2500).is_none(),
            "un port non-MCP ne doit jamais être découvert (faux positif)"
        );
        // et la sonde directe doit échouer aussi
        let p = probe_http(port, "/mcp", 2500);
        assert!(!p.mcp_up, "probe_http doit refuser un result vide");
    }

    #[test]
    fn find_sub_trouve_et_pas_trouve() {
        assert_eq!(find_sub(b"hello\r\n\r\nworld", b"\r\n\r\n"), Some(5));
        assert_eq!(find_sub(b"abc", b"xyz"), None);
        assert_eq!(find_sub(b"ab", b"abcdef"), None);
        assert_eq!(find_sub(b"", b"a"), None);
    }

    #[test]
    fn dechunk_simple_et_multiple() {
        let data = b"5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n";
        assert_eq!(dechunk(data), b"hello world".to_vec());
    }

    #[test]
    fn dechunk_avec_extension_et_tronque() {
        let data = b"4;foo=bar\r\nabcd\r\n0\r\n\r\n";
        assert_eq!(dechunk(data), b"abcd".to_vec());
        // chunk complet puis chunk tronqué : les données complètes survivent
        let tronque = b"5\r\nhello\r\nff\r\ncourt";
        assert_eq!(dechunk(tronque), b"hello".to_vec());
        // premier chunk déjà tronqué : rien de récupérable
        assert_eq!(dechunk(b"ff\r\ncourt"), Vec::<u8>::new());
    }

    #[test]
    fn dechunk_responses_hostiles_ne_paniquent_pas() {
        // Régression audit 2026-10-25 : deux panics dans dechunk() —
        //  (1) slice `&data[i..]` hors bornes quand i dépasse len ;
        //  (2) `start + size` en overflow usize sur une taille hexa absurde.
        // Un chunked tronqué suffisait à TUER le thread du poller en silence.
        // Aucune de ces entrées ne doit paniquer.
        let hostiles: Vec<&[u8]> = vec![
            b"5\r\nhello",           // chunk tronqué (pas de CRLF final)
            b"5\r\nhello\r\n",       // chunk complet, CRLF final absent
            b"3\r\nab",              // taille > contenu
            b"ffffffffffffffff\r\n", // taille hexa absurde → overflow
            b"ffffffff\r\nx\r\n",    // absurdité suivie d'un chunk
            b"0\r\n\r\n",            // fin normale
            b"\r\n",                 // ligne vide
            b"zz\r\nabc",            // taille non hexa
        ];
        for h in hostiles {
            let out = dechunk(h);
            // on vérifie surtout l'absence de panic (le test échoue si ça panic)
            assert!(
                out.len() <= h.len(),
                "dechunk ne doit jamais produire plus que l'entrée"
            );
        }
        // un chunked valide reste correctement décodé
        assert_eq!(
            dechunk(b"5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n"),
            b"hello world".to_vec()
        );
    }

    #[test]
    fn extract_json_depuis_sse() {
        let body =
            b": keepalive\n\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"ok\":true}}\n\n";
        let v = extract_json(body).expect("SSE doit etre parse");
        assert_eq!(v["result"]["ok"], serde_json::json!(true));
    }

    #[test]
    fn ws_upgrade_detecte_101_et_rejette_le_reste() {
        // 1) serveur qui accepte l'upgrade WebSocket → 101 → true
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        thread::spawn(move || {
            for stream in l.incoming() {
                let Ok(mut s) = stream else { return };
                let mut buf = [0u8; 4096];
                let n = s.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let resp = if req.contains("Upgrade: websocket") {
                    "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n"
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
                };
                let _ = s.write_all(resp.as_bytes());
            }
        });
        assert!(
            ws_upgrade_host("127.0.0.1", port, "/mcp", 1500),
            "un serveur qui répond 101 doit être détecté comme WS"
        );

        // 2) serveur HTTP ordinaire → 200 → false (pas de faux badge WS)
        let l2 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port2 = l2.local_addr().unwrap().port();
        thread::spawn(move || {
            for stream in l2.incoming() {
                let Ok(mut s) = stream else { return };
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                let _ = s.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                );
            }
        });
        assert!(
            !ws_upgrade_host("127.0.0.1", port2, "/mcp", 1500),
            "un serveur HTTP qui ne fait pas d'upgrade ne doit pas être marqué WS"
        );

        // 3) port fermé → false, jamais de panic
        let l3 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port3 = l3.local_addr().unwrap().port();
        drop(l3);
        assert!(!ws_upgrade_host("127.0.0.1", port3, "/mcp", 600));
    }

    #[test]
    fn sse_get_detecte_un_flux_event_stream_et_rejette_l_encodage_sse_du_streamable() {
        // 1. endpoint SSE legacy : 200 + "event:" dans le flux → true
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let mut buf = [0u8; 8192];
                let _ = s.read(&mut buf);
                let resp = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\nevent: endpoint\ndata: /messages?sessionId=abc\n\n";
                let _ = s.write_all(resp.as_bytes());
            }
        });
        assert!(
            sse_get(port, "/sse", 2000),
            "un endpoint SSE legacy doit etre detecte"
        );

        // 2. FastMCP httpStream : Content-Type text/event-stream MAIS corps JSON
        //    (réponses encodées, aucun "event:") → false. C'est la confusion
        //    exacte qui faisait badge tous les serveurs FastMCP en "SSE".
        let l2 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port2 = l2.local_addr().unwrap().port();
        thread::spawn(move || {
            if let Ok((mut s, _)) = l2.accept() {
                let mut buf = [0u8; 8192];
                let _ = s.read(&mut buf);
                let body = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{}\r\n0\r\n\r\n",
                    body.len(),
                    body
                );
                let _ = s.write_all(resp.as_bytes());
            }
        });
        assert!(
            !sse_get(port2, "/sse", 2000),
            "du JSON encodé SSE (streamable httpStream) ne doit PAS passer pour un endpoint SSE"
        );
    }

    #[test]
    fn extract_json_depuis_json_brut_et_texte_avant() {
        let v = extract_json(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[]}}").unwrap();
        assert!(v.get("result").is_some());
        let v2 = extract_json(
            b"HTTP/1.1 200 OK\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":3,\"error\":{\"code\":-1}}",
        )
        .unwrap();
        assert!(v2.get("error").is_some());
    }

    #[test]
    fn extract_json_rejete_du_bruit() {
        assert!(extract_json(b"Use /health").is_none());
        assert!(extract_json(b"").is_none());
        assert!(extract_json(b"{invalide").is_none());
    }

    fn spawn_fake_http(responses: Vec<String>) -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            let mut pending = responses.into_iter();
            for stream in listener.incoming() {
                let mut stream = match stream {
                    Ok(s) => s,
                    Err(_) => return,
                };
                let mut buf = [0u8; 16384];
                let n = match stream.read(&mut buf) {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                // sonde de port (TCP nu, pas de JSON-RPC) : on ferme sans consommer de réponse
                if !req.contains("\"method\"") {
                    continue;
                }
                let payload = match pending.next() {
                    Some(r) => r,
                    None => return,
                };
                let body = payload.into_bytes();
                let head = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(format!("{:x}\r\n", body.len()).as_bytes());
                let _ = stream.write_all(&body);
                let _ = stream.write_all(b"\r\n0\r\n\r\n");
                let _ = stream.flush();
            }
        });
        port
    }

    #[test]
    fn detect_framework_exige_les_deux_signatures() {
        // litemcp : protocol 2026-07-28 non négocié + /health « ok »
        assert_eq!(
            detect_framework(Some("2026-07-28"), true, false, false),
            Some("litemcp"),
            "les deux signatures = litemcp"
        );
        // un seul signal litemcp ne suffit pas (specs à jour ailleurs, /health génériques)
        assert_eq!(
            detect_framework(Some("2026-07-28"), false, false, false),
            None
        );
        // 2026-07-28 sans /health MAIS signature SSE+/sse complète → fastmcp
        assert_eq!(
            detect_framework(Some("2026-07-28"), false, true, true),
            Some("fastmcp-py")
        );
        assert_eq!(
            detect_framework(Some("2024-11-05"), true, false, false),
            None
        );
        assert_eq!(detect_framework(None, true, false, false), None);
        // fastmcp : réponses SSE + endpoint /sse legacy co-servi
        assert_eq!(
            detect_framework(Some("2024-11-05"), false, true, true),
            Some("fastmcp-py"),
            "SSE + /sse legacy = fastmcp"
        );
        // un seul des deux signaux fastmcp ne suffit pas
        assert_eq!(
            detect_framework(Some("2024-11-05"), false, true, false),
            None
        );
        assert_eq!(
            detect_framework(Some("2024-11-05"), false, false, true),
            None
        );
        // précédence : les deux fingerprints à la fois → litemcp (le plus spécifique)
        assert_eq!(
            detect_framework(Some("2026-07-28"), true, true, true),
            Some("litemcp")
        );
    }

    #[test]
    fn probe_mcp_any_path_trouve_racine_et_rejete_non_mcp() {
        // serveur MCP monté à la RACINE (ne répond QUE sur "POST / ")
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        thread::spawn(move || {
            for stream in l.incoming() {
                let Ok(mut s) = stream else { return };
                let mut buf = [0u8; 16384];
                let n = match s.read(&mut buf) {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                if !req.starts_with("POST / ") {
                    let _ = s.write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    continue;
                }
                let body = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2024-11-05\",\"serverInfo\":{\"name\":\"racine\",\"version\":\"1.0\"}}}";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = s.write_all(resp.as_bytes());
            }
        });
        let found = probe_mcp_any_path(port, 4000);
        assert!(
            found.is_some(),
            "un serveur monté à la racine doit être découvert"
        );
        let (path, p) = found.unwrap();
        assert_eq!(path, "/");
        assert_eq!(p.name.as_deref(), Some("racine"));

        // non-MCP (404 sur tous les chemins) → None
        let l2 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port2 = l2.local_addr().unwrap().port();
        thread::spawn(move || {
            for stream in l2.incoming() {
                let Ok(mut s) = stream else { return };
                let mut buf = [0u8; 16384];
                let _ = s.read(&mut buf);
                let _ = s.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        });
        assert!(probe_mcp_any_path(port2, 2000).is_none());
    }

    #[test]
    fn parse_listening_ports_windows_linux_et_exclusions() {
        // Windows netstat
        let win = "\
  TCP    0.0.0.0:8080          0.0.0.0:0              LISTENING   1234\r\n\
  TCP    127.0.0.1:5091        0.0.0.0:0              LISTENING   5678\r\n\
  TCP    0.0.0.0:135           0.0.0.0:0              LISTENING   999\r\n\
  TCP    0.0.0.0:445           0.0.0.0:0              LISTENING   999\r\n\
  UDP    0.0.0.0:5353          *:*                                999\r\n";
        let mut p = parse_listening_ports(win);
        p.sort();
        assert_eq!(
            p,
            vec![5091, 8080],
            "ports MCP gardés, système (135/445/5353) écartés"
        );

        // Linux ss -tln
        let linux = "State  Recv-Q Send-Q Local Address:Port  Peer Address:Port\n\
LISTEN  0      128        0.0.0.0:3099       0.0.0.0:*\n\
LISTEN  0      128           [::]:22            [::]:*\n\
LISTEN  0      128              *:3141             *:*";
        let mut p2 = parse_listening_ports(linux);
        p2.sort();
        assert_eq!(
            p2,
            vec![3099, 3141],
            "port 22 système écarté, ports MCP gardés"
        );
    }

    #[test]
    fn parse_listening_ports_bsd_macos() {
        // macOS/BSD `netstat -an -p tcp` : séparateur '.', IPv6 "::1.port",
        // zone "%lo0" possible avant le point ; ports système (22, 135) écartés
        let bsd = "\
Active Internet connections (including servers)\n\
Proto Recv-Q Send-Q  Local Address          Foreign Address        (state)\n\
tcp4       0      0  127.0.0.1.3166         *.*                    LISTEN\n\
tcp6       0      0  ::1.3167               *.*                    LISTEN\n\
tcp4       0      0  *.3168                 *.*                    LISTEN\n\
tcp6       0      0  fe80::1%lo0.3169       *.*                    LISTEN\n\
tcp4       0      0  127.0.0.1.22           *.*                    LISTEN\n\
tcp4       0      0  127.0.0.1.135          *.*                    LISTEN\n";
        let mut p = parse_listening_ports(bsd);
        p.sort();
        assert_eq!(
            p,
            vec![3166, 3167, 3168, 3169],
            "format BSD : ports MCP gardés, système écartés"
        );
    }

    #[test]
    fn ssh_safe_rejete_les_injections() {
        assert_eq!(ssh_safe("192.168.1.42").as_deref(), Some("192.168.1.42"));
        assert_eq!(ssh_safe("mon-pc.local").as_deref(), Some("mon-pc.local"));
        assert_eq!(ssh_safe("Deamon_1").as_deref(), Some("Deamon_1"));
        assert_eq!(ssh_safe("::1").as_deref(), Some("::1"));
        // injection classique : tout caractère hors liste est rejeté
        assert_eq!(ssh_safe("x; rm -rf /"), None);
        assert_eq!(ssh_safe("a$(reboot)"), None);
        assert_eq!(ssh_safe("a b"), None);
        assert_eq!(ssh_safe(""), None);
        assert_eq!(ssh_safe("a\"b"), None);
    }

    #[test]
    fn health_ok_detecte_la_sonde_litemcp_et_rejete_le_reste() {
        // 1. sonde litemcp : 200 + corps « ok » → true
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                let resp = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
                let _ = s.write_all(resp.as_bytes());
            }
        });
        assert!(health_ok_host("127.0.0.1", port, 2000));

        // 2. /health qui répond autre chose → false
        let l2 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port2 = l2.local_addr().unwrap().port();
        thread::spawn(move || {
            if let Ok((mut s, _)) = l2.accept() {
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                let resp = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"up\"}";
                let _ = s.write_all(resp.as_bytes());
            }
        });
        assert!(
            !health_ok_host("127.0.0.1", port2, 2000),
            "un /health JSON sans « ok » ne doit pas passer"
        );
    }

    #[test]
    fn probe_http_fingerprint_litemcp() {
        // Faux serveur litemcp : initialize répondu en 2026-07-28 (sans négocier),
        // tools/list, et GET /health → 200 « ok ». Le probe doit poser
        // framework = "litemcp" et inférer le langage rust.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            let init = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2026-07-28\",\"serverInfo\":{\"name\":\"hello\",\"version\":\"0.1.0\"}}}";
            let tools = "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"hello\",\"description\":\"Dit bonjour\"}]}}";
            let mut pending: Vec<&str> = vec![init, tools];
            for stream in listener.incoming() {
                let mut stream = match stream {
                    Ok(s) => s,
                    Err(_) => return,
                };
                let mut buf = [0u8; 16384];
                let n = match stream.read(&mut buf) {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                if req.starts_with("GET /health") {
                    let resp = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
                    let _ = stream.write_all(resp.as_bytes());
                    continue;
                }
                let Some(payload) = pending.first().copied() else {
                    return;
                };
                if req.contains("\"method\"") {
                    pending.remove(0);
                }
                let body = payload.as_bytes();
                let head = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: ";
                let _ = stream.write_all(
                    format!("{}{}\r\nConnection: close\r\n\r\n", head, body.len()).as_bytes(),
                );
                let _ = stream.write_all(body);
                let _ = stream.flush();
            }
        });
        let p = probe_http(port, "/mcp", 5000);
        assert!(p.mcp_up, "doit etre UP, erreur: {:?}", p.error);
        assert_eq!(
            p.framework.as_deref(),
            Some("litemcp"),
            "2026-07-28 non négocié + /health ok = litemcp"
        );
        assert_eq!(p.lang.as_deref(), Some("rust"), "litemcp = lib Rust");
        assert_eq!(p.protocol.as_deref(), Some("2026-07-28"));
    }

    #[test]
    fn probe_http_sans_sonde_health_pas_de_fingerprint() {
        // protocol 2026-07-28 mais /health sans réponse (le faux serveur ignore
        // les GET sans "method") : un seul signal → PAS de fingerprint.
        let port = spawn_fake_http(vec![
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2026-07-28\",\"serverInfo\":{\"name\":\"x\",\"version\":\"1\"}}}"
                .to_string(),
            "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[]}}".to_string(),
        ]);
        let p = probe_http(port, "/mcp", 5000);
        assert!(p.mcp_up);
        assert_eq!(
            p.framework, None,
            "sans /health « ok », pas de badge litemcp (un seul signal)"
        );
    }

    #[test]
    fn probe_http_initialise_et_liste_les_tools() {
        let init = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2024-11-05\",\"serverInfo\":{\"name\":\"test-mcp\",\"version\":\"9.9.9\"}}}";
        let tools = "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"tool_a\",\"description\":\"premiere\"},{\"name\":\"tool_b\",\"description\":\"deuxieme\"}]}}";
        let port = spawn_fake_http(vec![init.to_string(), tools.to_string()]);
        let p = probe_http(port, "/mcp", 5000);
        assert!(p.mcp_up, "doit etre UP, erreur: {:?}", p.error);
        assert_eq!(p.name.as_deref(), Some("test-mcp"));
        assert_eq!(p.version.as_deref(), Some("9.9.9"));
        assert_eq!(p.protocol.as_deref(), Some("2024-11-05"));
        assert_eq!(p.tools.len(), 2);
        assert_eq!(p.tools[0].name, "tool_a");
        assert_eq!(
            p.host.as_deref(),
            Some("127.0.0.1"),
            "fallback ::1 -> 127.0.0.1"
        );
        assert!(p.latency_ms < 5000);
    }

    #[test]
    fn probe_http_rejete_un_port_non_mcp() {
        let port = spawn_fake_http(vec!["Use /health".to_string(), "Use /health".to_string()]);
        let p = probe_http(port, "/mcp", 3000);
        assert!(!p.mcp_up, "un serveur non-MCP ne doit pas etre UP");
        assert!(p.error.is_some());
    }

    #[test]
    fn probe_peer_accepte_le_format_api_state() {
        // contrat (deux phases, audit 2026-10-24) : /api/hello donne un nonce,
        // puis /api/state (avec ou sans preuve selon le code disponible) répond
        // "servers" + "stats". Le nonce doit être UNIQUEMENT utilisé, jamais un secret.
        let corps = r#"{"servers":[{"id":"seed-overmind","label":"Overmind","transport":"http","transport_info":{"host":null,"port":3099,"path":"/mcp"},"enabled":true,"state":"UP","server_name":"OverMind-MCP","version":"3.10.5","tools_count":16}],"log":[],"stats":{"up":4,"total":7,"tools":59},"settings":{"refresh_secs":15}}"#;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let corps = corps.to_string();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { return };
                let mut buf = [0u8; 8192];
                let _ = s.read(&mut buf);
                let req = String::from_utf8_lossy(&buf).to_string();
                let resp = if req.starts_with("GET /api/hello") {
                    let body = r#"{"app":"mcp-control","version":"0.7.1","nonce":"aabbccdd00112233aabbccdd00112233aabbccdd00112233aabbccdd00112233"}"#;
                    format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                } else if req.starts_with("POST /api/state") {
                    // aucun code en clair ne doit transiter : seulement nonce + preuve
                    assert!(
                        !req.contains("X-MCC-Access:"),
                        "le code d'accès ne doit JAMAIS être envoyé au pair"
                    );
                    format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", corps.len(), corps)
                } else {
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        .to_string()
                };
                let _ = s.write_all(resp.as_bytes());
            }
        });
        let pr = probe_peer("127.0.0.1", port, None);
        assert!(
            pr.alive,
            "un moteur MCP Control doit être détecté comme pair"
        );
        let servers = pr.servers.expect("servers doit être présent");
        assert_eq!(servers["stats"]["total"], serde_json::json!(7));
        assert_eq!(
            servers["servers"][0]["server_name"],
            serde_json::json!("OverMind-MCP")
        );
    }

    #[test]
    fn probe_peer_rejete_un_serveur_qui_ne_est_pas_un_moteur_control() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let mut buf = [0u8; 8192];
                let _ = s.read(&mut buf);
                let body = b"Use /health";
                let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), String::from_utf8_lossy(body));
                let _ = s.write_all(resp.as_bytes());
            }
        });
        let pr = probe_peer("127.0.0.1", port, None);
        assert!(!pr.alive, "un service étranger ne doit pas être un pair");
        assert!(pr.servers.is_none());
    }

    #[test]
    fn probe_http_port_ferme_est_down() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let p = probe_http(port, "/mcp", 800);
        assert!(!p.mcp_up);
        assert!(!p.port_open);
    }

    #[test]
    fn probe_http_reponse_courte_sans_separateur_ne_panique_pas() {
        // Régression audit 2026-10-25 : une réponse de 1-3 octets sans CRLFCRLF
        // faisait panicer sur `data[sep + 4..]` → le thread du POLLER mourait
        // en silence et la supervision des serveurs s'arrêtait définitivement.
        // Le probe doit renvoyer une erreur, jamais paniquer.
        for reponse in [
            b"X".to_vec(),
            b"OK".to_vec(),
            b"abc".to_vec(),
            b"\r\n".to_vec(),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let value = reponse.clone();
            thread::spawn(move || {
                for _ in 0..3 {
                    let Ok((mut s, _)) = listener.accept() else {
                        return;
                    };
                    let mut buf = [0u8; 8192];
                    let _ = s.read(&mut buf);
                    let _ = s.write_all(&value);
                    let _ = s.flush();
                }
            });
            let p = probe_http(port, "/mcp", 2000);
            assert!(!p.mcp_up, "une réponse non-HTTP ne doit pas être UP");
            assert!(
                p.error.is_some(),
                "le probe doit signaler une erreur, pas paniquer (réponse {:?})",
                String::from_utf8_lossy(&reponse)
            );
        }
    }

    #[test]
    fn pids_listening_on_parse_par_colonne_exacte() {
        // Régression audit : ":3141" matchait aussi ":31410" par sous-chaîne → taskkill du mauvais PID
        let own = std::process::id().to_string();
        let netstat = format!(
            "  Proto  Adresse locale        Adresse distante  État        PID\r\n\
             TCP    0.0.0.0:3141          0.0.0.0:0         LISTENING   1111\r\n\
             TCP    0.0.0.0:31410         0.0.0.0:0         LISTENING   2222\r\n\
             TCP    [::]:3141             [::]:0            LISTENING   3333\r\n\
             TCP    127.0.0.1:5090        0.0.0.0:0         LISTENING   {}\r\n\
             UDP    0.0.0.0:3141          *:*                             4444\r\n",
            own
        );
        let mut pids = pids_listening_on(&netstat, 3141);
        pids.sort();
        assert_eq!(
            pids,
            vec![1111, 3333],
            ":31410 ne doit pas matcher, ni la ligne UDP, ni notre propre PID"
        );
        assert_eq!(pids_listening_on(&netstat, 31410), vec![2222]);
        assert!(
            pids_listening_on(&netstat, 5090).is_empty(),
            "notre propre PID est exclu (pas de suicide si l'id vise notre port UI)"
        );
    }

    #[test]
    fn pids_listening_on_ss_parse_par_colonne_exacte() {
        // Régression audit : kill_port était un no-op sur unix — parse ss -tlnp
        // par port exact, propre PID exclu (même contrat que pids_listening_on).
        let own = std::process::id().to_string();
        let ss = format!(
            "State  Recv-Q Send-Q Local Address:Port Peer Address:Port Process\n\
             LISTEN 0      128          0.0.0.0:3141       0.0.0.0:*    users:((\"node\",pid=1111,fd=20))\n\
             LISTEN 0      128        [::]:3141            [::]:*       users:((\"python3\",pid=2222,fd=19))\n\
             LISTEN 0      128          0.0.0.0:31410      0.0.0.0:*    users:((\"node\",pid=3333,fd=18))\n\
             LISTEN 0      128        127.0.0.1:5090       0.0.0.0:*    users:((\"mcp-control\",pid={},fd=12))\n",
            own
        );
        let mut pids = pids_listening_on_ss(&ss, 3141);
        pids.sort();
        assert_eq!(
            pids,
            vec![1111, 2222],
            ":31410 ne doit pas matcher, notre propre PID exclu"
        );
        assert_eq!(pids_listening_on_ss(&ss, 31410), vec![3333]);
        assert!(
            pids_listening_on_ss(&ss, 5090).is_empty(),
            "notre propre PID est exclu (pas de suicide)"
        );
    }

    #[test]
    fn control_ui_at_detecte_le_moteur_et_rejete_les_etrangers() {
        // un moteur MCP Control (hello avec app=mcp-control) → Some(url)
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        thread::spawn(move || {
            for stream in l.incoming() {
                let Ok(mut s) = stream else { return };
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                let req = String::from_utf8_lossy(&buf).to_string();
                let body = if req.starts_with("GET /api/hello") {
                    r#"{"app":"mcp-control","version":"0.7.2","nonce":"aa"}"#.to_string()
                } else {
                    String::new()
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = s.write_all(resp.as_bytes());
            }
        });
        assert!(
            control_ui_at(port).is_some(),
            "un moteur MCP Control doit être détecté (reconnexion au lieu d'un second)"
        );

        // un occupant étranger (autre logiciel HTTP) → None
        let l2 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port2 = l2.local_addr().unwrap().port();
        thread::spawn(move || {
            for stream in l2.incoming() {
                let Ok(mut s) = stream else { return };
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                let body = "<html>une autre application</html>";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = s.write_all(resp.as_bytes());
            }
        });
        assert!(
            control_ui_at(port2).is_none(),
            "un service étranger ne doit pas être pris pour un moteur"
        );

        // rien n'écoute → None (rapide, sans panic)
        let l3 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port3 = l3.local_addr().unwrap().port();
        drop(l3);
        assert!(control_ui_at(port3).is_none());
    }

    #[test]
    fn control_ui_at_hosts_trouve_un_moteur_bindi_sur_une_ip_precise() {
        // Serveur qui ne répond QUE sur l'adresse de loopback : on vérifie
        // que la liste d'hôtes est bien respectée (le bind configuré d'abord).
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        thread::spawn(move || {
            for stream in l.incoming() {
                let Ok(mut s) = stream else { return };
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                let body = r#"{"app":"mcp-control","version":"0.7.2","nonce":"x"}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = s.write_all(resp.as_bytes());
            }
        });
        // Le bind configuré est prioritaire : il est trouvé.
        assert!(control_ui_at_hosts(port, &["127.0.0.1", "::1"]).is_some());
        // Un bind sur une adresse qui n'écoute pas → pas de faux positif.
        assert!(control_ui_at_hosts(port, &["192.0.2.1"]).is_none());
    }

    #[test]
    fn sanitize_http_path_et_host_header_bloquent_crlf_et_bracketent_ipv6() {
        assert_eq!(http_host_header("::1", 8080), "[::1]:8080");
        assert_eq!(http_host_header("127.0.0.1", 8080), "127.0.0.1:8080");
        assert_eq!(sanitize_http_path("/mcp", "/mcp"), "/mcp");
        assert_eq!(sanitize_http_path("/", "/mcp"), "/");
        assert_eq!(
            sanitize_http_path("/mcp HTTP/1.1\r\nX-Inj: 1", "/mcp"),
            "/mcp"
        );
        assert_eq!(sanitize_http_path("/mcp\r\nX-Inj: 1", "/mcp"), "/mcp");
        assert_eq!(sanitize_http_path("sans-slash", "/mcp"), "/mcp");
        assert_eq!(sanitize_http_path("", "/sse"), "/sse");

        // Vérifie que ws_upgrade_host n'injecte jamais d'en-tête sur un path hostile
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let mut buf = [0u8; 4096];
                let n = s.read(&mut buf).unwrap_or(0);
                let _ = tx.send(String::from_utf8_lossy(&buf[..n]).to_string());
                let _ = s.write_all(b"HTTP/1.1 101 Switching Protocols\r\n\r\n");
            }
        });
        assert!(ws_upgrade_host(
            "127.0.0.1",
            port,
            "/mcp HTTP/1.1\r\nX-Injected: pwn",
            1000
        ));
        let req = rx.recv().unwrap();
        assert!(
            req.starts_with("GET /mcp HTTP/1.1\r\n"),
            "le path hostile doit retomber sur /mcp: {req}"
        );
        assert!(
            !req.contains("X-Injected"),
            "aucun en-tête injecté ne doit apparaître"
        );
        assert!(
            req.contains(&format!("Host: 127.0.0.1:{port}\r\n")),
            "Host doit inclure le port: {req}"
        );
    }

    #[test]
    fn read_bounded_line_plafonne_avant_allocation() {
        // Une ligne de 5 Mo sans '\n' ne doit jamais dépasser MAX_PROBE_READ (2 Mo) dans buf
        let big = vec![b'A'; MAX_PROBE_READ * 2 + 1234];
        let mut cursor = std::io::BufReader::new(std::io::Cursor::new(big));
        let mut buf = Vec::new();
        let n = read_bounded_line(&mut cursor, &mut buf).unwrap();
        assert!(n > 0);
        assert_eq!(buf.len(), MAX_PROBE_READ);
        // Une ligne normale avec '\n' est lue intégralement
        let normal = b"{\"jsonrpc\":\"2.0\",\"id\":1}\nreste";
        let mut cursor2 = std::io::BufReader::new(std::io::Cursor::new(&normal[..]));
        let n2 = read_bounded_line(&mut cursor2, &mut buf).unwrap();
        assert_eq!(n2, 25);
        assert_eq!(&buf[..], b"{\"jsonrpc\":\"2.0\",\"id\":1}\n");
    }

    #[test]
    fn probe_http_et_host_propagent_mcp_session_id() {
        // Serveur MCP stateful : émet Mcp-Session-Id (casse mixte) sur initialize
        // et REJETTE tools/list si Mcp-Session-Id est absent ou altéré.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { return };
                let mut buf = [0u8; 16384];
                let n = match s.read(&mut buf) {
                    Ok(n) if n > 0 => n,
                    _ => continue,
                };
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                if req.contains("\"initialize\"") {
                    let body = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-03-26\",\"serverInfo\":{\"name\":\"stateful-mcp\",\"version\":\"1.0.0\"}}}";
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nMcp-Session-Id: Sess_AbCd-1234\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = s.write_all(resp.as_bytes());
                } else if req.contains("\"tools/list\"") {
                    let has_sid = req
                        .lines()
                        .any(|l| l.trim() == "Mcp-Session-Id: Sess_AbCd-1234");
                    let body = if has_sid {
                        "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"secret_tool\",\"description\":\"ok\"}]}}"
                    } else {
                        "{\"jsonrpc\":\"2.0\",\"id\":2,\"error\":{\"code\":-32000,\"message\":\"missing session\"}}"
                    };
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = s.write_all(resp.as_bytes());
                } else {
                    let _ = s.write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                }
            }
        });
        let p = probe_http(port, "/mcp", 4000);
        assert!(p.mcp_up);
        assert_eq!(
            p.tools.len(),
            1,
            "probe_http doit rejouer Mcp-Session-Id sur tools/list"
        );
        assert_eq!(p.tools[0].name, "secret_tool");

        let p2 = probe_http_host("127.0.0.1", port, "/mcp", 4000);
        assert!(p2.mcp_up);
        assert_eq!(
            p2.tools.len(),
            1,
            "probe_http_host doit rejouer Mcp-Session-Id sur tools/list"
        );
    }

    #[test]
    fn ssh_error_hint_distingue_socket_bloque_et_cle_refusee() {
        assert_eq!(
            ssh_error_hint("ssh: connect to host 192.168.2.159 port 22: Permission denied"),
            " — machine injoignable (LAN coupé / VPN actif, ou machine éteinte)"
        );
        assert_eq!(
            ssh_error_hint("ssh: connect to host 192.168.2.159 port 22: Connection timed out"),
            " — machine injoignable (LAN coupé / VPN actif, ou machine éteinte)"
        );
        assert!(
            ssh_error_hint("user@192.168.2.159: Permission denied (publickey).")
                .contains("clé SSH refusée")
        );
    }
}
