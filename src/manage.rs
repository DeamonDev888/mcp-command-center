use serde_json::{json, Map, Value};
use std::process::Command;
use std::sync::Arc;
use crate::types::{lockv, LogEntry, Origin, SState, Server, StartCmd, Transport, AppState};
use crate::probe;

const MAX_LOG: usize = 400;

pub fn add_log(state: &AppState, level: &str, msg: &str) {
    let mut log = lockv(&state.log);
    log.push_back(LogEntry { ts: crate::types::now_ts(), level: level.to_string(), msg: msg.to_string() });
    while log.len() > MAX_LOG { log.pop_front(); }
    println!("[{}] {}", level, msg);
}

// ---------- config ----------

fn default_servers() -> Vec<Server> {
    let mk = |id: &str, label: &str, port: u16, enabled: bool| Server {
        id: id.to_string(), label: label.to_string(),
        transport: Transport::Http { host: None, port, path: "/mcp".to_string() },
        origin: Origin::Seed, enabled, auto_restart: true,
        state: if enabled { SState::Down } else { SState::Disabled },
        pid: None, bound_host: None, server_name: None,
        version: None, protocol: None, tools: vec![], latency_ms: None,
        last_seen_ts: None, last_error: None, start_cmd: None,
        tools_disabled: vec![],
    };
    vec![
        mk("seed-overmind", "Overmind", 3099, true),
        mk("seed-discord", "serveur_discord", 3141, true),
        mk("seed-postgresql", "serveur_PostGreSQL", 5433, true),
        mk("seed-debats", "debats", 3100, false),
        mk("seed-x", "X", 3142, true),
        mk("seed-news-btc", "NEWS_BTC", 3143, false),
        mk("seed-news", "NEWS", 3144, false),
    ]
}

pub fn load_config(path: &str) -> (Vec<Server>, crate::types::Settings) {
    let defaults = default_servers();
    let settings = crate::types::Settings::default();
    let file = match std::fs::read_to_string(path) { Ok(f) => f, Err(_) => return (defaults, settings) };
    let v: Value = match serde_json::from_str(&file) { Ok(v) => v, Err(_) => return (defaults, settings) };
    let mut servers = defaults;
    if let Some(arr) = v.get("servers").and_then(|x| x.as_array()) {
        for s in arr {
            let id = s.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
            if id.is_empty() { continue; }
            if let Some(existing) = servers.iter_mut().find(|e| e.id == id) {
                apply_json(existing, s);
            } else if let Some(mut srv) = from_json(s) {
                srv.origin = Origin::Manual;
                servers.push(srv);
            }
        }
    }
    // reconstruit start_cmd pour tout serveur stdio (config rechargée -> auto-start au boot)
    for s in servers.iter_mut() {
        if s.start_cmd.is_none() {
            if let Transport::Stdio { program, args, workdir } = s.transport.clone() {
                if !program.is_empty() {
                    s.start_cmd = Some(StartCmd { program, args, workdir });
                }
            }
        }
    }
    let settings = parse_settings(&v, settings);
    (servers, settings)
}

#[allow(unused_must_use)]
fn apply_json(srv: &mut Server, s: &Value) {
    if let Some(b) = s.get("enabled").and_then(|x| x.as_bool()) {
        srv.enabled = b;
        srv.state = if b { SState::Down } else { SState::Disabled };
    }
    if let Some(t) = s.get("transport").and_then(|x| x.as_str()) {
        if t == "http" {
            if let (Some(p), Some(path)) = (
                s.get("port").and_then(|x| x.as_u64()),
                s.get("path").and_then(|x| x.as_str()),
            ) {
                srv.transport = Transport::Http {
                    host: s.get("host").and_then(|x| x.as_str()).map(String::from),
                    port: p as u16,
                    path: path.to_string(),
                };
            }
        } else if t == "stdio" {
            let args: Vec<String> = s.get("args").and_then(|x| x.as_array()).map(|a| {
                a.iter().filter_map(|i| i.as_str().map(String::from)).collect()
            }).unwrap_or_default();
            srv.transport = Transport::Stdio {
                program: s.get("program").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                args,
                workdir: s.get("workdir").and_then(|x| x.as_str()).map(String::from),
            };
        }
    }
    if let Some(l) = s.get("label").and_then(|x| x.as_str()) { srv.label = l.to_string(); }
    if let Some(arr) = s.get("tools_disabled").and_then(|x| x.as_array()) {
        srv.tools_disabled = arr.iter().filter_map(|i| i.as_str().map(String::from)).collect();
    }
}

fn from_json(s: &Value) -> Option<Server> {
    let id = s.get("id").and_then(|x| x.as_str())?.to_string();
    let label = s.get("label").and_then(|x| x.as_str()).unwrap_or(&id).to_string();
    let transport = match s.get("transport").and_then(|x| x.as_str()) {
        Some("stdio") => Transport::Stdio {
            program: s.get("program").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            args: s.get("args").and_then(|x| x.as_array()).map(|a| {
                a.iter().filter_map(|i| i.as_str().map(String::from)).collect()
            }).unwrap_or_default(),
            workdir: s.get("workdir").and_then(|x| x.as_str()).map(String::from),
        },
        _ => Transport::Http {
            host: s.get("host").and_then(|x| x.as_str()).map(String::from),
            port: s.get("port").and_then(|x| x.as_u64())? as u16,
            path: s.get("path").and_then(|x| x.as_str()).unwrap_or("/mcp").to_string(),
        },
    };
    let enabled = s.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true);
    Some(Server {
        id, label, transport, origin: Origin::Manual, enabled, auto_restart: true,
        state: if enabled { SState::Down } else { SState::Disabled },
        pid: None, bound_host: None, server_name: None, version: None, protocol: None,
        tools: vec![], latency_ms: None, last_seen_ts: None, last_error: None, start_cmd: None,
        tools_disabled: vec![],
    })
}

