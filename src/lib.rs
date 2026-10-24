pub mod logos;
pub mod manage;
pub mod probe;
pub mod store_aitmpl;
pub mod store_github;
pub mod store_glama;
pub mod types;
pub mod ui;

use crate::types::{lockv, AppState};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Corps de requête max (anti-DoS : l'ancien code suivait un Content-Length
/// choisi par le client sans plafond → gonflement mémoire infini).
const MAX_BODY: usize = 2 * 1024 * 1024;
/// Connexions simultanées max (un thread par connexion : au-delà on refuse net).
const MAX_CONNS: usize = 64;

/// Version de spec annoncée par litemcp — premier signal du fingerprint :
/// litemcp répond initialize avec 2026-07-28 même quand le client demande
/// 2024-11-05 (stateless strict, pas de négociation).
pub const LITEMCP_PROTOCOL: &str = "2026-07-28";

static OPEN_CONNS: AtomicUsize = AtomicUsize::new(0);

struct ConnGuard;
impl Drop for ConnGuard {
    fn drop(&mut self) {
        OPEN_CONNS.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Vrai si le bind n'expose que la machine locale.
pub fn is_loopback_bind(s: &str) -> bool {
    matches!(s, "127.0.0.1" | "localhost" | "::1" | "[::1]")
}

/// Code d'accès aléatoire 128 bits (hex) — pas de dépendance crypto : RandomState
/// est semé aléatoirement par process, deux tirages suffisent pour un secret LAN.
fn generate_access() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::with_capacity(32);
    for _ in 0..2 {
        let h = std::collections::hash_map::RandomState::new().build_hasher();
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

/// Extrait un paramètre de query d'un chemin (ex. access= pour /api/detail).
fn query_param(path: &str, name: &str) -> Option<String> {
    let q = path.split_once('?')?.1;
    for pair in q.split('&') {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        if k == name {
            return Some(urldecode(v));
        }
    }
    None
}

/// Contrôle d'accès central (pur, testable) :
/// - OPTIONS → 403 : les préflights CORS meurent ici, aucun cross-site ne passe ;
/// - Host hors liste → 403 (anti DNS-rebinding : un domaine rebindexé n'est pas le nôtre) ;
/// - Origin hors liste → 403 (anti CSRF cross-site) ;
/// - /api/* hors loopback → 401 sans le bon code (header X-MCC-Access ou ?access=) ;
/// - POST /api/action → 415 si Content-Type ≠ application/json (bloque les POST
///   « simples » text/plain des sites web, qui s'exécutent sans préflight).
///
/// Extrait la partie hote (sans port, sans crochets) d'un header Host ou d'une origine.
/// Gere l'IPv6 brackete et les ports numeriques ; minuscules pour comparaison.
pub fn host_only_of(h: &str) -> String {
    let h = h.trim();
    if let Some(rest) = h.strip_prefix('[') {
        if let Some(idx) = rest.find(']') {
            return rest[..idx].to_ascii_lowercase();
        }
    }
    if let Some((host, port)) = h.rsplit_once(':') {
        if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
            let host = host.trim_matches(|c| c == '[' || c == ']');
            return host.to_ascii_lowercase();
        }
    }
    h.trim_matches(|c| c == '[' || c == ']')
        .to_ascii_lowercase()
}

/// Hote normalise extrait d'une origine HTTP complete (schema://hote[:port][/...]).
fn origin_host(o: &str) -> String {
    let r = o.split("://").nth(1).unwrap_or(o);
    let r = r.split('/').next().unwrap_or(r);
    host_only_of(r)
}

pub fn check_access(
    require_access: bool,
    api_access: Option<&str>,
    allowed_hosts: &[String],
    allowed_origins: &[String],
    method: &str,
    path: &str,
    headers: &[(String, String)],
) -> Result<(), (u16, &'static str)> {
    if method == "OPTIONS" {
        return Err((403, "Forbidden"));
    }
    if let Some((_, h)) = headers.iter().find(|(n, _)| n == "host") {
        let host_only = host_only_of(h);
        if !allowed_hosts
            .iter()
            .any(|a| a.to_ascii_lowercase() == host_only)
        {
            return Err((403, "Forbidden"));
        }
    }
    // anti request-smuggling : refuser les en-tetes de cadrage ambigus/dupliques
    let cl_count = headers
        .iter()
        .filter(|(n, _)| n == "content-length")
        .count();
    if cl_count > 1 {
        return Err((400, "Bad Request"));
    }
    if headers.iter().any(|(n, _)| n == "transfer-encoding") {
        return Err((400, "Bad Request"));
    }
    if let Some((_, o)) = headers.iter().find(|(n, _)| n == "origin") {
        if !allowed_origins.iter().any(|a| a == o) {
            return Err((403, "Forbidden"));
        }
    }
    let bare_path = path.split('?').next().unwrap_or("");
    if require_access && bare_path.starts_with("/api") {
        let hdr = headers
            .iter()
            .find(|(n, _)| n == "x-mcc-access")
            .map(|(_, v)| v.clone());
        let q = query_param(path, "access");
        let provided: Option<&str> = hdr.as_deref().or(q.as_deref());
        let api_access = api_access.filter(|s| !s.is_empty());
        match (api_access, provided) {
            (Some(expected), Some(got)) if expected == got => {}
            _ => return Err((401, "Unauthorized")),
        }
    }
    if method == "POST" && bare_path == "/api/action" {
        let ok = headers
            .iter()
            .find(|(n, _)| n == "content-type")
            .map(|(_, v)| v.to_ascii_lowercase().starts_with("application/json"))
            .unwrap_or(false);
        if !ok {
            return Err((415, "Unsupported Media Type"));
        }
    }
    Ok(())
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        400 => "Bad Request",
        200 => "OK",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        500 => "Internal Server Error",
        _ => "Bad Request",
    }
}

pub fn run_headless() -> String {
    let config_path =
        manage::resolve_config_path(env!("CARGO_MANIFEST_DIR"), std::env::var("MCC_CONFIG").ok());
    let (servers, settings) = manage::load_config(&config_path);
    let state = Arc::new(AppState {
        servers: Mutex::new(servers),
        log: Mutex::new(std::collections::VecDeque::new()),
        settings: Mutex::new(settings.clone()),
        children: Mutex::new(std::collections::HashMap::new()),
        peers: Mutex::new(std::collections::HashMap::new()),
        allowed_origins: Mutex::new(Vec::new()),
        allowed_hosts: Mutex::new(Vec::new()),
        spawn_fails: Mutex::new(std::collections::HashMap::new()),
        spawning: Mutex::new(std::collections::HashSet::new()),
        update_latest: Mutex::new(None),
        config_path,
    });
    // cache de démarrage : restaure le dernier état connu AVANT la première
    // sonde — l'UI est peuplée instantanément, les sondes corrigent ensuite
    manage::load_cache(&state);
    manage::add_log(
        &state,
        "info",
        &format!("mcp-command-center v{} démarré", env!("CARGO_PKG_VERSION")),
    );

    let mut ui_port = settings.ui_port;
    let ui_bind = settings.ui_bind.clone();
    let listener = loop {
        match TcpListener::bind((ui_bind.as_str(), ui_port)) {
            Ok(l) => break l,
            Err(_) => {
                manage::add_log(
                    &state,
                    "warn",
                    &format!("port {} occupé, essai {}", ui_port, ui_port + 1),
                );
                ui_port += 1;
                if ui_port > settings.ui_port + 9 {
                    eprintln!(
                        "[fatal] aucun port UI libre ({}..{})",
                        settings.ui_port,
                        settings.ui_port + 9
                    );
                    std::process::exit(1);
                }
            }
        }
    };
    {
        let mut st = lockv(&state.settings);
        st.ui_port = ui_port;
    }
    // SÉCURITÉ (audit v0.6.6) :
    // - bind non-loopback → code d'accès API généré au boot si absent (persisté) ;
    // - origines/hostes autorisés (anti CSRF / DNS-rebinding) figés au boot.
    let local_ips_list = manage::local_ips();
    let loopback = is_loopback_bind(&ui_bind);
    let mut generated = false;
    if !loopback {
        let mut st = lockv(&state.settings);
        if st.api_access.as_deref().map(str::is_empty).unwrap_or(true) {
            st.api_access = Some(generate_access());
            generated = true;
        }
    }
    {
        let extra = lockv(&state.settings).allowed_origins.clone();
        let mut origins: Vec<String> = vec![
            format!("http://127.0.0.1:{}", ui_port),
            format!("http://localhost:{}", ui_port),
            format!("http://[::1]:{}", ui_port),
        ];
        let mut hosts: Vec<String> = vec![
            "127.0.0.1".to_string(),
            "localhost".to_string(),
            "::1".to_string(),
        ];
        for ip in &local_ips_list {
            origins.push(format!("http://{}:{}", ip, ui_port));
            hosts.push(ip.clone());
        }
        for o in &extra {
            origins.push(o.clone());
            hosts.push(origin_host(o));
        }
        *lockv(&state.allowed_origins) = origins;
        *lockv(&state.allowed_hosts) = hosts;
    }
    if generated {
        manage::save_config(&state);
        manage::add_log(
            &state,
            "info",
            "code d'accès API généré au boot (persisté dans config.json: settings.api_access) — exigé sur /api/* pour tout accès hors loopback ; pour les pairs, mettre la même valeur des deux côtés",
        );
    }
    let ui_host = if ui_bind == "0.0.0.0" {
        "<ip-lan>".to_string()
    } else {
        ui_bind.clone()
    };
    let ui_url_host = if ui_bind == "0.0.0.0" {
        "127.0.0.1".to_string()
    } else {
        ui_bind.clone()
    };
    let ui_url = format!("http://{}:{}", ui_url_host, ui_port);
    println!("[UI] {}  (Ctrl+C pour arrêter)", ui_url);
    manage::add_log(
        &state,
        "info",
        &format!(
            "UI prête sur http://{}:{} (bind {})",
            ui_host, ui_port, ui_bind
        ),
    );

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
    // démarrage automatique : aligne le registre/fichier sur la préférence
    // persistée (créé au premier lancement — settings.autostart vaut true)
    manage::sync_autostart_at_boot(&state);
    // vérification de mise à jour (release GitHub) : au boot puis chaque heure
    {
        let st = state.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(8));
            manage::check_update(&st);
            thread::sleep(Duration::from_secs(3600));
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

    // serveur HTTP en tâche de fond : run_headless retourne, l'app hôte décide
    // (main = park forever, desktop = boucle d'événements Tauri)
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    let st = state.clone();
                    thread::spawn(move || handle_conn(s, st));
                }
                Err(e) => eprintln!("[accept] {}", e),
            }
        }
    });
    ui_url
}

