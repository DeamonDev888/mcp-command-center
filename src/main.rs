mod manage;
mod probe;
mod types;
mod ui;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crate::types::{lockv, AppState};

fn main() {
    let config_path = manage::resolve_config_path(
        env!("CARGO_MANIFEST_DIR"),
        std::env::var("MCC_CONFIG").ok(),
    );
    let (servers, settings) = manage::load_config(&config_path);
    let state = Arc::new(AppState {
        servers: Mutex::new(servers),
        log: Mutex::new(std::collections::VecDeque::new()),
        settings: Mutex::new(settings.clone()),
        children: Mutex::new(std::collections::HashMap::new()),
        peers: Mutex::new(std::collections::HashMap::new()),
        config_path,
    });
    manage::add_log(&state, "info", &format!("mcp-command-center v{} démarré", env!("CARGO_PKG_VERSION")));

    let mut ui_port = settings.ui_port;
    let ui_bind = settings.ui_bind.clone();
    let listener = loop {
        match TcpListener::bind((ui_bind.as_str(), ui_port)) {
            Ok(l) => break l,
            Err(_) => {
                manage::add_log(&state, "warn", &format!("port {} occupé, essai {}", ui_port, ui_port + 1));
                ui_port += 1;
                if ui_port > settings.ui_port + 9 {
                    eprintln!("[fatal] aucun port UI libre ({}..{})", settings.ui_port, settings.ui_port + 9);
                    std::process::exit(1);
                }
            }
        }
    };
    {
        let mut st = lockv(&state.settings);
        st.ui_port = ui_port;
    }
    let ui_host = if ui_bind == "0.0.0.0" { "<ip-lan>".to_string() } else { ui_bind.clone() };
    println!("[UI] http://{}:{}  (Ctrl+C pour arrêter)", ui_host, ui_port);
    manage::add_log(&state, "info", &format!("UI prête sur http://{}:{} (bind {})", ui_host, ui_port, ui_bind));

    // poller : première passe immédiate, ensuite refresh_secs
    {
        let st = state.clone();
        thread::spawn(move || loop {
            manage::poll_all(&st);
            let secs = lockv(&st.settings).refresh_secs;
            thread::sleep(Duration::from_secs(secs));
        });
    }
    // scan de découverte peu après le boot
    {
        let st = state.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1200));
            manage::scan_ports(&st, None);
        });
    }
    // découverte des pairs command-center + réseau (après le scan local)
    {
        let st = state.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(30));
            manage::discover_peers(&st);
            let enabled = crate::types::lockv(&st.settings).network_enabled;
            if enabled {
                let (from, to) = {
                    let s = crate::types::lockv(&st.settings);
                    (s.scan_from, s.scan_to)
                };
                manage::scan_network(&st, Some((from, to)));
            }
        });
    }

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let st = state.clone();
                thread::spawn(move || handle_conn(s, st));
            }
            Err(e) => eprintln!("[accept] {}", e),
        }
    }
}

fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > hay.len() { return None; }
    hay.windows(needle.len()).position(|w| w == needle)
}

fn handle_conn(mut stream: TcpStream, state: Arc<AppState>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let mut buf = Vec::new();
    let mut tmp = [0u8; 16384];
    let mut head_end = None;
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if let Some(p) = find_sub(&buf, b"\r\n\r\n") { head_end = Some(p); break; }
                if buf.len() > 1_048_576 { break; }
            }
            Err(_) => break,
        }
    }
    let head_end = match head_end { Some(p) => p, None => return };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let req_line = lines.next().unwrap_or("");
    let mut parts = req_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").to_string();
    let mut content_length = 0usize;
    for l in lines {
        let lower = l.to_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < content_length {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => body.extend_from_slice(&tmp[..n]),
            Err(_) => break,
        }
    }
    body.truncate(content_length);

    let (status, ctype, payload) = route(&state, &method, &path, &body);
    let reason = if status == 200 { "OK" } else if status == 404 { "Not Found" } else { "Bad Request" };
    let resp = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        status, reason, ctype, payload.len()
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.write_all(&payload);
    let _ = stream.flush();
}

fn route(state: &Arc<AppState>, method: &str, path: &str, body: &[u8]) -> (u16, &'static str, Vec<u8>) {
    if method == "GET" && (path == "/" || path.starts_with("/?")) {
        return (200, "text/html; charset=utf-8", ui::INDEX_HTML.as_bytes().to_vec());
    }
    if (method == "GET" || method == "POST") && path == "/api/state" {
        let j = manage::state_json(state);
        return (200, "application/json", j.to_string().into_bytes());
    }
    if method == "GET" && path.starts_with("/api/detail?id=") {
        let id = urldecode(&path["/api/detail?id=".len()..]);
        let j = manage::detail_json(state, &id);
        return (200, "application/json", j.to_string().into_bytes());
    }
    if method == "POST" && path == "/api/action" {
        let text = String::from_utf8_lossy(body).to_string();
        let j = manage::handle_action(state, &text);
        return (200, "application/json", j.to_string().into_bytes());
    }
    (404, "application/json", br#"{"ok":false,"error":"not found"}"#.to_vec())
}

fn urldecode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => { out.push(b); i += 3; }
                    Err(_) => { out.push(bytes[i]); i += 1; }
                }
            }
            b'+' => { out.push(b' '); i += 1; }
            b => { out.push(b); i += 1; }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urldecode_classique() {
        assert_eq!(urldecode("seed-overmind"), "seed-overmind");
        assert_eq!(urldecode("a%20b+c"), "a b c");
        assert_eq!(urldecode("caf%C3%A9"), "caf\u{e9}");
        assert_eq!(urldecode("%2Fapi%3Fid%3D1"), "/api?id=1");
    }

    #[test]
    fn urldecode_tolerant_aux_sequences_invalides() {
        assert_eq!(urldecode("100%"), "100%");
        assert_eq!(urldecode("%ZZ"), "%ZZ");
        assert_eq!(urldecode("%A"), "%A");
    }

    #[test]
    fn find_sub_cas_limites() {
        assert_eq!(find_sub(b"\r\n\r\n", b"\r\n\r\n"), Some(0));
        assert_eq!(find_sub(&[13u8, 10u8], b"\r\n\r\n"), None);
    }
}