fn parse_settings(v: &Value, mut st: crate::types::Settings) -> crate::types::Settings {
    if let Some(s) = v.get("settings") {
        if let Some(x) = s.get("refresh_secs").and_then(|x| x.as_u64()) { st.refresh_secs = x; }
        if let Some(x) = s.get("scan_from").and_then(|x| x.as_u64()) { st.scan_from = x as u16; }
        if let Some(x) = s.get("scan_to").and_then(|x| x.as_u64()) { st.scan_to = x as u16; }
        if let Some(x) = s.get("ui_port").and_then(|x| x.as_u64()) { st.ui_port = x as u16; }
        if let Some(x) = s.get("ui_bind").and_then(|x| x.as_str()) { st.ui_bind = x.to_string(); }
        if let Some(x) = s.get("network_enabled").and_then(|x| x.as_bool()) { st.network_enabled = x; }
        if let Some(arr) = s.get("network_ports").and_then(|x| x.as_array()) {
            let ports: Vec<u16> = arr.iter().filter_map(|x| x.as_u64()).filter(|p| *p >= 1 && *p <= 65535).map(|p| p as u16).collect();
            if !ports.is_empty() { st.network_ports = ports; }
        }
    }
    st
}

#[allow(unused_must_use)]
pub fn save_config(state: &AppState) {
    let servers = lockv(&state.servers).clone();
    let settings = lockv(&state.settings).clone();
    let mut arr = Vec::new();
    for s in &servers {
        // les serveurs découverts (scan local/réseau) ne sont PAS persistés : redécouverts au boot
        if s.origin == Origin::Discovered { continue; }
        let mut o = Map::new();
        o.insert("id".into(), json!(s.id));
        o.insert("label".into(), json!(s.label));
        o.insert("enabled".into(), json!(s.enabled));
        if !s.tools_disabled.is_empty() {
            o.insert("tools_disabled".into(), json!(s.tools_disabled));
        }
        match &s.transport {
            Transport::Http { host, port, path } => {
                o.insert("transport".into(), json!("http"));
                if let Some(h) = host { o.insert("host".into(), json!(h)); }
                o.insert("port".into(), json!(port));
                o.insert("path".into(), json!(path));
            }
            Transport::Stdio { program, args, workdir } => {
                o.insert("transport".into(), json!("stdio"));
                o.insert("program".into(), json!(program));
                o.insert("args".into(), json!(args));
                if let Some(w) = workdir { o.insert("workdir".into(), json!(w)); }
            }
        }
        arr.push(Value::Object(o));
    }
    let doc = json!({
        "servers": arr,
        "settings": {
            "refresh_secs": settings.refresh_secs,
            "scan_from": settings.scan_from,
            "scan_to": settings.scan_to,
            "ui_port": settings.ui_port,
            "ui_bind": settings.ui_bind,
            "network_enabled": settings.network_enabled,
            "network_ports": settings.network_ports,
        }
    });
    let pretty = serde_json::to_string_pretty(&doc).unwrap_or_default();
    let tmp = format!("{}.tmp", state.config_path);
    std::fs::write(&tmp, pretty).ok();
    std::fs::rename(&tmp, &state.config_path).ok();
}

// ---------- polling ----------

/// Nombre de tools actifs (tools_disabled écartés).
pub fn tools_actifs(s: &Server) -> usize {
    s.tools.iter().filter(|t| !s.tools_disabled.contains(&t.name)).count()
}

pub fn poll_all(state: &Arc<AppState>) {
    let ids: Vec<String> = lockv(&state.servers).iter().map(|s| s.id.clone()).collect();
    for id in ids {
        poll_one(state, &id);
    }
}

#[allow(unused_must_use)]
pub fn poll_one(state: &Arc<AppState>, id: &str) {
    let (transport, enabled) = {
        let servers = lockv(&state.servers);
        match servers.iter().find(|s| s.id == id) {
            Some(s) => (s.transport.clone(), s.enabled),
            None => return,
        }
    };
    if !enabled { return; }
    match transport {
        Transport::Http { host, port, path } => {
            let res = match &host {
                Some(h) => probe::probe_http_host(h, port, &path, 6000),
                None => probe::probe_http(port, &path, 6000),
            };
            let mut servers = lockv(&state.servers);
            if let Some(srv) = servers.iter_mut().find(|s| s.id == id) {
                if res.mcp_up {
                    let changed = srv.server_name != res.name || srv.state != SState::Up;
                    srv.state = SState::Up;
                    srv.bound_host = res.host.clone();
                    srv.server_name = res.name.clone();
                    srv.version = res.version.clone();
                    srv.protocol = res.protocol.clone();
                    srv.tools = res.tools.clone();
                    srv.latency_ms = Some(res.latency_ms);
                    srv.last_seen_ts = Some(crate::types::now_ts());
                    srv.last_error = None;
                    if changed {
                        add_log(state, "info", &format!(
                            "[{}] UP : {} v{} — {} tools ({} ms)", srv.label,
                            res.name.as_deref().unwrap_or("?"),
                            res.version.as_deref().unwrap_or("?"),
                            tools_actifs(srv), res.latency_ms));
                    }
                } else {
                    let was_up = srv.state == SState::Up;
                    // port fermé = DOWN ; port ouvert sans MCP = PORT-ONLY (zombie)
                    srv.state = if res.port_open { SState::PortOnly } else { SState::Down };
                    srv.last_error = res.error.clone();
                    srv.latency_ms = None;
                    if was_up {
                        add_log(state, "warn", &format!("[{}] PORT-ONLY : {}", srv.label,
                            res.error.as_deref().unwrap_or("?")));
                    }
                }
            }
        }
        Transport::Stdio { .. } => {
            let alive = {
                let mut children = lockv(&state.children);
                let mut still = Vec::new();
                for (cid, h) in children.iter_mut() {
                    if cid == id {
                        match h.child.try_wait() {
                            Ok(Some(_)) => still.push(cid.clone()),
                            Ok(None) => {}
                            Err(_) => still.push(cid.clone()),
                        }
                    }
                }
                for cid in still { children.remove(&cid); }
                children.contains_key(id)
            };
            let (enabled, has_cmd) = {
                let servers = lockv(&state.servers);
                match servers.iter().find(|s| s.id == id) {
                    Some(s) => (s.enabled, s.start_cmd.is_some() && s.auto_restart),
                    None => return,
                }
            };
            if alive {
                let mut servers = lockv(&state.servers);
                if let Some(srv) = servers.iter_mut().find(|s| s.id == id) {
                    srv.state = SState::Up;
                    srv.last_seen_ts = Some(crate::types::now_ts());
                }
            } else if enabled && has_cmd {
                // (re)démarrage auto avec handshake complet
                let cmd = {
                    let mut servers = lockv(&state.servers);
                    servers.iter_mut().find(|s| s.id == id).and_then(|s| s.start_cmd.clone())
                };
                if let Some(cmd) = cmd {
                    match probe::probe_stdio(&cmd.program, &cmd.args, cmd.workdir.as_deref(), 6000, true) {
                        Ok((p, handle)) => {
                            if let Some(h) = handle {
                                lockv(&state.children).insert(id.to_string(), h);
                            }
                            let mut servers = lockv(&state.servers);
                            if let Some(srv) = servers.iter_mut().find(|s| s.id == id) {
                                srv.state = SState::Up;
                                srv.pid = p.pid;
                                srv.server_name = p.name.clone();
                                srv.version = p.version.clone();
                                srv.protocol = p.protocol.clone();
                                srv.tools = p.tools.clone();
                                srv.latency_ms = Some(p.latency_ms);
                                srv.last_seen_ts = Some(crate::types::now_ts());
                                srv.last_error = None;
                            }
                            drop(servers);
                            add_log(state, "info", &format!("[{}] stdio démarré (pid {}) : {} — {} tools",
                                id, p.pid.map(|x| x.to_string()).unwrap_or_else(|| "?".into()),
                                p.name.as_deref().unwrap_or("?"), p.tools.len()));
                        }
                        Err(e) => {
                            let mut servers = lockv(&state.servers);
                            if let Some(srv) = servers.iter_mut().find(|s| s.id == id) {
                                srv.state = SState::Down;
                                srv.last_error = Some(e.clone());
                                srv.pid = None;
                            }
                            drop(servers);
                            add_log(state, "warn", &format!("[{}] stdio KO : {}", id, e));
                        }
                    }
                }
            } else {
                let mut servers = lockv(&state.servers);
                if let Some(srv) = servers.iter_mut().find(|s| s.id == id) {
                    srv.state = SState::Down;
                }
            }
        }
    }
}

