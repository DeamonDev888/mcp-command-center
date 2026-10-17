use crate::types::ToolInfo;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

pub struct McpProbe {
    pub mcp_up: bool,
    #[allow(dead_code)] // réservé : distinguer port fermé vs MCP muet côté UI
    pub port_open: bool,
    pub pid: Option<u32>,
    pub host: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub protocol: Option<String>,
    /// Transport SSE détecté (Content-Type: text/event-stream sur la réponse initialize)
    pub sse: bool,
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

/// Tue le process qui ÉCOUTE réellement sur un port local (méthode du launcher
/// MCP MONITOR v7.3) : `child.kill()` ne tue que le lanceur, pas le serveur
/// détaché qu'il a spawné. netstat -ano → PIDs LISTENING → taskkill /F /PID.
#[cfg(windows)]
pub fn kill_port(port: u16) {
    if let Ok(out) = Command::new("netstat").args(["-ano"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        let needle = format!(":{}", port);
        for line in text.lines() {
            if line.contains("LISTENING") && line.contains(&needle) {
                if let Some(pid) = line.split_whitespace().last() {
                    if !pid.is_empty() && pid.chars().all(|c| c.is_ascii_digit()) {
                        let _ = Command::new("taskkill")
                            .args(["/F", "/PID", pid])
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    }
                }
            }
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

/// POST JSON vers localhost (essaye ::1 puis 127.0.0.1), lit toute la réponse (gère chunked).
/// Renvoie (host_utilisé, corps, transport SSE détecté).
pub fn http_post(
    port: u16,
    path: &str,
    body: &str,
    timeout_ms: u64,
) -> Result<(String, Vec<u8>, bool), String> {
    let mut last_err = String::new();
    for host in ["::1", "127.0.0.1"] {
        match http_post_host(host, port, path, body, timeout_ms) {
            Ok((b, ct)) => return Ok((host.to_string(), b, ct.contains("text/event-stream"))),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

/// POST JSON vers un hôte précis (localhost OU hôte du réseau local).
pub fn http_post_host(
    host: &str,
    port: u16,
    path: &str,
    body: &str,
    timeout_ms: u64,
) -> Result<(Vec<u8>, String), String> {
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
                let req = format!(
                    "POST {} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nMCP-Protocol-Version: 2024-11-05\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    path, body.len(), body
                );
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
                let content_type = head
                    .lines()
                    .find_map(|l| l.strip_prefix("content-type:"))
                    .map(|v| v.trim().to_string())
                    .unwrap_or_default();
                let mut body_bytes = data[sep + 4..].to_vec();
                if head.contains("transfer-encoding: chunked") {
                    body_bytes = dechunk(&body_bytes);
                }
                return Ok((body_bytes, content_type));
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
) -> Result<(Value, String, bool), String> {
    let body = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string();
    let (host, bytes, sse) = http_post(port, path, &body, timeout_ms)?;
    let v =
        extract_json(&bytes).ok_or_else(|| format!("réponse non JSON ({} octets)", bytes.len()))?;
    if let Some(err) = v.get("error") {
        return Err(format!("rpc error: {}", err));
    }
    Ok((v.get("result").cloned().unwrap_or(Value::Null), host, sse))
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
    let (res, host, sse) = match rpc_call(port, path, "initialize", init_params, 1, timeout_ms) {
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
        let _ = child.kill();
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
        let _ = child.kill();
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
        let _ = child.kill();
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
) -> Result<(Value, bool), String> {
    let body = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string();
    let (bytes, content_type) = http_post_host(host, port, path, &body, timeout_ms)?;
    let sse = content_type.contains("text/event-stream");
    let v =
        extract_json(&bytes).ok_or_else(|| format!("réponse non JSON ({} octets)", bytes.len()))?;
    if let Some(err) = v.get("error") {
        return Err(format!("rpc error: {}", err));
    }
    Ok((v.get("result").cloned().unwrap_or(Value::Null), sse))
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
    let (res, sse) = match rpc_call_host(host, port, path, "initialize", init_params, 1, timeout_ms)
    {
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
pub fn probe_peer(ip: &str, port: u16) -> PeerProbe {
    let body = http_post_host(ip, port, "/api/state", "", 1200).map(|(b, _)| b);
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
    let out = Command::new("arp").arg("-a").output();
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

fn looks_like_ip(s: &str) -> bool {
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
        let pr = probe_peer("127.0.0.1", port);
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
        let pr = probe_peer("127.0.0.1", port);
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
}
