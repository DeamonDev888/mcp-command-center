// Import « collez n'importe quoi » : parsing des configs mcpServers/mcp_servers (porté de mcp-doc.ts).
// (split v0.7.20 : manage.rs reste la racine — API stable via réexport)
use super::*;

pub fn parse_servers_doc(raw: &str) -> Result<Vec<(String, Value)>, String> {
    let parsed: Value = serde_json::from_str(raw).map_err(|e| format!("JSON invalide: {}", e))?;
    let obj = parsed.as_object().ok_or("attendu un objet JSON")?;
    let map = obj
        .get("mcpServers")
        .or_else(|| obj.get("mcp_servers"))
        .and_then(|w| w.as_object())
        .unwrap_or(obj); // map brute
    Ok(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
}

pub(super) fn entry_to_server(id: String, name: &str, entry: &Value) -> Option<Server> {
    let command = entry.get("command").and_then(|x| x.as_str());
    let url = entry.get("url").and_then(|x| x.as_str());
    let args: Vec<String> = entry
        .get("args")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|i| i.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if let Some(cmd) = command {
        let workdir = entry.get("cwd").and_then(|x| x.as_str()).map(String::from);
        return Some(Server {
            start_cmd: Some(StartCmd {
                program: cmd.to_string(),
                args: args.clone(),
                workdir: workdir.clone(),
            }),
            id,
            label: name.to_string(),
            transport: Transport::Stdio {
                program: cmd.to_string(),
                args,
                workdir,
            },
            origin: Origin::Manual,
            enabled: true,
            state: SState::Down,
            pid: None,
            bound_host: None,
            server_name: None,
            version: None,
            protocol: None,
            sse: false,
            sse_endpoint: false,
            ws: false,
            lang: None,
            framework: None,
            via_ssh: false,
            tools: vec![],
            latency_ms: None,
            last_seen_ts: None,
            last_error: None,
            auto_restart: true,
            favorite: false,
            tools_disabled: vec![],
        });
    }
    if let Some(u) = url {
        let (host, port, path) = parse_url_parts(u)?;
        return Some(Server {
            start_cmd: None,
            id,
            label: name.to_string(),
            transport: Transport::Http {
                host: Some(host),
                port,
                path,
            },
            origin: Origin::Manual,
            enabled: true,
            state: SState::Down,
            pid: None,
            bound_host: None,
            server_name: None,
            version: None,
            protocol: None,
            sse: false,
            sse_endpoint: false,
            ws: false,
            lang: None,
            framework: None,
            via_ssh: false,
            tools: vec![],
            latency_ms: None,
            last_seen_ts: None,
            last_error: None,
            auto_restart: true,
            favorite: false,
            tools_disabled: vec![],
        });
    }
    None
}

pub(super) fn parse_url_parts(u: &str) -> Option<(String, u16, String)> {
    // TLS non supporté par ce client (std sans rustls) : une URL https finirait en
    // sonde plaintext illisible et en DOWN permanent — on refuse proprement.
    // (import_mcp_text renvoie un message clair pour https://)
    let rest = u.strip_prefix("http://")?;
    let (hostport, path) = match rest.find('/') {
        Some(i) => (&rest[..i], rest[i..].to_string()),
        None => (rest, "/mcp".to_string()),
    };
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) => (h.to_string(), p.parse().ok()?),
        _ => (
            hostport.to_string(),
            if u.starts_with("https") { 443 } else { 80 },
        ),
    };
    Some((host, port, path))
}