// ---------- scan / découverte ----------

#[allow(unused_must_use)]
pub fn scan_ports(state: &Arc<AppState>, range: Option<(u16, u16)>) {
    let (from, to) = match range {
        Some(r) => r,
        None => { let s = lockv(&state.settings); (s.scan_from, s.scan_to) }
    };
    add_log(state, "info", &format!("scan {}..{} sur [::1]/127.0.0.1…", from, to));
    let mut found = 0u32;
    let mut batch: Vec<u16> = Vec::new();
    let mut open: Vec<(u16, String)> = Vec::new();
    for port in from..=to {
        if probe::port_open("::1", port, std::time::Duration::from_millis(150)) {
            open.push((port, "::1".into()));
        } else if probe::port_open("127.0.0.1", port, std::time::Duration::from_millis(150)) {
            open.push((port, "127.0.0.1".into()));
        }
        batch.push(port);
        if batch.len() >= 64 {
            open.sort();
            open.dedup();
            batch.clear();
        }
    }
    open.sort();
    open.dedup();
    let known: Vec<u16> = lockv(&state.servers).iter().filter_map(|s| match &s.transport {
        Transport::Http { host, port, .. } if host.is_none() => Some(*port),
        _ => None,
    }).collect();
    for (port, host) in open {
        if known.contains(&port) { continue; }
        let p = probe::probe_http(port, "/mcp", 4000);
        if p.mcp_up {
                found += 1;
                let id = format!("disc-{}", port);
                let label = p.name.clone().unwrap_or_else(|| format!("port-{}", port));
                let mut servers = lockv(&state.servers);
                if !servers.iter().any(|s| s.id == id) {
                    servers.push(Server {
                        id: id.clone(), label, transport: Transport::Http { host: Some(host.clone()), port, path: "/mcp".into() },
                        origin: Origin::Discovered, enabled: true, state: SState::Up, auto_restart: true,
                        pid: p.pid, bound_host: Some(host),
                        server_name: p.name.clone(), version: p.version.clone(),
                        protocol: p.protocol.clone(), tools: p.tools, latency_ms: Some(p.latency_ms),
                        last_seen_ts: Some(crate::types::now_ts()), last_error: None, start_cmd: None,
                        tools_disabled: vec![],
                    });
                    drop(servers);
                    add_log(state, "info", &format!("[scan] serveur MCP découvert sur port {} : {} v{}",
                        port, p.name.as_deref().unwrap_or("?"), p.version.as_deref().unwrap_or("?")));
                }
            }
    }
    add_log(state, "info", &format!("scan terminé : {} nouveaux serveurs MCP", found));
}

// ---------- scan réseau local (MCP distants) ----------

#[allow(unused_must_use)]
pub fn scan_network(state: &Arc<AppState>, range: Option<(u16, u16)>) {
    let (from, to) = match range {
        Some(r) => r,
        None => { let s = lockv(&state.settings); (s.scan_from, s.scan_to) }
    };
    let prefixes = network_prefixes();
    if prefixes.is_empty() {
        add_log(state, "warn", "scan réseau : aucune interface IPv4 trouvée");
        return;
    }
    let total_found = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    for (prefix, self_ip) in &prefixes {
        add_log(state, "info", &format!("scan réseau {}.1-254 (self={}) ports {}..{} (parallèle)", prefix, self_ip, from, to));
        let (found, _tested) = scan_network_range(state, prefix, self_ip, from, to);
        total_found.fetch_add(found, std::sync::atomic::Ordering::Relaxed);
    }
    let found = total_found.load(std::sync::atomic::Ordering::Relaxed);
    add_log(state, "info", &format!("scan réseau terminé : {} nouveau(x) serveur(s) MCP distant(s)", found));
}

