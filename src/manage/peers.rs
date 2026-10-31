// Pairs MCP Control du LAN et machines distantes (SSH).
// (split v0.7.20 : manage.rs reste la racine — API stable via réexport)
use super::*;

pub fn discover_peers(state: &Arc<AppState>) {
    if !lockv(&state.settings).network_enabled {
        return;
    }
    let ports = lockv(&state.settings).network_ports.clone();
    let own_port = lockv(&state.settings).ui_port;
    let prefixes = network_prefixes();
    let my_ips: Vec<String> = crate::manage::local_ips();
    let mut hosts: Vec<String> = vec!["127.0.0.1".to_string()]; // autres instances locales
    for (prefix, self_ip) in &prefixes {
        for h in probe::arp_hosts(prefix, self_ip) {
            if !hosts.contains(&h) {
                hosts.push(h);
            }
        }
    }
    // une cible qui est une de NOS ips sur NOTRE port UI = nous-mêmes -> skip
    hosts.retain(|h| !(my_ips.contains(h)));
    add_log(
        state,
        "info",
        &format!(
            "pairs : sondage {} cibles (ARP + loopback) sur ports UI {:?}",
            hosts.len(),
            ports
        ),
    );

    use std::sync::atomic::{AtomicU32, Ordering};
    let new_peers = std::sync::Arc::new(AtomicU32::new(0));
    // secret partagé : la sonde de pairs présente NOTRE code d'accès (le pair doit
    // avoir la même valeur dans son config.json pour accepter /api/state)
    let access = lockv(&state.settings).api_access.clone();
    let mut handles = Vec::new();
    for ip in hosts {
        let ports = ports.clone();
        let state = state.clone();
        let new_peers = new_peers.clone();
        let access = access.clone();
        handles.push(std::thread::spawn(move || {
            for port in &ports {
                if *port == own_port {
                    continue;
                } // ne jamais se sonder soi-même sur notre port
                let pr = probe::probe_peer(&ip, *port, access.as_deref());
                if pr.alive {
                    if let Some(servers) = pr.servers {
                        let count = servers
                            .get("stats")
                            .and_then(|s| s.get("total"))
                            .and_then(|x| x.as_u64())
                            .unwrap_or(0);
                        let mut peers = lockv(&state.peers);
                        let is_new = !peers.contains_key(&ip);
                        peers.insert(
                            ip.clone(),
                            crate::types::Peer {
                                ip: ip.clone(),
                                ui_port: *port,
                                last_seen: crate::types::now_ts(),
                                servers,
                            },
                        );
                        drop(peers);
                        if is_new {
                            new_peers.fetch_add(1, Ordering::Relaxed);
                        }
                        add_log(
                            &state,
                            "info",
                            &format!(
                                "[pairs] MCP Control détecté sur {}:{} ({} serveurs)",
                                ip, port, count
                            ),
                        );
                        break;
                    }
                }
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    let total = lockv(&state.peers).len();
    let nouveaux = new_peers.load(Ordering::Relaxed);
    add_log(
        state,
        "info",
        &format!(
            "pairs : {} peer(s) connu(s) ({} nouveau(x))",
            total, nouveaux
        ),
    );
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
            if s.get("transport").and_then(|x| x.as_str()) != Some("http") {
                continue;
            }
            if s.get("enabled") != Some(&json!(true)) {
                continue;
            }
            let (host, port, path, name, version) = (
                s.get("transport_info")
                    .and_then(|t| t.get("host"))
                    .and_then(|x| x.as_str())
                    .map(String::from),
                s.get("transport_info")
                    .and_then(|t| t.get("port"))
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0) as u16,
                s.get("transport_info")
                    .and_then(|t| t.get("path"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("/mcp")
                    .to_string(),
                s.get("server_name")
                    .and_then(|x| x.as_str())
                    .map(String::from),
                s.get("version").and_then(|x| x.as_str()).map(String::from),
            );
            // host: null (localhost du pair) -> réécrire vers l'IP du pair si ce n'est pas notre machine
            let host = match host {
                Some(h) => h,
                None if ip == "127.0.0.1" => continue, // même machine : localhost reste localhost
                None => ip.to_string(),                // pair distant : pointer son IP
            };
            let mut servers = lockv(&state.servers);
            let already = servers.iter().any(|srv| {
                matches!(&srv.transport,
                Transport::Http { host: Some(h), port: p, .. } if h == &host && p == &port)
            });
            if already {
                continue;
            }
            let id = format!("net-{}.{}", host, port);
            if servers.iter().any(|s| s.id == id) {
                continue;
            }
            servers.push(Server {
                id,
                label: name.clone().unwrap_or_else(|| format!("{}:{}", host, port)),
                transport: Transport::Http {
                    host: Some(host.clone()),
                    port,
                    path,
                },
                origin: Origin::Discovered,
                enabled: true,
                state: SState::Down,
                auto_restart: true,
                favorite: false,
                pid: None,
                bound_host: None,
                server_name: name,
                version,
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
                start_cmd: None,
                tools_disabled: vec![],
            });
            added += 1;
        }
    }
    drop(snapshot);
    save_config(state);
    add_log(
        state,
        "info",
        &format!(
            "[pairs] import depuis {} (UI {}) : {} serveur(s) ajouté(s)",
            ip, ui_port, added
        ),
    );
    json!({"ok": true, "added": added})
}

// ---------- découverte SSH d'une machine distante ----------

/// Connexion SSH à une machine (client ssh système, auth par clé), netstat
/// distant, puis sonde des ports DEPUIS LA MACHINE (curl interne sur
/// 127.0.0.1 via la connexion ssh — passe les binds localhost et ufw) →
/// import des serveurs MCP trouvés en mode via_ssh (pollés par SSH ensuite).
pub fn ssh_discover(state: &Arc<AppState>, ip: &str, user: &str) -> Value {
    add_log(
        state,
        "info",
        &format!("[ssh] connexion à {user}@{ip} — vérification de la machine…"),
    );
    // Health-check AVANT le scan : une machine éteinte ou une clé absente
    // doit être dit immédiatement (au lieu d'attendre 30 s de scan futile).
    if let Err(e) = probe::ssh_health(ip, user, 8) {
        add_log(state, "warn", &e);
        return json!({"ok": false, "error": e});
    }
    let ports = match probe::ssh_netstat(ip, user, 8) {
        Ok(p) => p,
        Err(e) => {
            add_log(state, "warn", &e);
            return json!({"ok": false, "error": e});
        }
    };
    add_log(
        state,
        "info",
        &format!(
            "[ssh {ip}] {} port(s) en écoute (système écarté) — sonde MCP depuis la machine…",
            ports.len()
        ),
    );
    let found = match probe::ssh_probe_mcp_batch(ip, user, &ports, "/mcp", 4) {
        Ok(f) => f,
        Err(e) => {
            add_log(state, "warn", &e);
            return json!({"ok": false, "error": e});
        }
    };
    let mut serveurs: Vec<Value> = Vec::new();
    let mut imported = 0usize;
    for (port, v) in &found {
        let res = v.get("result").cloned().unwrap_or(json!({}));
        let si = res.get("serverInfo").cloned().unwrap_or(json!({}));
        let raw_name = si.get("name").and_then(|x| x.as_str());
        let name = raw_name
            .map(String::from)
            .unwrap_or_else(|| format!("{ip}:{port}"));
        let version = si.get("version").and_then(|x| x.as_str()).map(String::from);
        let protocol = res
            .get("protocolVersion")
            .and_then(|x| x.as_str())
            .map(String::from);
        let lang = probe::detect_lang_http(&probe::RespHead::default(), raw_name);
        let framework = crate::frameworks::detect(
            raw_name,
            protocol.as_deref(),
            false,
            false,
            false,
            lang.as_deref(),
        )
        .map(|f| f.id.to_string());
        let id = format!("net-{ip}.{port}");
        let mut servers = lockv(&state.servers);
        // DÉ-DUPLICATION SSH : un même serveur (même nom + même machine)
        // ne doit créer qu'UNE entrée. On cherche d'abord l'exact
        // (hôte+port), puis le même nom sur la même machine (le serveur a
        // changé de port) — sinon chaque import recrée un doublon.
        let existing_idx = {
            let mut hit: Option<usize> = None;
            for (i, sv) in servers.iter().enumerate() {
                // une entrée désactivée par l'utilisateur ne doit jamais être
                // détournée ni rafraîchie par la découverte SSH
                if !sv.enabled {
                    continue;
                }
                let same_port = matches!(&sv.transport,
                    Transport::Http { host: Some(h), port: sp, .. } if h == ip && sp == port);
                let same_name = raw_name.is_some()
                    && sv
                        .server_name
                        .as_deref()
                        .map(|n| n.eq_ignore_ascii_case(raw_name.unwrap_or("")))
                        .unwrap_or(false)
                    && matches!(&sv.transport, Transport::Http { host: Some(h), .. } if h == ip);
                if same_port || same_name {
                    hit = Some(i);
                    break;
                }
            }
            hit
        };
        if let Some(idx) = existing_idx {
            let existing = &mut servers[idx];
            // les entrées manuelles gardent leur mode de sonde : on ne force
            // pas via_ssh (audit 2026-10-26 — sinon elles échouent ensuite
            // « machine SSH non enregistrée » dès l'oubli de l'hôte)
            if existing.origin != Origin::Manual {
                existing.via_ssh = true;
            }
            // le port a pu changer : on suit le port réellement sondé
            if let Transport::Http {
                port: ref mut sp, ..
            } = existing.transport
            {
                if *sp != *port {
                    *sp = *port;
                }
            }
            existing.state = SState::Up;
            // surcharge CONDITIONNELLE (audit 2026-10-26) : une sonde validée
            // sans serverInfo/version ne doit pas effacer ce qu'on savait déjà
            if let Some(n) = raw_name {
                existing.server_name = Some(n.to_string());
                // le label fabriqué « ip:port » suit le nom annoncé ; un label
                // saisi par l'utilisateur est conservé
                if existing.label.trim().is_empty() || existing.label.contains(':') {
                    existing.label = n.to_string();
                }
            }
            if version.is_some() {
                existing.version = version.clone();
            }
            if protocol.is_some() {
                existing.protocol = protocol.clone();
            }
            if lang.is_some() {
                existing.lang = lang.clone();
            }
            if framework.is_some() {
                existing.framework = framework.clone();
            }
            existing.last_seen_ts = Some(crate::types::now_ts());
            serveurs.push(json!({"port": port, "name": name, "imported": false}));
            continue;
        }
        servers.push(Server {
            id: id.clone(),
            label: name.clone(),
            transport: Transport::Http {
                host: Some(ip.to_string()),
                port: *port,
                path: "/mcp".into(),
            },
            origin: Origin::Discovered,
            enabled: true,
            state: SState::Up,
            auto_restart: true,
            favorite: false,
            pid: None,
            bound_host: Some(ip.to_string()),
            server_name: raw_name.map(String::from),
            version: version.clone(),
            protocol,
            sse: false,
            sse_endpoint: false,
            ws: false,
            lang,
            framework,
            via_ssh: true,
            tools: vec![],
            latency_ms: None,
            last_seen_ts: Some(crate::types::now_ts()),
            last_error: None,
            start_cmd: None,
            tools_disabled: vec![],
        });
        drop(servers);
        imported += 1;
        serveurs.push(json!({"port": port, "name": name, "imported": true}));
        add_log(
            state,
            "info",
            &format!(
                "[ssh {ip}] serveur MCP importé (via ssh) : {}:{} — {} {}",
                ip,
                port,
                name,
                version.as_deref().unwrap_or("")
            ),
        );
    }
    // fusion des doublons (même nom + même machine) : la découverte SSH
    // pouvait créer plusieurs entrées pour un même serveur
    merge_duplicate_servers(state);
    if !serveurs.is_empty() {
        save_config(state);
    }
    add_log(
        state,
        "info",
        &format!(
            "[ssh {ip}] terminé : {} serveur(s) MCP trouvé(s), {} importé(s)",
            serveurs.len(),
            imported
        ),
    );
    json!({"ok": true, "ports": ports.len(), "found": imported, "serveurs": serveurs})
}
