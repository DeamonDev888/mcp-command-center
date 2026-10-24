use crate::types::ToolInfo;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

/// En-têtes utiles d'une réponse HTTP (inférence du langage serveur).
#[derive(Default, Clone)]
pub struct RespHead {
    pub content_type: String,
    pub server: String,
    pub powered_by: String,
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

#[cfg(not(windows))]
pub fn kill_port(_port: u16) {}

/// Spawn un process serveur (HTTP/HTTPS lancé par le command center) sans handshake
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
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: localhost\r\nAccept: text/event-stream\r\nConnection: close\r\n\r\n",
        path
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
    let host_hdr = if host.contains(':') {
        format!("[{}]:{}", host, port)
    } else {
        format!("{}:{}", host, port)
    };
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
            Ok(n) => data.extend_from_slice(&tmp[..n]),
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
fn detect_framework(protocol: Option<&str>, health: bool) -> Option<&'static str> {
    if protocol == Some(crate::LITEMCP_PROTOCOL) && health {
        Some("litemcp")
    } else {
        None
    }
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
        // Linux : "LISTEN 0 128 0.0.0.0:8080 ..." ou "tcp LISTEN 0 128 *:8080 ..."
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
        let low = stderr.to_lowercase();
        let hint = if low.contains("permission denied") || low.contains("publickey") {
            " — auth refusée : ajoute ta clé publique sur la machine (authorized_keys), le mot de passe n'est pas possible en non-interactif"
        } else if low.contains("could not resolve")
            || low.contains("timed out")
            || low.contains("connection refused")
        {
            " — machine injoignable"
        } else {
            ""
        };
        return Err(format!("ssh {user}@{host} : {}{}", stderr.trim(), hint));
    }
    Ok(stdout)
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
            "clientInfo": {"name": "mcp-command-center", "version": env!("CARGO_PKG_VERSION")}
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
            if v.get("result").and_then(|r| r.get("serverInfo")).is_some() {
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
        Err(e) => return McpProbe::fail(e, true),
    };
    let latency = t0.elapsed().as_millis() as u64;
    let (init_body, tools_body) = out.split_once("__MCC_SEP__").unwrap_or((out.as_str(), ""));
    let Some(v) = extract_json(init_body.as_bytes()) else {
        return McpProbe::fail("via ssh : pas de réponse initialize".to_string(), true);
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
        lang: None,
        framework: None,
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

/// POST JSON vers un hôte précis (localhost OU hôte du réseau local).
/// `extra_headers` : en-têtes additionnels (ex. code d'accès pour la sonde de pairs).
pub fn http_post_host(
    host: &str,
    port: u16,
    path: &str,
    body: &str,
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
                let host_hdr = if host.contains(':') {
                    format!("[{}]:{}", host, port)
                } else {
                    format!("{}:{}", host, port)
                };
                let mut req = format!(
                    "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nMCP-Protocol-Version: 2024-11-05\r\n",
                    path, host_hdr
                );
                for (n, v) in extra_headers {
                    req.push_str(&format!("{}: {}\r\n", n, v));
                }
                req.push_str(&format!(
                    "Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                ));
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
                        Ok(n) => data.extend_from_slice(&tmp[..n]),
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
                let sep = find_sub(&data, b"\r\n\r\n").unwrap_or(0);
                let head = String::from_utf8_lossy(&data[..sep]).to_lowercase();
                let header = |name: &str| -> String {
                    head.lines()
                        .find_map(|l| l.strip_prefix(name))
                        .map(|v| v.trim().to_string())
                        .unwrap_or_default()
                };
                let resp_head = RespHead {
                    content_type: header("content-type:"),
                    server: header("server:"),
                    powered_by: header("x-powered-by:"),
                };
                let mut body_bytes = data[sep + 4..].to_vec();
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
    while let Some(p) = find_sub(&data[i..], b"\r\n") {
        let line_end = i + p;
        let size_str = String::from_utf8_lossy(&data[i..line_end]);
        let size = usize::from_str_radix(size_str.trim().split(';').next().unwrap_or("0"), 16)
            .unwrap_or(0);
        if size == 0 {
            break;
        }
        let start = line_end + 2;
        let end = start + size;
        if end > data.len() {
            break;
        }
        out.extend_from_slice(&data[start..end]);
        i = end + 2;
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
    Ok((v.get("result").cloned().unwrap_or(Value::Null), host, head))
}

/// Probe MCP streamable-HTTP complète : initialize + tools/list.
pub fn probe_http(port: u16, path: &str, timeout_ms: u64) -> McpProbe {
    let t0 = Instant::now();
    let quick = Duration::from_millis(timeout_ms.min(1500));
    let open = port_open("::1", port, quick) || port_open("127.0.0.1", port, quick);
    if !open {
        return McpProbe::fail(
            "port fermé (rien n'écoute sur ::1 ni 127.0.0.1)".to_string(),
            false,
        );
    }
    let init_params = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "mcp-command-center", "version": env!("CARGO_PKG_VERSION")}
    });
    let (res, host, head) = match rpc_call(port, path, "initialize", init_params, 1, timeout_ms) {
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
    // Fingerprint litemcp : la sonde /health ne part que si l'initialize a
    // répondu 2026-07-28 (requête émise : 2024-11-05) — aucune requête de plus
    // pour les serveurs qui négocient normalement.
    let framework = if protocol.as_deref() == Some(crate::LITEMCP_PROTOCOL) {
        detect_framework(
            protocol.as_deref(),
            health_ok_host(&host, port, timeout_ms.clamp(400, 1200)),
        )
        .map(String::from)
    } else {
        None
    };
    // 1) en-têtes Server/X-Powered-By ; 2) sinon le vrai process qui écoute le port ;
    // 3) sinon le framework (litemcp = lib Rust par définition)
    let lang = detect_lang_http(&head, name.as_deref())
        .or_else(|| port_owner_lang(port))
        .or_else(|| framework.as_ref().map(|_| "rust".to_string()));
    let mut tools = Vec::new();
    if let Ok((tr, _, _)) = rpc_call(port, path, "tools/list", json!({}), 2, timeout_ms) {
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
        lang,
        framework,
        tools,
        latency_ms: latency,
        error: None,
    }
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
        let reader = std::io::BufReader::new(stdout);
        use std::io::BufRead;
        for line in reader.lines() {
            match line {
                Ok(l) => {
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
            "clientInfo": {"name": "mcp-command-center", "version": env!("CARGO_PKG_VERSION")}
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
                lang: detect_lang_stdio(&program, args),
                framework: None,
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
                lang: detect_lang_stdio(&program, args),
                framework: None,
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
    let body = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string();
    let (bytes, head) = http_post_host(host, port, path, &body, timeout_ms, &[])?;
    let v =
        extract_json(&bytes).ok_or_else(|| format!("réponse non JSON ({} octets)", bytes.len()))?;
    if let Some(err) = v.get("error") {
        return Err(format!("rpc error: {}", err));
    }
    Ok((v.get("result").cloned().unwrap_or(Value::Null), head))
}

/// Probe MCP streamable-HTTP sur un hôte précis : initialize + tools/list.
pub fn probe_http_host(host: &str, port: u16, path: &str, timeout_ms: u64) -> McpProbe {
    let t0 = Instant::now();
    if !port_open(host, port, Duration::from_millis(timeout_ms.min(1500))) {
        return McpProbe::fail(format!("{}:{} injoignable", host, port), false);
    }
    let init_params = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "mcp-command-center", "version": env!("CARGO_PKG_VERSION")}
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
    let framework = if protocol.as_deref() == Some(crate::LITEMCP_PROTOCOL) {
        detect_framework(
            protocol.as_deref(),
            health_ok_host(host, port, timeout_ms.clamp(400, 1200)),
        )
        .map(String::from)
    } else {
        None
    };
    let lang = detect_lang_http(&head, name.as_deref())
        .or_else(|| framework.as_ref().map(|_| "rust".to_string()));
    let mut tools = Vec::new();
    if let Ok((tr, _)) = rpc_call_host(host, port, path, "tools/list", json!({}), 2, timeout_ms) {
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
        lang,
        framework,
        tools,
        latency_ms: latency,
        error: None,
    }
}

// ---------- découverte de pairs command-center sur le LAN ----------

pub struct PeerProbe {
    pub alive: bool,
    pub servers: Option<Value>,
}

/// Sonde /api/state d'un command-center distant (cache de pairs).
/// `access` : code d'accès API du pair (secret partagé — la même valeur côté
/// émetteur et récepteur dans config.json). Sans lui, un pair en bind non-loopback refusela sonde.
pub fn probe_peer(ip: &str, port: u16, access: Option<&str>) -> PeerProbe {
    let extras: Vec<(&str, &str)> = access
        .map(|a| vec![("X-MCC-Access", a)])
        .unwrap_or_default();
    let body = http_post_host(ip, port, "/api/state", "", 1200, &extras).map(|(b, _)| b);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

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
    fn extract_json_depuis_sse() {
        let body =
            b": keepalive\n\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"ok\":true}}\n\n";
        let v = extract_json(body).expect("SSE doit etre parse");
        assert_eq!(v["result"]["ok"], serde_json::json!(true));
    }

    #[test]
    fn sse_get_detecte_un_flux_event_stream_et_rejete_l_encodage_sse_du_streamable() {
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
        // les deux signaux : protocol 2026-07-28 non négocié + /health « ok »
        assert_eq!(
            detect_framework(Some("2026-07-28"), true),
            Some("litemcp"),
            "les deux signatures = litemcp"
        );
        // un seul signal ne suffit pas (specs à jour ailleurs, /health génériques)
        assert_eq!(detect_framework(Some("2026-07-28"), false), None);
        assert_eq!(detect_framework(Some("2024-11-05"), true), None);
        assert_eq!(detect_framework(None, true), None);
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
        // contrat : un command-center distant répond /api/state avec "servers" + "stats"
        let corps = r#"{"servers":[{"id":"seed-overmind","label":"Overmind","transport":"http","transport_info":{"host":null,"port":3099,"path":"/mcp"},"enabled":true,"state":"UP","server_name":"OverMind-MCP","version":"3.10.5","tools_count":16}],"log":[],"stats":{"up":4,"total":7,"tools":59},"settings":{"refresh_secs":15}}"#;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let corps = corps.to_string();
        thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let mut buf = [0u8; 8192];
                let _ = s.read(&mut buf);
                let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", corps.len(), corps);
                let _ = s.write_all(resp.as_bytes());
            }
        });
        let pr = probe_peer("127.0.0.1", port, None);
        assert!(pr.alive, "un command-center doit être détecté comme pair");
        let servers = pr.servers.expect("servers doit être présent");
        assert_eq!(servers["stats"]["total"], serde_json::json!(7));
        assert_eq!(
            servers["servers"][0]["server_name"],
            serde_json::json!("OverMind-MCP")
        );
    }

    #[test]
    fn probe_peer_rejete_un_serveur_qui_ne_est_pas_un_command_center() {
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
        assert!(!pr.alive, "un non-command-center ne doit pas être un pair");
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
}