/// Toutes les IPv4 locales (toutes interfaces), dédupliquées.
/// Sources : ipconfig (Windows) / hostname -I (unix) + fallback UDP (interface de routage).
/// Le fallback UDP seul peut renvoyer l'IP VPN — d'où le parse des interfaces en priorité.
pub fn local_ips() -> Vec<String> {
    let mut ips = std::collections::BTreeSet::new();
    let output = if cfg!(windows) {
        Command::new("ipconfig").output()
    } else {
        Command::new("hostname").arg("-I").output()
    };
    if let Ok(o) = output {
        let text = String::from_utf8_lossy(&o.stdout).to_string();
        // ne prendre que les lignes "IPv4" (exclut masques et lignes "Passerelle par défaut"/"Default Gateway")
        // — le marqueur "ipv4" existe en FR ("Adresse IPv4") comme en EN ("IPv4 Address")
        for line in text.lines() {
            if !line.to_lowercase().contains("ipv4") { continue; }
            for tok in line.split_whitespace() {
                let tok = tok.trim_end_matches(',').trim_start_matches('.');
                let parts: Vec<&str> = tok.split('.').collect();
                let est_ip = parts.len() == 4
                    && parts.iter().all(|p| !p.is_empty() && p.len() <= 3 && p.chars().all(|c| c.is_ascii_digit()) && p.parse::<u8>().is_ok());
                if est_ip {
                    ips.insert(tok.to_string());
                }
            }
        }
    }
    // fallback UDP : interface de routage sortante (paquet non émis)
    if let Ok(s) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if s.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = s.local_addr() {
                ips.insert(addr.ip().to_string());
            }
        }
    }
    ips.into_iter().collect()
}

/// Préfixes /24 distincts des interfaces locales.
pub fn network_prefixes() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for ip in local_ips() {
        if let Some((prefix, _)) = ip.rsplit_once('.') {
            if !out.iter().any(|(p, _)| p == prefix) {
                out.push((prefix.to_string(), ip));
            }
        }
    }
    out
}

/// Parallélisé : 254 hôtes × plage de ports. Retourne (nouveauserveurs, hôtes testés).
#[allow(unused_must_use)]
pub fn scan_network_range(state: &Arc<AppState>, prefix: &str, self_ip: &str, from: u16, to: u16) -> (u32, u32) {
    let ports: Vec<u16> = (from..=to).collect();
    let known_hosts_ports: Vec<(Option<String>, u16)> = lockv(&state.servers).iter().filter_map(|s| match &s.transport {
        Transport::Http { host, port, .. } => Some((host.clone(), *port)),
        _ => None,
    }).collect();

    use std::sync::atomic::{AtomicU32, Ordering};
    let found = std::sync::Arc::new(AtomicU32::new(0));
    let tested = std::sync::Arc::new(AtomicU32::new(0));
    let mut handles = Vec::new();
    for last in 1u16..=254 {
        let ip = format!("{}.{}", prefix, last);
        if ip == self_ip { continue; } // ne pas se scanner soi-même (scan loopback déjà couvert)
        let ports = ports.clone();
        let known = known_hosts_ports.clone();
        let state = state.clone();
        let found = found.clone();
        let tested = tested.clone();
        handles.push(std::thread::spawn(move || {
            let mut hit = false;
            for port in &ports {
                if !probe::port_open(&ip, *port, std::time::Duration::from_millis(400)) { continue; }
                hit = true;
                if known.iter().any(|(h, p)| h.as_deref() == Some(ip.as_str()) && p == port) { continue; }
                let p = probe::probe_http_host(&ip, *port, "/mcp", 3000);
                if p.mcp_up {
                    found.fetch_add(1, Ordering::Relaxed);
                    let label = p.name.clone().unwrap_or_else(|| format!("{}:{}", ip, port));
                    let mut servers = lockv(&state.servers);
                    if !servers.iter().any(|s| matches!(&s.transport,
                        Transport::Http { host: Some(h), port: sp, .. } if h == &ip && sp == port))
                    {
                        servers.push(Server {
                            id: format!("net-{}.{}", ip, port), label,
                            transport: Transport::Http { host: Some(ip.clone()), port: *port, path: "/mcp".into() },
                            origin: Origin::Discovered, enabled: true, state: SState::Up, auto_restart: true,
                            pid: None, bound_host: Some(ip.clone()),
                            server_name: p.name.clone(), version: p.version.clone(),
                            protocol: p.protocol.clone(), tools: p.tools, latency_ms: Some(p.latency_ms),
                            last_seen_ts: Some(crate::types::now_ts()), last_error: None,
                            start_cmd: None, tools_disabled: vec![],
                        });
                        drop(servers);
                        add_log(&state, "info", &format!("[réseau] serveur MCP distant découvert : {}:{} — {} v{}",
                            ip, port, p.name.as_deref().unwrap_or("?"), p.version.as_deref().unwrap_or("?")));
                    }
                }
            }
            if hit { tested.fetch_add(1, Ordering::Relaxed); }
        }));
    }
    for h in handles { let _ = h.join(); }
    (found.load(Ordering::Relaxed), tested.load(Ordering::Relaxed))
}

// ---------- découverte des pairs command-center ----------

#[allow(unused_must_use)]
pub fn discover_peers(state: &Arc<AppState>) {
    if !lockv(&state.settings).network_enabled { return; }
    let ports = lockv(&state.settings).network_ports.clone();
    let own_port = lockv(&state.settings).ui_port;
    let prefixes = network_prefixes();
    let my_ips: Vec<String> = crate::manage::local_ips();
    let mut hosts: Vec<String> = vec!["127.0.0.1".to_string()]; // autres instances locales
    for (prefix, self_ip) in &prefixes {
        for h in probe::arp_hosts(prefix, self_ip) {
            if !hosts.contains(&h) { hosts.push(h); }
        }
    }
    // une cible qui est une de NOS ips sur NOTRE port UI = nous-mêmes -> skip
    hosts.retain(|h| !(my_ips.contains(h)));
    add_log(state, "info", &format!("pairs : sondage {} cibles (ARP + loopback) sur ports UI {:?}", hosts.len(), ports));

    use std::sync::atomic::{AtomicU32, Ordering};
    let new_peers = std::sync::Arc::new(AtomicU32::new(0));
    let mut handles = Vec::new();
    for ip in hosts {
        let ports = ports.clone();
        let state = state.clone();
        let new_peers = new_peers.clone();
        handles.push(std::thread::spawn(move || {
            for port in &ports {
                if *port == own_port { continue; } // ne jamais se sonder soi-même sur notre port
                let pr = probe::probe_peer(&ip, *port);
                if pr.alive {
                    if let Some(servers) = pr.servers {
                        let count = servers.get("stats").and_then(|s| s.get("total")).and_then(|x| x.as_u64()).unwrap_or(0);
                        let mut peers = lockv(&state.peers);
                        let is_new = !peers.contains_key(&ip);
                        peers.insert(ip.clone(), crate::types::Peer {
                            ip: ip.clone(), ui_port: *port,
                            last_seen: crate::types::now_ts(), servers,
                        });
                        drop(peers);
                        if is_new { new_peers.fetch_add(1, Ordering::Relaxed); }
                        add_log(&state, "info", &format!("[pairs] command-center détecté sur {}:{} ({} serveurs)",
                            ip, port, count));
                        break;
                    }
                }
            }
        }));
    }
    for h in handles { let _ = h.join(); }
    let total = lockv(&state.peers).len();
    let nouveaux = new_peers.load(Ordering::Relaxed);
    add_log(state, "info", &format!("pairs : {} peer(s) connu(s) ({} nouveau(x))", total, nouveaux));
}