/// Bloque le thread courant indéfiniment (mode CLI headless).
pub fn park_forever() {
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}

pub fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > hay.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

pub fn handle_conn(mut stream: TcpStream, state: Arc<AppState>) {
    // garde de charge : au-delà de MAX_CONNS connexions simultanées, on refuse net
    // (un thread par connexion — sinon épuisement trivial depuis le LAN)
    if OPEN_CONNS.fetch_add(1, Ordering::SeqCst) >= MAX_CONNS {
        OPEN_CONNS.fetch_sub(1, Ordering::SeqCst);
        return;
    }
    let _guard = ConnGuard;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let mut buf = Vec::new();
    let mut tmp = [0u8; 16384];
    let mut head_end = None;
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if let Some(p) = find_sub(&buf, b"\r\n\r\n") {
                    head_end = Some(p);
                    break;
                }
                if buf.len() > 1_048_576 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let head_end = match head_end {
        Some(p) => p,
        None => return,
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let req_line = lines.next().unwrap_or("");
    let mut parts = req_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").to_string();
    let mut content_length = 0usize;
    let mut headers: Vec<(String, String)> = Vec::new();
    for l in lines {
        if let Some((name, value)) = l.split_once(':') {
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
    for (name, value) in &headers {
        if name == "content-length" {
            content_length = value.parse().unwrap_or(0);
        }
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < content_length {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                body.extend_from_slice(&tmp[..n]);
                if body.len() > MAX_BODY {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if body.len() > MAX_BODY {
        let j = serde_json::json!({"ok": false, "error": "body trop grand (max 2 Mo)"}).to_string();
        let resp = format!(
            "HTTP/1.1 413 Payload Too Large\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            j.len(),
            j
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }
    body.truncate(content_length.min(MAX_BODY));

    // contrôle d'accès (token/Origin/Host/Content-Type) puis routage
    let (loopback_bind, api_access) = {
        let st = lockv(&state.settings);
        (is_loopback_bind(&st.ui_bind), st.api_access.clone())
    };
    let (allowed_hosts, allowed_origins) = (
        lockv(&state.allowed_hosts).clone(),
        lockv(&state.allowed_origins).clone(),
    );
    let gated = check_access(
        !loopback_bind,
        api_access.as_deref(),
        &allowed_hosts,
        &allowed_origins,
        &method,
        &path,
        &headers,
    );
    let (status, ctype, payload) = match gated {
        Err((code, reason)) => {
            let j = serde_json::json!({"ok": false, "error": reason}).to_string();
            (code, "application/json", j.into_bytes())
        }
        Ok(()) => route(&state, &method, &path, &body),
    };
    // ACAO dynamique : on reflète UNIQUEMENT une origine autorisée (jamais *),
    // sinon aucun site tiers ne peut lire nos réponses (/api/state inclu).
    let mut resp = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n",
        status,
        reason_phrase(status),
        ctype,
        payload.len()
    );
    if let Some((_, o)) = headers.iter().find(|(n, _)| n == "origin") {
        if allowed_origins.iter().any(|a| a == o) {
            resp.push_str(&format!("Access-Control-Allow-Origin: {}\r\n", o));
        }
    }
    resp.push_str("Connection: close\r\n\r\n");
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.write_all(&payload);
    let _ = stream.flush();
}

pub fn route(
    state: &Arc<AppState>,
    method: &str,
    path: &str,
    body: &[u8],
) -> (u16, &'static str, Vec<u8>) {
    if method == "GET" && (path == "/" || path.starts_with("/?")) {
        // le code d'accès est injecté dans la page (même origine) : vide en bind loopback
        let access = lockv(&state.settings)
            .api_access
            .clone()
            .unwrap_or_default();
        let html = ui::INDEX_HTML
            .replace("__MCC_ACCESS__", &access)
            .replace("__MCC_VERSION__", env!("CARGO_PKG_VERSION"));
        return (200, "text/html; charset=utf-8", html.into_bytes());
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
    if method == "GET" && path == "/api/store" {
        let j = manage::store_json();
        return (200, "application/json", j.to_string().into_bytes());
    }
    if method == "GET" && path == "/api/store-logos" {
        return (
            200,
            "application/json",
            logos::LOGOS_JSON.as_bytes().to_vec(),
        );
    }
    if method == "POST" && path == "/api/action" {
        let text = String::from_utf8_lossy(body).to_string();
        let j = manage::handle_action(state, &text);
        return (200, "application/json", j.to_string().into_bytes());
    }
    (
        404,
        "application/json",
        br#"{"ok":false,"error":"not found"}"#.to_vec(),
    )
}

pub fn urldecode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn check_access_options_est_toujours_refuse() {
        let hosts = vec!["127.0.0.1".to_string()];
        let origins = vec!["http://127.0.0.1:5090".to_string()];
        let r = check_access(
            false,
            None,
            &hosts,
            &origins,
            "OPTIONS",
            "/api/action",
            &[("host".into(), "127.0.0.1:5090".into())],
        );
        assert_eq!(r, Err((403, "Forbidden")));
    }

    #[test]
    fn check_access_host_inconnu_refuse_dns_rebinding() {
        let hosts = vec![
            "127.0.0.1".to_string(),
            "192.0.2.10".to_string(),
            "::1".to_string(),
        ];
        let origins = vec![];
        let r = check_access(
            false,
            None,
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(&[("host", "evil.example.com:5090")]),
        );
        assert_eq!(r, Err((403, "Forbidden")));
        // host autorisé, avec ou sans port, IPv6 entre crochets
        for h in ["127.0.0.1:5090", "127.0.0.1", "[::1]:5090"] {
            let r2 = check_access(
                false,
                None,
                &hosts,
                &origins,
                "GET",
                "/api/state",
                &hs(&[("host", h)]),
            );
            assert!(r2.is_ok(), "host {} doit passer", h);
        }
    }

    #[test]
    fn check_access_origin_inconnue_refuse() {
        let hosts = vec!["127.0.0.1".to_string()];
        let origins = vec!["http://127.0.0.1:5090".to_string()];
        let r = check_access(
            false,
            None,
            &hosts,
            &origins,
            "POST",
            "/api/action",
            &hs(&[
                ("host", "127.0.0.1:5090"),
                ("origin", "https://malveille.example"),
            ]),
        );
        assert_eq!(r, Err((403, "Forbidden")));
        let ok = check_access(
            false,
            None,
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(&[
                ("host", "127.0.0.1:5090"),
                ("origin", "http://127.0.0.1:5090"),
            ]),
        );
        assert!(ok.is_ok());
    }

    #[test]
    fn check_access_api_exige_le_code_hors_loopback() {
        let hosts = vec!["127.0.0.1".to_string()];
        let origins = vec!["http://127.0.0.1:5090".to_string()];
        let base = &[("host", "127.0.0.1:5090")] as &[(&str, &str)];
        // sans code → 401
        let r = check_access(
            true,
            Some("abc"),
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(base),
        );
        assert_eq!(r, Err((401, "Unauthorized")));
        // mauvais code → 401
        let r = check_access(
            true,
            Some("abc"),
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(&[("host", "127.0.0.1:5090"), ("x-mcc-access", "zzz")]),
        );
        assert_eq!(r, Err((401, "Unauthorized")));
        // bon code (header) → OK ; bon code (?access=) → OK
        assert!(check_access(
            true,
            Some("abc"),
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(&[("host", "127.0.0.1:5090"), ("x-mcc-access", "abc")])
        )
        .is_ok());
        assert!(check_access(
            true,
            Some("abc"),
            &hosts,
            &origins,
            "GET",
            "/api/detail?id=x&access=abc",
            &hs(base)
        )
        .is_ok());
        // loopback (require_access=false) : aucun code requis
        assert!(check_access(
            false,
            Some("abc"),
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(base)
        )
        .is_ok());
    }

    #[test]
    fn check_access_action_post_exige_content_type_json() {
        let hosts = vec!["127.0.0.1".to_string()];
        let origins = vec![];
        // text/plain = requête « simple » sans préflight depuis un site web → 415
        let r = check_access(
            false,
            None,
            &hosts,
            &origins,
            "POST",
            "/api/action",
            &hs(&[("host", "127.0.0.1:5090"), ("content-type", "text/plain")]),
        );
        assert_eq!(r, Err((415, "Unsupported Media Type")));
        // application/json (+paramètres) → OK ; absent → 415
        assert!(check_access(
            false,
            None,
            &hosts,
            &origins,
            "POST",
            "/api/action",
            &hs(&[
                ("host", "127.0.0.1:5090"),
                ("content-type", "application/json; charset=utf-8")
            ])
        )
        .is_ok());
        assert_eq!(
            check_access(
                false,
                None,
                &hosts,
                &origins,
                "POST",
                "/api/action",
                &hs(&[("host", "127.0.0.1:5090")])
            ),
            Err((415, "Unsupported Media Type"))
        );
        // GET /api/state sans content-type : pas concerné
        assert!(check_access(
            false,
            None,
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(&[("host", "127.0.0.1:5090")])
        )
        .is_ok());
    }

    #[test]
    fn is_loopback_bind_cas_connus() {
        for s in ["127.0.0.1", "localhost", "::1", "[::1]"] {
            assert!(is_loopback_bind(s), "{} = loopback", s);
        }
        for s in ["0.0.0.0", "", "192.0.2.10"] {
            assert!(!is_loopback_bind(s), "{} = PAS loopback", s);
        }
    }

    #[test]
    fn query_param_extrait_et_decode() {
        assert_eq!(
            query_param("/api/detail?id=a%20b&access=c+d", "access"),
            Some("c d".to_string())
        );
        assert_eq!(query_param("/api/detail?id=x", "access"), None);
        assert_eq!(query_param("/api/state", "access"), None);
    }

    #[test]
    fn generate_access_fait_32_hex_et_est_unique() {
        let a = generate_access();
        let b = generate_access();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b, "deux tirages RandomState ne doivent pas coller");
    }

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
