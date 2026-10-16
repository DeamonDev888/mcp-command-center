use serde_json::{json, Map, Value};
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
        transport: Transport::Http { port, path: "/mcp".to_string() },
        origin: Origin::Seed, enabled, auto_restart: true,
        state: if enabled { SState::Down } else { SState::Disabled },
        pid: None, bound_host: None, server_name: None,
        version: None, protocol: None, tools: vec![], latency_ms: None,
        last_seen_ts: None, last_error: None, start_cmd: None,
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
                srv.transport = Transport::Http { port: p as u16, path: path.to_string() };
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
    })
}

fn parse_settings(v: &Value, mut st: crate::types::Settings) -> crate::types::Settings {
    if let Some(s) = v.get("settings") {
        if let Some(x) = s.get("refresh_secs").and_then(|x| x.as_u64()) { st.refresh_secs = x; }
        if let Some(x) = s.get("scan_from").and_then(|x| x.as_u64()) { st.scan_from = x as u16; }
        if let Some(x) = s.get("scan_to").and_then(|x| x.as_u64()) { st.scan_to = x as u16; }
        if let Some(x) = s.get("ui_port").and_then(|x| x.as_u64()) { st.ui_port = x as u16; }
    }
    st
}

#[allow(unused_must_use)]
pub fn save_config(state: &AppState) {
    let servers = lockv(&state.servers).clone();
    let settings = lockv(&state.settings).clone();
    let mut arr = Vec::new();
    for s in &servers {
        let mut o = Map::new();
        o.insert("id".into(), json!(s.id));
        o.insert("label".into(), json!(s.label));
        o.insert("enabled".into(), json!(s.enabled));
        match &s.transport {
            Transport::Http { port, path } => {
                o.insert("transport".into(), json!("http"));
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
        }
    });
    let pretty = serde_json::to_string_pretty(&doc).unwrap_or_default();
    let tmp = format!("{}.tmp", state.config_path);
    std::fs::write(&tmp, pretty).ok();
    std::fs::rename(&tmp, &state.config_path).ok();
}

// ---------- polling ----------

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
        Transport::Http { port, path } => {
            let res = probe::probe_http(port, &path, 6000);
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
                            srv.tools.len(), res.latency_ms));
                    }
                } else {
                    let was_up = srv.state == SState::Up;
                    srv.state = SState::PortOnly;
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
    let known: Vec<u16> = lockv(&state.servers).iter().filter_map(|s| match s.transport {
        Transport::Http { port, .. } => Some(port),
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
                        id: id.clone(), label, transport: Transport::Http { port, path: "/mcp".into() },
                        origin: Origin::Discovered, enabled: true, state: SState::Up, auto_restart: true,
                        pid: p.pid, bound_host: Some(host),
                        server_name: p.name.clone(), version: p.version.clone(),
                        protocol: p.protocol.clone(), tools: p.tools, latency_ms: Some(p.latency_ms),
                        last_seen_ts: Some(crate::types::now_ts()), last_error: None, start_cmd: None,
                    });
                    drop(servers);
                    add_log(state, "info", &format!("[scan] serveur MCP découvert sur port {} : {} v{}",
                        port, p.name.as_deref().unwrap_or("?"), p.version.as_deref().unwrap_or("?")));
                }
            }
    }
    add_log(state, "info", &format!("scan terminé : {} nouveaux serveurs MCP", found));
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
                        if let Some(s) = servers.iter_mut().find(|s| s.id == id) { s.auto_restart = true; }
                    }
                    poll_one(state, &id);
                    json!({"ok": true})
                }
                "kill_stdio" => {
                    {
                        let mut servers = lockv(&state.servers);
                        if let Some(s) = servers.iter_mut().find(|s| s.id == id) { s.auto_restart = false; }
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
                        port,
                        path: req.get("path").and_then(|x| x.as_str()).unwrap_or("/mcp").to_string(),
                    },
                    origin: Origin::Manual, enabled: true, state: SState::Down,
                    pid: None, bound_host: None, server_name: None, version: None,
                    protocol: None, tools: vec![], latency_ms: None,
                    last_seen_ts: None, last_error: None, start_cmd: None, auto_restart: true,
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
                (Some(a), Some(b)) if a >= 1 && b <= 65535 && a <= b => Some((a as u16, b as u16)),
                _ => None,
            };
            scan_ports(state, range);
            json!({"ok": true})
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
        Transport::Http { port, path } => ("http", json!({"port": port, "path": path})),
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
        "tools_count": s.tools.len(), "latency_ms": s.latency_ms,
        "last_seen": s.last_seen_ts, "last_error": s.last_error,
        "can_start": s.start_cmd.is_some(), "auto_restart": s.auto_restart,
    })
}

pub fn state_json(state: &Arc<AppState>) -> Value {
    let servers = lockv(&state.servers);
    let log = lockv(&state.log);
    let settings = lockv(&state.settings);
    let list: Vec<Value> = servers.iter().map(server_json).collect();
    let up = servers.iter().filter(|s| s.state == SState::Up).count();
    let total = servers.len();
    let tools: usize = servers.iter().map(|s| s.tools.len()).sum();
    let logarr: Vec<Value> = log.iter().rev().take(80).map(|e| json!({
        "ts": e.ts, "level": e.level, "msg": e.msg
    })).collect();
    json!({
        "servers": list, "log": logarr,
        "stats": {"up": up, "total": total, "tools": tools},
        "settings": {"refresh_secs": settings.refresh_secs, "ui_port": settings.ui_port,
                     "scan_from": settings.scan_from, "scan_to": settings.scan_to},
    })
}

pub fn detail_json(state: &Arc<AppState>, id: &str) -> Value {
    let servers = lockv(&state.servers);
    match servers.iter().find(|s| s.id == id) {
        Some(s) => {
            let mut o = match server_json(s) { Value::Object(o) => o, _ => Map::new() };
            let tools: Vec<Value> = s.tools.iter().map(|t| json!({"name": t.name, "desc": t.desc})).collect();
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
            Transport::Http { port, path } => { assert_eq!(*port, 4567); assert_eq!(path, "/x"); }
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
    fn state_json_compte_bon_les_stats() {
        let state = AppState {
            servers: Mutex::new(vec![
                Server { id: "a".into(), label: "A".into(), transport: Transport::Http { port: 1, path: "/mcp".into() },
                    origin: Origin::Seed, enabled: true, auto_restart: true, state: SState::Up,
                    pid: None, bound_host: None, server_name: None, version: None, protocol: None,
                    tools: vec![crate::types::ToolInfo { name: "t".into(), desc: String::new() }],
                    latency_ms: None, last_seen_ts: None, last_error: None, start_cmd: None },
                Server { id: "b".into(), label: "B".into(), transport: Transport::Http { port: 2, path: "/mcp".into() },
                    origin: Origin::Seed, enabled: false, auto_restart: true, state: SState::Disabled,
                    pid: None, bound_host: None, server_name: None, version: None, protocol: None,
                    tools: vec![], latency_ms: None, last_seen_ts: None, last_error: None, start_cmd: None },
            ]),
            log: Mutex::new(VecDeque::new()),
            settings: Mutex::new(Settings::default()),
            children: Mutex::new(std::collections::HashMap::new()),
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