/// Importe depuis un pair tous les serveurs HTTP qu'on n'a pas encore (host:ip).
#[allow(unused_must_use)]
pub fn import_peer(state: &Arc<AppState>, ip: &str) -> Value {
    let snapshot = {
        let peers = lockv(&state.peers);
        peers.get(ip).map(|p| (p.ui_port, p.servers.clone()))
    };
    let (ui_port, snapshot) = match snapshot {
        Some(x) => x,
        None => return json!({"ok": false, "error": "pair inconnu"}),
    };
    let mut added = 0usize;
    if let Some(arr) = snapshot.get("servers").and_then(|x| x.as_array()) {
        for s in arr {
            if s.get("transport").and_then(|x| x.as_str()) != Some("http") { continue; }
            if s.get("enabled") != Some(&json!(true)) { continue; }
            let (host, port, path, name, version) = (
                s.get("transport_info").and_then(|t| t.get("host")).and_then(|x| x.as_str()).map(String::from),
                s.get("transport_info").and_then(|t| t.get("port")).and_then(|x| x.as_u64()).unwrap_or(0) as u16,
                s.get("transport_info").and_then(|t| t.get("path")).and_then(|x| x.as_str()).unwrap_or("/mcp").to_string(),
                s.get("server_name").and_then(|x| x.as_str()).map(String::from),
                s.get("version").and_then(|x| x.as_str()).map(String::from),
            );
            // host: null (localhost du pair) -> réécrire vers l'IP du pair si ce n'est pas notre machine
            let host = match host {
                Some(h) => h,
                None if ip == "127.0.0.1" => continue, // même machine : localhost reste localhost
                None => ip.to_string(),                 // pair distant : pointer son IP
            };
            let mut servers = lockv(&state.servers);
            let already = servers.iter().any(|srv| matches!(&srv.transport,
                Transport::Http { host: Some(h), port: p, .. } if h == &host && p == &port));
            if already { continue; }
            let id = format!("net-{}.{}", host, port);
            if servers.iter().any(|s| s.id == id) { continue; }
            servers.push(Server {
                id, label: name.clone().unwrap_or_else(|| format!("{}:{}", host, port)),
                transport: Transport::Http { host: Some(host.clone()), port, path },
                origin: Origin::Discovered, enabled: true, state: SState::Down, auto_restart: true,
                pid: None, bound_host: None, server_name: name, version,
                protocol: None, tools: vec![], latency_ms: None,
                last_seen_ts: None, last_error: None, start_cmd: None, tools_disabled: vec![],
            });
            added += 1;
        }
    }
    drop(snapshot);
    save_config(state);
    add_log(state, "info", &format!("[pairs] import depuis {} (UI {}) : {} serveur(s) ajouté(s)", ip, ui_port, added));
    json!({"ok": true, "added": added})
}

// ---------- actions ----------