/// Porté de mcp-import.ts : collez n'importe quoi — snippet mcp.json, commande
/// npx/uvx/docker, ligne `claude mcp add`, ou simple URL → serveurs ajoutés.
pub fn import_mcp_text(state: &Arc<AppState>, raw: &str) -> Value {
    let text = raw.trim();
    if text.is_empty() {
        return json!({"ok": false, "error": "texte vide"});
    }

    // 1. JSON (mcp.json)
    if text.starts_with('{') {
        match parse_servers_doc(text) {
            Ok(entries) if !entries.is_empty() => {
                let mut added = 0usize;
                for (name, entry) in entries {
                    let id = format!("store-{}", name);
                    let mut servers = lockv(&state.servers);
                    if servers.iter().any(|s| s.id == id) {
                        continue;
                    }
                    if let Some(srv) = entry_to_server(id, &name, &entry) {
                        let label = srv.label.clone();
                        servers.push(srv);
                        drop(servers);
                        add_log(
                            state,
                            "info",
                            &format!("[boutique] import JSON : {}", label),
                        );
                        added += 1;
                    }
                }
                if added > 0 {
                    save_config(state);
                }
                return json!({"ok": true, "added": added, "source": "json"});
            }
            Ok(_) => { /* json vide → continuer sur les autres pistes */ }
            Err(e) => return json!({"ok": false, "error": e}),
        }
    }

    // 2. `claude mcp add <name> <cmd> [args...]` — 5 tokens minimum (nom + programme),
    //    sinon un « claude mcp add » tronqué paniquait sur toks[3]/toks[4] (audit 2026-10-24)
    let toks: Vec<&str> = text.split_whitespace().collect();
    if toks.len() >= 5 && toks[0] == "claude" && toks[1] == "mcp" && toks[2] == "add" {
        let name = toks[3].to_string();
        let program = toks[4].to_string();
        let args: Vec<String> = toks[5..].iter().map(|s| s.to_string()).collect();
        return push_imported(state, name, program, args);
    }

    // 3. URL directe → serveur HTTP (https refusé : pas de TLS dans ce client std)
    if text.starts_with("https://") {
        return json!({"ok": false, "error": "TLS non supporté : MCP Control ne sonde que du http:// (passez par un tunnel http local pour un endpoint https)"});
    }
    if text.starts_with("http://") {
        if let Some((host, port, path)) = parse_url_parts(text) {
            let name = name_from_url(&host);
            let id = format!("store-{}", name);
            let mut servers = lockv(&state.servers);
            if !servers.iter().any(|s| s.id == id) {
                servers.push(Server {
                    start_cmd: None,
                    id: id.clone(),
                    label: name.clone(),
                    transport: Transport::Http {
                        host: Some(host),
                        port,
                        path,
                    },
                    origin: Origin::Manual,
                    enabled: true,
                    state: SState::Down,
                    pid: None,
                    bound_host: None,
                    server_name: None,
                    version: None,
                    protocol: None,
                    sse: false,
                    sse_endpoint: false,
                    ws: false,
                    lang: None,
                    framework: None,
                    via_ssh: false,
                    tools: vec![],
                    latency_ms: None,
                    last_seen_ts: None,
                    last_error: None,
                    auto_restart: true,
                    favorite: false,
                    tools_disabled: vec![],
                });
                drop(servers);
                save_config(state);
                add_log(state, "info", &format!("[boutique] import URL : {}", name));
                return json!({"ok": true, "added": 1, "source": "url"});
            }
            return json!({"ok": true, "added": 0, "source": "url", "note": "déjà présent"});
        }
    }

    // 4. Commande npx/uvx/docker → serveur stdio
    if let Some(first) = toks.first() {
        if ["npx", "uvx", "docker", "node", "python", "python3"].contains(first) {
            // premier token qui n'est pas une option (-y, --yes…) = le package
            let spec = toks
                .iter()
                .skip(1)
                .find(|t| !t.starts_with('-'))
                .copied()
                .unwrap_or("");
            let name = if spec.is_empty() {
                "serveur-importe".to_string()
            } else {
                cleanup_name(spec)
            };
            let name = format!("{}-{}", name, now_compact());
            let program = first.to_string();
            let args: Vec<String> = toks[1..].iter().map(|s| s.to_string()).collect();
            return push_imported(state, name, program, args);
        }
    }

    json!({"ok": false, "error": "rien de reconnu : colle un mcp.json, une commande npx/uvx, une URL, ou `claude mcp add ...`"})
}

pub(super) fn push_imported(
    state: &Arc<AppState>,
    name: String,
    program: String,
    args: Vec<String>,
) -> Value {
    let id = format!("store-{}", name);
    let mut servers = lockv(&state.servers);
    if servers.iter().any(|s| s.id == id) {
        return json!({"ok": true, "added": 0, "note": "déjà présent"});
    }
    servers.push(Server {
        start_cmd: Some(StartCmd {
            program: program.clone(),
            args: args.clone(),
            workdir: None,
        }),
        id: id.clone(),
        label: name.clone(),
        transport: Transport::Stdio {
            program,
            args,
            workdir: None,
        },
        origin: Origin::Manual,
        enabled: true,
        state: SState::Down,
        pid: None,
        bound_host: None,
        server_name: None,
        version: None,
        protocol: None,
        sse: false,
        sse_endpoint: false,
        ws: false,
        lang: None,
        framework: None,
        via_ssh: false,
        tools: vec![],
        latency_ms: None,
        last_seen_ts: None,
        last_error: None,
        auto_restart: true,
        favorite: false,
        tools_disabled: vec![],
    });
    drop(servers);
    save_config(state);
    add_log(
        state,
        "info",
        &format!("[boutique] import commande : {}", name),
    );
    json!({"ok": true, "added": 1, "source": "commande"})
}

/// Porté de mcp-import.ts nameFromUrl : "mcp.linear.app" → "linear"
/// (dégage api/mcp/www, garde la marque).
pub(super) fn name_from_url(host: &str) -> String {
    let labels: Vec<&str> = host.split('.').filter(|l| !l.is_empty()).collect();
    let mut it: Vec<&str> = labels.to_vec();
    while it.len() > 1 && ["api", "mcp", "www"].contains(&it[0]) {
        it.remove(0);
    }
    it.first()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "server".to_string())
}