#[allow(unused_must_use)]
pub fn handle_action(state: &Arc<AppState>, body: &str) -> Value {
    let req: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return json!({"ok": false, "error": format!("body invalide: {}", e)}),
    };
    let action = req.get("action").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let id = req.get("id").and_then(|x| x.as_str()).map(String::from);

    match action.as_str() {
        "toggle" | "refresh" | "kill_stdio" | "start_stdio" => {
            let id = match id { Some(i) => i, None => return json!({"ok": false, "error": "id manquant"}) };
            match action.as_str() {
                "toggle" => {
                    let now_enabled = {
                        let mut servers = lockv(&state.servers);
                        match servers.iter_mut().find(|s| s.id == id) {
                            Some(s) => {
                                s.enabled = !s.enabled;
                                s.state = if s.enabled { SState::Down } else { SState::Disabled };
                                if s.enabled {
                                    s.start_cmd = start_cmd_for(s);
                                    s.auto_restart = true;
                                } else {
                                    s.auto_restart = false;
                                }
                                s.enabled
                            }
                            None => return json!({"ok": false, "error": "serveur inconnu"}),
                        }
                    };
                    save_config(state);
                    add_log(state, "info", &format!("[{}] {}", id, if now_enabled { "activé" } else { "désactivé" }));
                    json!({"ok": true, "enabled": now_enabled})
                }
                "refresh" => { poll_one(state, &id); json!({"ok": true}) }
                "start_stdio" => {
                    {
                        let mut servers = lockv(&state.servers);
                        match servers.iter_mut().find(|s| s.id == id) {
                            Some(s) => {
                                // Start explicite : réactive le serveur même désactivé
                                // (sinon poll_one sort sur !enabled et le bouton ne fait rien)
                                if !s.enabled {
                                    s.enabled = true;
                                    s.state = SState::Down;
                                }
                                if s.start_cmd.is_none() {
                                    s.start_cmd = start_cmd_for(s);
                                }
                                s.auto_restart = true;
                            }
                            None => return json!({"ok": false, "error": "serveur inconnu"}),
                        }
                    }
                    save_config(state);
                    poll_one(state, &id);
                    json!({"ok": true})
                }
                "kill_stdio" => {
                    {
                        let mut servers = lockv(&state.servers);
                        if let Some(s) = servers.iter_mut().find(|s| s.id == id) {
                            s.auto_restart = false;
                            // feedback immédiat : sinon le pid fantôme reste affiché
                            s.pid = None;
                            s.state = SState::Down;
                        }
                    }
                    let mut children = lockv(&state.children);
                    if let Some(h) = children.get_mut(&id) {
                        let _ = h.child.kill();
                        let _ = h.child.wait();
                    }
                    children.remove(&id);
                    drop(children);
                    poll_one(state, &id);
                    add_log(state, "info", &format!("[{}] stdio arrêté", id));
                    json!({"ok": true})
                }
                _ => unreachable!(),
            }
        }
        "add" => {
            let transport = req.get("transport").and_then(|x| x.as_str()).unwrap_or("http");
            let srv = if transport == "stdio" {
                let program = req.get("program").and_then(|x| x.as_str()).unwrap_or("");
                if program.is_empty() { return json!({"ok": false, "error": "program manquant"}); }
                let sargs: Vec<String> = req.get("args").and_then(|x| x.as_array()).map(|a| {
                    a.iter().filter_map(|i| i.as_str().map(String::from)).collect()
                }).unwrap_or_default();
                let sworkdir = req.get("workdir").and_then(|x| x.as_str()).map(String::from);
                Server {
                    id: format!("manual-stdio-{}", now_compact()),
                    label: req.get("label").and_then(|x| x.as_str()).unwrap_or(program).to_string(),
                    transport: Transport::Stdio {
                        program: program.to_string(),
                        args: sargs.clone(),
                        workdir: sworkdir.clone(),
                    },
                    origin: Origin::Manual, enabled: true, state: SState::Down,
                    pid: None, bound_host: None, server_name: None, version: None,
                    protocol: None, tools: vec![], latency_ms: None,
                    last_seen_ts: None, last_error: None,
                    start_cmd: Some(StartCmd { program: program.to_string(), args: sargs, workdir: sworkdir }),
                    auto_restart: true,
                    tools_disabled: vec![],
                }
            } else {
                let port = match req.get("port").and_then(|x| x.as_u64()) {
                    Some(p) if (1..=65535).contains(&p) => p as u16,
                    _ => return json!({"ok": false, "error": "port invalide"}),
                };
                Server {
                    id: format!("manual-http-{}", now_compact()),
                    label: req.get("label").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    transport: Transport::Http {
                        host: req.get("host").and_then(|x| x.as_str()).map(String::from),
                        port,
                        path: req.get("path").and_then(|x| x.as_str()).unwrap_or("/mcp").to_string(),
                    },
                    origin: Origin::Manual, enabled: true, state: SState::Down,
                    pid: None, bound_host: None, server_name: None, version: None,
                    protocol: None, tools: vec![], latency_ms: None,
                    last_seen_ts: None, last_error: None, start_cmd: None, auto_restart: true,
                    tools_disabled: vec![],
                }
            };
            let label = srv.label.clone();
            let sid = srv.id.clone();
            lockv(&state.servers).push(srv);
            save_config(state);
            add_log(state, "info", &format!("[+] serveur ajouté : {} ({})", label, sid));
            json!({"ok": true, "id": sid})
        }
        "remove" => {
            let id = match id { Some(i) => i, None => return json!({"ok": false, "error": "id manquant"}) };
            let mut children = lockv(&state.children);
            if let Some(h) = children.get_mut(&id) {
                let _ = h.child.kill();
                let _ = h.child.wait();
            }
            children.remove(&id);
            drop(children);
            let removed = {
                let mut servers = lockv(&state.servers);
                let before = servers.len();
                servers.retain(|s| s.id != id);
                before != servers.len()
            };
            save_config(state);
            add_log(state, "info", &format!("[-] serveur supprimé : {} ({})", id, if removed { "ok" } else { "introuvable" }));
            json!({"ok": removed})
        }
        "scan" => {
            let range = match (req.get("from").and_then(|x| x.as_u64()), req.get("to").and_then(|x| x.as_u64())) {
                (Some(a), Some(b)) if (1..=65535).contains(&a) && (1..=65535).contains(&b) && a <= b => Some((a as u16, b as u16)),
                _ => None,
            };
            scan_ports(state, range);
            json!({"ok": true})
        }
        "scan_network" => {
            let range = match (req.get("from").and_then(|x| x.as_u64()), req.get("to").and_then(|x| x.as_u64())) {
                (Some(a), Some(b)) if (1..=65535).contains(&a) && (1..=65535).contains(&b) && a <= b => Some((a as u16, b as u16)),
                _ => None,
            };
            let st = state.clone();
            std::thread::spawn(move || scan_network(&st, range));
            json!({"ok": true, "started": true, "note": "scan asynchrone — voir le journal pour les détections"})
        }
        "discover_peers" => { discover_peers(state); json!({"ok": true}) }
        "import_peer" => {
            let ip = req.get("ip").and_then(|x| x.as_str()).unwrap_or("");
            if ip.is_empty() { return json!({"ok": false, "error": "ip manquante"}); }
            import_peer(state, ip)
        }
        "toggle_tool" => {
            let sid = match req.get("id").and_then(|x| x.as_str()) {
                Some(x) => x.to_string(),
                None => return json!({"ok": false, "error": "id manquant"}),
            };
            let tool = match req.get("tool").and_then(|x| x.as_str()) {
                Some(x) => x.to_string(),
                None => return json!({"ok": false, "error": "tool manquant"}),
            };
            let (now_disabled, count) = {
                let mut servers = lockv(&state.servers);
                match servers.iter_mut().find(|s| s.id == sid) {
                    Some(s) => {
                        if let Some(pos) = s.tools_disabled.iter().position(|t| t == &tool) {
                            s.tools_disabled.remove(pos);
                            (false, tools_actifs(s))
                        } else {
                            s.tools_disabled.push(tool.clone());
                            (true, tools_actifs(s))
                        }
                    }
                    None => return json!({"ok": false, "error": "serveur inconnu"}),
                }
            };
            save_config(state);
            add_log(state, "info", &format!("[{}] tool '{}' {} ({} actifs)", sid, tool,
                if now_disabled { "DÉSACTIVÉ" } else { "réactivé" }, count));
            json!({"ok": true, "disabled": now_disabled, "tools_actifs": count})
        }
        _ => json!({"ok": false, "error": format!("action inconnue: {}", action)}),
    }
}

fn now_compact() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64 % 100_000_000).unwrap_or(0)
}

pub fn start_cmd_for(srv: &Server) -> Option<StartCmd> {
    match &srv.transport {
        Transport::Stdio { program, args, workdir } => Some(StartCmd {
            program: program.clone(), args: args.clone(), workdir: workdir.clone(),
        }),
        Transport::Http { .. } => None,
    }
}

// ---------- sérialisation UI ----------

fn server_json(s: &Server) -> Value {
    let (ttype, extra) = match &s.transport {
        Transport::Http { host, port, path } => ("http", json!({"host": host, "port": port, "path": path})),
        Transport::Stdio { program, args, workdir } => ("stdio", json!({
            "program": program, "args": args, "workdir": workdir,
        })),
    };
    let origin = s.origin.as_str();
    json!({
        "id": s.id, "label": s.label, "transport": ttype, "transport_info": extra,
        "origin": origin, "enabled": s.enabled, "state": s.state.as_str(),
        "pid": s.pid, "bound_host": s.bound_host,
        "server_name": s.server_name, "version": s.version, "protocol": s.protocol,
        "tools_count": tools_actifs(s), "tools_total": s.tools.len(),
        "tools_disabled": s.tools_disabled,
        "latency_ms": s.latency_ms,
        "last_seen": s.last_seen_ts, "last_error": s.last_error,
        "can_start": s.start_cmd.is_some(), "auto_restart": s.auto_restart,
    })
}

pub fn state_json(state: &Arc<AppState>) -> Value {
    let servers = lockv(&state.servers);
    let log = lockv(&state.log);
    let settings = lockv(&state.settings);
    let peers = lockv(&state.peers);
    let list: Vec<Value> = servers.iter().map(server_json).collect();
    let up = servers.iter().filter(|s| s.state == SState::Up).count();
    let total = servers.len();
    let tools: usize = servers.iter().map(tools_actifs).sum();
    let logarr: Vec<Value> = log.iter().rev().take(80).map(|e| json!({
        "ts": e.ts, "level": e.level, "msg": e.msg
    })).collect();
    json!({
        "servers": list, "log": logarr,
        "stats": {"up": up, "total": total, "tools": tools},
        "settings": {"refresh_secs": settings.refresh_secs, "ui_port": settings.ui_port,
                     "scan_from": settings.scan_from, "scan_to": settings.scan_to,
                     "network_enabled": settings.network_enabled,
                     "network_ports": settings.network_ports},
        "peers": peers.values().map(|p| json!({
            "ip": p.ip, "ui_port": p.ui_port, "last_seen": p.last_seen,
            "stats": p.servers.get("stats"),
            "servers": p.servers.get("servers"),
        })).collect::<Vec<_>>(),
    })
}

pub fn detail_json(state: &Arc<AppState>, id: &str) -> Value {
    let servers = lockv(&state.servers);
    match servers.iter().find(|s| s.id == id) {
        Some(s) => {
            let mut o = match server_json(s) { Value::Object(o) => o, _ => Map::new() };
            let tools: Vec<Value> = s.tools.iter().map(|t| json!({
                "name": t.name, "desc": t.desc,
                "disabled": s.tools_disabled.contains(&t.name),
            })).collect();
            o.insert("tools_list".into(), json!(tools));
            Value::Object(o)
        }
        None => json!({"ok": false, "error": "serveur inconnu"}),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AppState, Settings};
    use std::collections::VecDeque;
    use std::sync::Mutex;

    fn tmp_config(nom: &str, contenu: Option<&str>) -> String {
        let dir = std::env::temp_dir().join("mcpcc_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(nom);
        if let Some(c) = contenu {
            std::fs::write(&path, c).unwrap();
        }
        path.to_string_lossy().to_string()
    }

    #[test]
    fn config_absente_renvoie_les_7_seeds() {
        let (servers, st) = load_config(&tmp_config("inexistant_aucune_chance.json", None));
        assert_eq!(servers.len(), 7);
        assert_eq!(servers[0].id, "seed-overmind");
        assert_eq!(st.refresh_secs, 15);
        for id in ["seed-debats", "seed-news-btc", "seed-news"] {
            let s = servers.iter().find(|s| s.id == id).unwrap();
            assert!(!s.enabled, "{} doit etre desactive par defaut", id);
            assert_eq!(s.state, SState::Disabled);
        }
        for id in ["seed-overmind", "seed-discord", "seed-postgresql", "seed-x"] {
            assert!(servers.iter().find(|s| s.id == id).unwrap().enabled);
        }
    }

    #[test]
    fn load_config_reactive_un_seed_et_ajoute_un_inconnu() {
        let cfg = "{\"servers\":[{\"id\":\"seed-debats\",\"enabled\":true},{\"id\":\"mon-mcp\",\"label\":\"Mon MCP\",\"transport\":\"http\",\"port\":4567,\"path\":\"/x\",\"enabled\":false}]}";
        let (servers, _) = load_config(&tmp_config("cfg_mixte.json", Some(cfg)));
        let debats = servers.iter().find(|s| s.id == "seed-debats").unwrap();
        assert!(debats.enabled);
        assert_eq!(debats.state, SState::Down);
        let m = servers.iter().find(|s| s.id == "mon-mcp").unwrap();
        assert_eq!(m.origin, Origin::Manual);
        assert!(!m.enabled);
        match &m.transport {
            Transport::Http { host, port, path } => { assert!(host.is_none()); assert_eq!(*port, 4567); assert_eq!(path, "/x"); }
            _ => panic!("transport http attendu"),
        }
    }

    #[test]
    fn load_config_reconstruit_start_cmd_stdio_persiste() {
        // LE bug v0.1 : un stdio recharge du disque sans start_cmd n'etait jamais demarre au boot
        let cfg = "{\"servers\":[{\"id\":\"man-s\",\"label\":\"S\",\"transport\":\"stdio\",\"program\":\"node\",\"args\":[\"a.js\",\"--p\",\"1\"],\"workdir\":\"C:/w\",\"enabled\":true}]}";
        let (servers, _) = load_config(&tmp_config("cfg_stdio.json", Some(cfg)));
        let s = servers.iter().find(|s| s.id == "man-s").unwrap();
        let cmd = s.start_cmd.as_ref().expect("start_cmd doit etre reconstruit au load");
        assert_eq!(cmd.program, "node");
        assert_eq!(cmd.args, vec!["a.js".to_string(), "--p".to_string(), "1".to_string()]);
        assert_eq!(cmd.workdir.as_deref(), Some("C:/w"));
        assert!(s.auto_restart);
    }

    #[test]
    fn save_puis_load_roundtrip_stdio() {
        let path = tmp_config("cfg_roundtrip.json", None);
        let state = AppState {
            servers: Mutex::new(Vec::new()),
            log: Mutex::new(VecDeque::new()),
            settings: Mutex::new(Settings::default()),
            children: Mutex::new(std::collections::HashMap::new()),
            peers: Mutex::new(std::collections::HashMap::new()),
            config_path: path.clone(),
        };
        {
            let mut servers = lockv(&state.servers);
            servers.push(Server {
                id: "rt1".into(), label: "RT".into(),
                transport: Transport::Stdio { program: "prog".into(), args: vec!["-x".into()], workdir: Some("W".into()) },
                origin: Origin::Manual, enabled: true, auto_restart: true, state: SState::Up,
                pid: Some(42), bound_host: None, server_name: Some("n".into()), version: Some("1".into()),
                protocol: None, tools: vec![], latency_ms: Some(3),
                last_seen_ts: Some(1), last_error: None,
                start_cmd: Some(StartCmd { program: "prog".into(), args: vec!["-x".into()], workdir: Some("W".into()) }),
                tools_disabled: vec![],
            });
        }
        save_config(&state);
        let (servers, _) = load_config(&path);
        let s = servers.iter().find(|s| s.id == "rt1").expect("doit etre persiste");
        match &s.transport {
            Transport::Stdio { program, args, workdir } => {
                assert_eq!(program, "prog");
                assert_eq!(args, &vec!["-x".to_string()]);
                assert_eq!(workdir.as_deref(), Some("W"));
            }
            _ => panic!("stdio attendu apres round-trip"),
        }
        assert!(s.start_cmd.is_some(), "start_cmd survit au round-trip");
    }

    #[test]
    fn settings_sont_parsees() {
        let cfg = "{\"servers\":[],\"settings\":{\"refresh_secs\":42,\"scan_from\":1000,\"scan_to\":1009,\"ui_port\":6000}}";
        let (_, st) = load_config(&tmp_config("cfg_settings.json", Some(cfg)));
        assert_eq!(st.refresh_secs, 42);
        assert_eq!((st.scan_from, st.scan_to), (1000, 1009));
        assert_eq!(st.ui_port, 6000);
    }

    #[test]
    fn toggle_tool_desactive_et_reactive() {
        let state = AppState {
            servers: Mutex::new(vec![
                Server { id: "s".into(), label: "S".into(),
                    transport: Transport::Http { host: None, port: 1, path: "/mcp".into() },
                    origin: Origin::Seed, enabled: true, auto_restart: true, state: SState::Up,
                    pid: None, bound_host: None, server_name: None, version: None, protocol: None,
                    tools: vec![
                        crate::types::ToolInfo { name: "a".into(), desc: String::new() },
                        crate::types::ToolInfo { name: "b".into(), desc: String::new() },
                    ],
                    latency_ms: None, last_seen_ts: None, last_error: None, start_cmd: None,
                    tools_disabled: vec![] },
            ]),
            log: Mutex::new(VecDeque::new()),
            settings: Mutex::new(Settings::default()),
            children: Mutex::new(std::collections::HashMap::new()),
            peers: Mutex::new(std::collections::HashMap::new()),
            config_path: String::new(),
        };
        let arc = Arc::new(state);
        let body = serde_json::to_string(&serde_json::json!({"action": "toggle_tool", "id": "s", "tool": "a"})).unwrap();
        let j = handle_action(&arc, &body);
        assert_eq!(j["ok"], json!(true));
        assert_eq!(j["disabled"], json!(true));
        assert_eq!(j["tools_actifs"], json!(1));
        let j2 = handle_action(&arc, &body);
        assert_eq!(j2["disabled"], json!(false));
        assert_eq!(j2["tools_actifs"], json!(2));
    }

    #[test]
    fn tools_actifs_exclut_les_desactives() {
        let state = AppState {
            servers: Mutex::new(vec![
                Server { id: "x".into(), label: "X".into(),
                    transport: Transport::Http { host: None, port: 9, path: "/mcp".into() },
                    origin: Origin::Seed, enabled: true, auto_restart: true, state: SState::Up,
                    pid: None, bound_host: None, server_name: None, version: None, protocol: None,
                    tools: vec![
                        crate::types::ToolInfo { name: "a".into(), desc: String::new() },
                        crate::types::ToolInfo { name: "b".into(), desc: String::new() },
                        crate::types::ToolInfo { name: "c".into(), desc: String::new() },
                    ],
                    latency_ms: None, last_seen_ts: None, last_error: None, start_cmd: None,
                    tools_disabled: vec!["a".into(), "c".into()] },
            ]),
            log: Mutex::new(VecDeque::new()),
            settings: Mutex::new(Settings::default()),
            children: Mutex::new(std::collections::HashMap::new()),
            peers: Mutex::new(std::collections::HashMap::new()),
            config_path: String::new(),
        };
        let servers = lockv(&state.servers);
        assert_eq!(tools_actifs(&servers[0]), 1);
    }

    #[test]
    fn state_json_compte_bon_les_stats() {
        let state = AppState {
            servers: Mutex::new(vec![
                Server { id: "a".into(), label: "A".into(), transport: Transport::Http { host: None, port: 1, path: "/mcp".into() },
                    origin: Origin::Seed, enabled: true, auto_restart: true, state: SState::Up,
                    pid: None, bound_host: None, server_name: None, version: None, protocol: None,
                    tools: vec![crate::types::ToolInfo { name: "t".into(), desc: String::new() }],
                    latency_ms: None, last_seen_ts: None, last_error: None, start_cmd: None,
                    tools_disabled: vec![] },
                Server { id: "b".into(), label: "B".into(), transport: Transport::Http { host: None, port: 2, path: "/mcp".into() },
                    origin: Origin::Seed, enabled: false, auto_restart: true, state: SState::Disabled,
                    pid: None, bound_host: None, server_name: None, version: None, protocol: None,
                    tools: vec![], latency_ms: None, last_seen_ts: None, last_error: None, start_cmd: None,
                    tools_disabled: vec![] },
            ]),
            log: Mutex::new(VecDeque::new()),
            settings: Mutex::new(Settings::default()),
            children: Mutex::new(std::collections::HashMap::new()),
            peers: Mutex::new(std::collections::HashMap::new()),
            config_path: String::new(),
        };
        let j = state_json(&Arc::new(state));
        assert_eq!(j["stats"]["total"], serde_json::json!(2));
        assert_eq!(j["stats"]["up"], serde_json::json!(1));
        assert_eq!(j["stats"]["tools"], serde_json::json!(1));
        assert_eq!(j["servers"][1]["state"], serde_json::json!("DISABLED"));
        assert_eq!(j["servers"][1]["origin"], serde_json::json!("seed"));
    }
}
