// Dispatcher d'actions API (POST /api/action) + surface ACTIONS partagée avec
// les contrats de tests. Les familles pourront être extraites en sous-fonctions
// par thème (serveur, config, boutique, updates) — move pur ici.
use super::*;

pub const ACTIONS: &[&str] = &[
    "toggle",
    "refresh",
    "kill_stdio",
    "start_stdio",
    "start",
    "kill",
    "scan",
    "scan_network",
    "discover",
    "discover_peers",
    "import_peer",
    "add",
    "remove",
    "toggle_tool",
    "set_start_cmd",
    "set_host",
    "set_auto_restart",
    "set_favorite",
    "reorder",
    "import_mcp",
    "install_store",
    "ssh_save",
    "ssh_forget",
    "ssh_discover",
    "set_settings",
    "set_notify",
    "set_autostart",
    "reset_state",
    "reset_runtime",
    "check_update",
    "open_release_page",
    "download_update",
    "open_path",
    "test_notify",
];

/// Asset de la release correspondant à (type de binaire, OS) — table blanche
/// pure et testée. `None` = plateforme inconnue (jamais d'URL fabriquée hors
/// de cette table). Les assets n'embarquent pas la version : l'URL
/// `releases/latest/download/<asset>` sert toujours LA dernière release.

pub fn handle_action(state: &Arc<AppState>, body: &str) -> Value {
    let req: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return json!({"ok": false, "error": format!("body invalide: {}", e)}),
    };
    let action = req
        .get("action")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let id = req.get("id").and_then(|x| x.as_str()).map(String::from);

    match action.as_str() {
        "toggle" | "refresh" | "kill_stdio" | "start_stdio" | "start" | "kill" => {
            let id = match id {
                Some(i) => i,
                None => return json!({"ok": false, "error": "id manquant"}),
            };
            match action.as_str() {
                "toggle" => {
                    let now_enabled = {
                        let mut servers = lockv(&state.servers);
                        match servers.iter_mut().find(|s| s.id == id) {
                            Some(s) => {
                                s.enabled = !s.enabled;
                                s.state = if s.enabled {
                                    SState::Down
                                } else {
                                    SState::Disabled
                                };
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
                    if !now_enabled {
                        // OFF réel : tuer le process lancé par MCP Control (stdio ou HTTP)
                        let mut children = lockv(&state.children);
                        if let Some(h) = children.get_mut(&id) {
                            probe::kill_tree(h.child.id());
                            let _ = h.child.wait();
                        }
                        children.remove(&id);
                        drop(children);
                        let mut servers = lockv(&state.servers);
                        if let Some(s) = servers.iter_mut().find(|s| s.id == id) {
                            s.pid = None;
                        }
                    }
                    add_log(
                        state,
                        "info",
                        &format!(
                            "[{}] {}",
                            id,
                            if now_enabled {
                                "activé"
                            } else {
                                "désactivé"
                            }
                        ),
                    );
                    if now_enabled {
                        // ON réel : démarrer/sonder immédiatement si une commande de lancement existe
                        poll_one(state, &id);
                    }
                    json!({"ok": true, "enabled": now_enabled})
                }
                "refresh" => {
                    poll_one(state, &id);
                    json!({"ok": true})
                }
                "start_stdio" | "start" => {
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
                "kill_stdio" | "kill" => {
                    let local_port = {
                        let servers = lockv(&state.servers);
                        servers
                            .iter()
                            .find(|s| s.id == id)
                            .and_then(|s| match &s.transport {
                                Transport::Http {
                                    host: None, port, ..
                                } => Some(*port),
                                _ => None,
                            })
                    };
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
                        probe::kill_tree(h.child.id());
                        let _ = h.child.wait();
                    }
                    children.remove(&id);
                    drop(children);
                    // Le launcher détache le vrai serveur : child.kill() ne tue que le
                    // lanceur. On tue aussi le process qui écoute sur le port (local only).
                    if let Some(p) = local_port {
                        probe::kill_port(p);
                    }
                    // Le serveur réel (node/fastmcp) met 1-3 s à libérer le port :
                    // on re-polle jusqu'à DOWN (borné à 10 s) pour un feedback UI exact.
                    for _ in 0..20 {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        poll_one(state, &id);
                        let encore_up = {
                            let servers = lockv(&state.servers);
                            servers
                                .iter()
                                .find(|s| s.id == id)
                                .map(|s| s.state == SState::Up)
                                .unwrap_or(false)
                        };
                        if !encore_up {
                            break;
                        }
                    }
                    add_log(state, "info", &format!("[{}] stdio arrêté", id));
                    json!({"ok": true})
                }
                _ => unreachable!(),
            }
        }
        "add" => {
            let transport = req
                .get("transport")
                .and_then(|x| x.as_str())
                .unwrap_or("http");
            let srv = if transport == "stdio" {
                let program = req.get("program").and_then(|x| x.as_str()).unwrap_or("");
                if program.is_empty() {
                    return json!({"ok": false, "error": "program manquant"});
                }
                let sargs: Vec<String> = req
                    .get("args")
                    .and_then(|x| x.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|i| i.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let sworkdir = req
                    .get("workdir")
                    .and_then(|x| x.as_str())
                    .map(String::from);
                Server {
                    id: format!("manual-stdio-{}", now_compact()),
                    label: req
                        .get("label")
                        .and_then(|x| x.as_str())
                        .unwrap_or(program)
                        .to_string(),
                    transport: Transport::Stdio {
                        program: program.to_string(),
                        args: sargs.clone(),
                        workdir: sworkdir.clone(),
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
                    start_cmd: Some(StartCmd {
                        program: program.to_string(),
                        args: sargs,
                        workdir: sworkdir,
                    }),
                    auto_restart: true,
                    favorite: false,
                    tools_disabled: vec![],
                }
            } else {
                let port = match req.get("port").and_then(|x| x.as_u64()) {
                    Some(p) if (1..=65535).contains(&p) => p as u16,
                    _ => return json!({"ok": false, "error": "port invalide"}),
                };
                // commande de lancement optionnelle → MCP Control peut Start/Stop ce HTTP
                let sstart = {
                    let program = req
                        .get("program")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    if program.is_empty() {
                        None
                    } else {
                        Some(StartCmd {
                            program,
                            args: req
                                .get("args")
                                .and_then(|x| x.as_array())
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|i| i.as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default(),
                            workdir: req
                                .get("workdir")
                                .and_then(|x| x.as_str())
                                .map(String::from),
                        })
                    }
                };
                Server {
                    id: format!("manual-http-{}", now_compact()),
                    label: req
                        .get("label")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    transport: Transport::Http {
                        host: req.get("host").and_then(|x| x.as_str()).map(String::from),
                        port,
                        path: req
                            .get("path")
                            .and_then(|x| x.as_str())
                            .unwrap_or("/mcp")
                            .to_string(),
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
                    start_cmd: sstart,
                    auto_restart: true,
                    favorite: false,
                    tools_disabled: vec![],
                }
            };
            let label = srv.label.clone();
            let sid = srv.id.clone();
            lockv(&state.servers).push(srv);
            save_config(state);
            add_log(
                state,
                "info",
                &format!("[+] serveur ajouté : {} ({})", label, sid),
            );
            json!({"ok": true, "id": sid})
        }
        "remove" => {
            let id = match id {
                Some(i) => i,
                None => return json!({"ok": false, "error": "id manquant"}),
            };
            // durable = « Retirer durablement » sur un seed : l'id est consigné
            // dans settings.removed_seeds pour ne pas être réinjecté au boot
            let durable = req
                .get("durable")
                .and_then(|x| x.as_bool())
                .unwrap_or(false);
            let mut children = lockv(&state.children);
            if let Some(h) = children.get_mut(&id) {
                probe::kill_tree(h.child.id());
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
            if durable && removed {
                let mut st = lockv(&state.settings);
                if !st.removed_seeds.iter().any(|x| x == &id) {
                    st.removed_seeds.push(id.clone());
                }
            }
            if removed {
                lockv(&state.settings).server_order.retain(|x| x != &id);
            }
            save_config(state);
            add_log(
                state,
                "info",
                &format!(
                    "[-] serveur supprimé : {} ({}){}",
                    id,
                    if removed { "ok" } else { "introuvable" },
                    if durable && removed {
                        " — retiré durablement (non réinjecté au boot)"
                    } else {
                        ""
                    }
                ),
            );
            json!({"ok": removed})
        }
        "scan" => {
            let range = match (
                req.get("from").and_then(|x| x.as_u64()),
                req.get("to").and_then(|x| x.as_u64()),
            ) {
                (Some(a), Some(b))
                    if (1..=65535).contains(&a) && (1..=65535).contains(&b) && a <= b =>
                {
                    Some((a as u16, b as u16))
                }
                _ => None,
            };
            scan_ports(state, range);
            json!({"ok": true})
        }
        "scan_network" => {
            let range = match (
                req.get("from").and_then(|x| x.as_u64()),
                req.get("to").and_then(|x| x.as_u64()),
            ) {
                (Some(a), Some(b))
                    if (1..=65535).contains(&a) && (1..=65535).contains(&b) && a <= b =>
                {
                    Some((a as u16, b as u16))
                }
                _ => None,
            };
            let st = state.clone();
            std::thread::spawn(move || scan_network(&st, range));
            json!({"ok": true, "started": true, "note": "scan asynchrone — voir le journal pour les détections"})
        }
        "discover_peers" => {
            discover_peers(state);
            json!({"ok": true})
        }
        // découverte locale à la demande (bouton ⟳ de l'UI) : tous ports en écoute
        "discover" => {
            scan_listening(state);
            json!({"ok": true})
        }
        "ssh_save" => {
            let (ip, user) = match (
                req.get("ip").and_then(|x| x.as_str()),
                req.get("user").and_then(|x| x.as_str()),
            ) {
                (Some(i), Some(u)) if !i.trim().is_empty() && !u.trim().is_empty() => {
                    (i.trim().to_string(), u.trim().to_string())
                }
                _ => return json!({"ok": false, "error": "ip et user requis"}),
            };
            let mut st = lockv(&state.settings);
            if let Some(h) = st.ssh_hosts.iter_mut().find(|h| h.ip == ip) {
                h.user = user;
            } else {
                st.ssh_hosts.push(crate::types::SshHost { ip, user });
            }
            drop(st);
            save_config(state);
            add_log(state, "info", "[machines] machine enregistrée");
            json!({"ok": true})
        }
        "set_start_cmd" => {
            // Un serveur local sans commande de lancement (découvert par scan de
            // ports) n'a ni Start/Stop ni autoboot : l'utilisateur peut poser la
            // commande à la main ici. Seeds refusés (la config par défaut
            // réécraserait) et serveurs distants refusés (le process appartient
            // à sa machine hôte, pas au moteur local).
            let id = match id {
                Some(i) => i,
                None => return json!({"ok": false, "error": "id manquant"}),
            };
            let program = match req.get("program").and_then(|x| x.as_str()) {
                Some(p) if !p.trim().is_empty() => p.trim().to_string(),
                _ => {
                    return json!({"ok": false, "error": "program manquant (ex : node, python, chemin .exe)"});
                }
            };
            let args: Vec<String> = req
                .get("args")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|i| i.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let workdir = req
                .get("workdir")
                .and_then(|x| x.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let mut servers = lockv(&state.servers);
            let Some(srv) = servers.iter_mut().find(|s| s.id == id) else {
                return json!({"ok": false, "error": "serveur inconnu"});
            };
            if srv.origin == Origin::Seed {
                return json!({"ok": false, "error": "serveur seed : commande non modifiable (retire-le durablement puis réajoute-le)"});
            }
            if matches!(&srv.transport, Transport::Http { host: Some(_), .. }) {
                return json!({"ok": false, "error": "serveur distant : la commande de lancement appartient à sa machine hôte"});
            }
            srv.start_cmd = Some(StartCmd {
                program: program.clone(),
                args,
                workdir,
            });
            // Un serveur découvert (scan) n'est PAS persisté (redétecté au boot).
            // Donner une commande = l'utilisateur le revendique : promotion en
            // manual pour que commande et boutons survivent au redémarrage.
            srv.origin = Origin::Manual;
            drop(servers);
            save_config(state);
            add_log(
                state,
                "info",
                &format!("[set_start_cmd] {} : {}", id, program),
            );
            json!({"ok": true})
        }
        "set_host" => {
            // Glisser-déposer d'une ligne sur l'en-tête d'un groupe machine :
            // le host du transport devient la clé du groupe (re-host sans
            // réimporter). Un stdio n'a pas de host (toujours local) — refus.
            let id = match id {
                Some(i) => i,
                None => return json!({"ok": false, "error": "id manquant"}),
            };
            let host_arg = match req.get("host").and_then(|x| x.as_str()) {
                Some(h) if !h.trim().is_empty() => h.trim().to_string(),
                _ => {
                    return json!({"ok": false, "error": "host requis (IP ou hostname de la machine)"});
                }
            };
            let mut servers = lockv(&state.servers);
            let Some(srv) = servers.iter_mut().find(|s| s.id == id) else {
                return json!({"ok": false, "error": "serveur inconnu"});
            };
            if !matches!(srv.transport, Transport::Http { .. }) {
                return json!({"ok": false, "error": "set_host : serveurs stdio non concernés (toujours locaux)"});
            }
            let old = match &mut srv.transport {
                Transport::Http { host, .. } => host.take(),
                _ => unreachable!(),
            };
            if let Transport::Http { host, .. } = &mut srv.transport {
                *host = Some(host_arg.clone());
            }
            let is_seed = srv.origin == Origin::Seed;
            drop(servers);
            // Un serveur seed ne doit PAS être réécrit : au boot suivant la
            // config par défaut réimposerait son host. Refus + restauration.
            save_config(state);
            if is_seed {
                let mut servers = lockv(&state.servers);
                if let Some(srv) = servers.iter_mut().find(|s| s.id == id) {
                    if let Transport::Http { host, .. } = &mut srv.transport {
                        *host = old;
                    }
                }
                drop(servers);
                save_config(state);
                add_log(
                    state,
                    "warn",
                    &format!("[set_host] refusé sur seed : {}", id),
                );
                return json!({"ok": false, "error": "serveur seed : host non modifiable (retire-le durablement puis réajoute-le)"});
            }
            add_log(
                state,
                "info",
                &format!(
                    "[set_host] {} : {} -> {}",
                    id,
                    old.as_deref().unwrap_or("local"),
                    host_arg
                ),
            );
            json!({"ok": true, "old_host": old})
        }
        "ssh_forget" => {
            let ip = req
                .get("ip")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let removed = {
                let mut st = lockv(&state.settings);
                let before = st.ssh_hosts.len();
                st.ssh_hosts.retain(|h| h.ip != ip);
                before != st.ssh_hosts.len()
            };
            save_config(state);
            json!({"ok": removed})
        }
        "open_path" => {
            // « Ouvrir dans l'explorateur » (menu contextuel de l'explorateur).
            // Chemin local passé en ARGUMENT (jamais via un shell) ; un fichier
            // est sélectionné dans son dossier, un dossier est ouvert.
            let path = req
                .get("path")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if path.is_empty() || path.chars().any(|c| c.is_control()) {
                return json!({"ok": false, "error": "chemin invalide"});
            }
            let pb = std::path::PathBuf::from(&path);
            if !pb.exists() {
                return json!({"ok": false, "error": "chemin introuvable"});
            }
            #[cfg(windows)]
            let mut cmd = {
                let mut c = std::process::Command::new("explorer.exe");
                if pb.is_file() {
                    c.arg(format!("/select,{}", pb.display()));
                } else {
                    c.arg(&pb);
                }
                c
            };
            #[cfg(not(windows))]
            let mut cmd = {
                let mut c = std::process::Command::new(if cfg!(target_os = "macos") {
                    "open"
                } else {
                    "xdg-open"
                });
                if pb.is_file() {
                    c.arg(pb.parent().unwrap_or(&pb));
                } else {
                    c.arg(&pb);
                }
                c
            };
            probe::hidden(&mut cmd);
            match cmd.spawn() {
                Ok(_) => json!({"ok": true}),
                Err(e) => json!({"ok": false, "error": e.to_string()}),
            }
        }
        "reorder" => {
            // glisser-déposer de l'explorateur : nouvel ordre d'affichage et de
            // priorité de sondage. Ids inconnus filtrés, manquants ajoutés en fin.
            let Some(ids) = req.get("ids").and_then(|x| x.as_array()) else {
                return json!({"ok": false, "error": "ids requis"});
            };
            let mut new_order: Vec<String> = ids
                .iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect();
            let known: Vec<String> = {
                let servers = lockv(&state.servers);
                servers.iter().map(|s| s.id.clone()).collect()
            };
            new_order.retain(|id| known.contains(id));
            for id in &known {
                if !new_order.contains(id) {
                    new_order.push(id.clone());
                }
            }
            lockv(&state.settings).server_order = new_order.clone();
            save_config(state);
            add_log(
                state,
                "info",
                &format!(
                    "[ordre] {} serveur(s) réordonné(s) (préférence / priorité de sondage)",
                    new_order.len()
                ),
            );
            json!({"ok": true, "count": new_order.len()})
        }
        "set_settings" => {
            // Réglages principaux du client (dialog ⚙) : sondage, scan, réseau.
            // Validation stricte : whitelist du bind, clamps, ports 1-65535,
            // scan_from <= scan_to. ui_bind est figé au boot (listener) : un
            // changement s'applique au prochain démarrage.
            let mut st = lockv(&state.settings);
            let mut changed: Vec<&str> = Vec::new();
            if let Some(x) = req.get("refresh_secs").and_then(|x| x.as_u64()) {
                let v = x.clamp(5, 3600);
                if st.refresh_secs != v {
                    st.refresh_secs = v;
                    changed.push("refresh");
                }
            }
            if let (Some(f), Some(t)) = (
                req.get("scan_from").and_then(|x| x.as_u64()),
                req.get("scan_to").and_then(|x| x.as_u64()),
            ) {
                let f = f.clamp(1, 65535) as u16;
                let t = t.clamp(1, 65535) as u16;
                let (f, t) = (f.min(t), f.max(t));
                if st.scan_from != f || st.scan_to != t {
                    st.scan_from = f;
                    st.scan_to = t;
                    changed.push("scan");
                }
            }
            if let Some(b) = req.get("ui_bind").and_then(|x| x.as_str()) {
                let b = match b {
                    "0.0.0.0" => "0.0.0.0",
                    _ => "127.0.0.1",
                };
                if st.ui_bind != b {
                    st.ui_bind = b.to_string();
                    changed.push("ui_bind");
                }
            }
            if let Some(en) = req.get("network_enabled").and_then(|x| x.as_bool()) {
                if st.network_enabled != en {
                    st.network_enabled = en;
                    changed.push("network");
                }
            }
            if let Some(ports) = req.get("network_ports").and_then(|x| x.as_array()) {
                let v: Vec<u16> = ports
                    .iter()
                    .filter_map(|p| p.as_u64())
                    .filter(|p| (1..=65535).contains(p))
                    .map(|p| p as u16)
                    .collect();
                if !v.is_empty() && st.network_ports != v {
                    st.network_ports = v;
                    changed.push("network_ports");
                }
            }
            drop(st);
            save_config(state);
            add_log(
                state,
                "info",
                &format!(
                    "[réglages] {}",
                    if changed.is_empty() {
                        "aucun changement".to_string()
                    } else {
                        changed.join(", ")
                    }
                ),
            );
            json!({"ok": true, "changed": changed})
        }
        "set_notify" => {
            // Configuration des notifications (dialog Réglages) :
            // - desktop : toast Windows
            // - webhook : URL + en-têtes (une par ligne "K: V") + méthode
            //   - états notifiés (DOWN, PORT-ONLY par défaut)
            //   - délai minimum entre deux notifications du même serveur
            let mut c = lockv(&state.settings).notify.clone();
            if let Some(b) = req.get("desktop").and_then(|x| x.as_bool()) {
                c.desktop = b;
            }
            if let Some(u) = req.get("webhook_url").and_then(|x| x.as_str()) {
                let u = u.trim();
                if !u.is_empty() && !crate::notify::valid_webhook_url(u) {
                    return json!({"ok": false, "error": "URL de webhook invalide (http:// ou https:// attendu)"});
                }
                c.webhook_url = u.to_string();
            }
            if let Some(m) = req.get("method").and_then(|x| x.as_str()) {
                let m = m.trim().to_uppercase();
                if m == "POST" || m == "PUT" {
                    c.method = m;
                }
            }
            if let Some(h) = req.get("headers").and_then(|x| x.as_str()) {
                c.headers = h.to_string();
            }
            if let Some(arr) = req.get("states").and_then(|x| x.as_array()) {
                let v: Vec<String> = arr
                    .iter()
                    .filter_map(|x| x.as_str())
                    .map(|x| x.trim().to_uppercase())
                    .filter(|x| !x.is_empty())
                    .collect();
                if !v.is_empty() {
                    c.states = v;
                }
            }
            if let Some(i) = req.get("min_interval_secs").and_then(|x| x.as_u64()) {
                c.min_interval_secs = i.clamp(0, 86400);
            }
            lockv(&state.settings).notify = c.clone();
            save_config(state);
            add_log(state, "info", "[notif] configuration enregistrée");
            json!({"ok": true, "notify": crate::notify::to_json(&c)})
        }
        "test_notify" => {
            // Bouton « Tester » du panneau Notifications (dialog Réglages) :
            // émet une notification de test sur chaque canal ACTIVÉ et rend le
            // résultat par canal. Non exposé au gateway (déclenche un toast /
            // une requête réseau vers l'URL configurée — surface de confiance).
            let cfg = lockv(&state.settings).notify.clone();
            if !cfg.desktop && cfg.webhook_url.trim().is_empty() {
                return json!({
                    "ok": false,
                    "error": "aucun canal actif : active le toast desktop ou renseigne une URL de webhook"
                });
            }
            let desktop = if cfg.desktop {
                Some(
                    match crate::notify::desktop_toast(
                        "MCP Control",
                        "notification de test — le canal desktop fonctionne",
                    ) {
                        Ok(()) => json!({"ok": true}),
                        Err(e) => json!({"ok": false, "error": e}),
                    },
                )
            } else {
                None
            };
            let webhook = if !cfg.webhook_url.trim().is_empty() {
                Some(
                    match crate::notify::send_webhook(
                        &cfg,
                        "test",
                        "TEST",
                        "notification de test émise depuis Réglages → Notifications",
                    ) {
                        Ok(()) => json!({"ok": true}),
                        Err(e) => json!({"ok": false, "error": e}),
                    },
                )
            } else {
                None
            };
            add_log(state, "info", "[notif] test émis depuis les réglages");
            json!({"ok": true, "desktop": desktop, "webhook": webhook})
        }
        "reset_runtime" => {
            // Effacer l'historique runtime (crashs, redémarrages) et les
            // compteurs d'échec — utile quand un serveur a été réparé à la
            // main et que l'app doit repartir sur des bases propres.
            let mut rt = lockv(&state.runtime);
            let n = rt.len();
            rt.clear();
            drop(rt);
            lockv(&state.spawn_fails).clear();
            save_config(state);
            add_log(
                state,
                "info",
                &format!("[runtime] historique effac\u{e9} ({n} serveur(s))"),
            );
            json!({"ok": true, "cleared": n})
        }
        "check_update" => {
            // Re-vérification forcée de la release GitHub — le thread de fond
            // ne tourne qu'au boot puis chaque heure. Le bouton « Vérifier » du
            // dialog Réglages obtient un résultat immédiat ET honnête :
            // `checked:false` = le check a échoué (réseau) — l'UI ne doit PAS
            // afficher « à jour » dans ce cas (c'était le clic mort d'avant).
            let verdict = check_update(state);
            let latest = lockv(&state.update_latest).clone();
            let checked = !matches!(verdict, UpdateCheck::Unreachable);
            json!({
                "ok": true,
                "checked": checked,
                "current": env!("CARGO_PKG_VERSION"),
                "latest": latest,
                "update_available": latest.is_some(),
            })
        }
        "open_release_page" => {
            // Ouvre la page des releases dans le navigateur par défaut de la
            // machine qui fait tourner le moteur — FIX du clic mort : dans la
            // webview Tauri (desktop), `window.open` est neutralisé, donc le
            // badge « mise à jour » et le bouton des réglages passent par ici.
            // URL FIXE sans argument : aucune surface d'injection ; non exposé
            // au gateway (le serveur n'a pas à ouvrir des navigateurs).
            const RELEASES_URL: &str =
                "https://github.com/DeamonDev888/mcp-control/releases/latest";
            #[cfg(windows)]
            let mut cmd = {
                let mut c = std::process::Command::new("rundll32");
                c.args(["url.dll,FileProtocolHandler", RELEASES_URL]);
                c
            };
            #[cfg(target_os = "macos")]
            let mut cmd = {
                let mut c = std::process::Command::new("open");
                c.arg(RELEASES_URL);
                c
            };
            #[cfg(all(unix, not(target_os = "macos")))]
            let mut cmd = {
                let mut c = std::process::Command::new("xdg-open");
                c.arg(RELEASES_URL);
                c
            };
            let opened = {
                #[cfg(any(windows, target_os = "macos", all(unix, not(target_os = "macos"))))]
                {
                    probe::probe_hidden_cmd(&mut cmd);
                    cmd.spawn().is_ok()
                }
                #[cfg(not(any(
                    windows,
                    target_os = "macos",
                    all(unix, not(target_os = "macos"))
                )))]
                {
                    false
                }
            };
            if opened {
                json!({"ok": true, "url": RELEASES_URL})
            } else {
                json!({
                    "ok": false,
                    "url": RELEASES_URL,
                    "error": "pas de navigateur lançable sur cette machine — ouvre l'URL à la main",
                })
            }
        }
        "download_update" => {
            // Bouton « ⬆ Mettre à jour » : téléchargement DIRECT du BON asset
            // (installateur desktop ou CLI, pour le BON OS) au lieu de la page
            // des releases. Détection : l'OS vient de la requête quand l'UI
            // connaît celui du NAVIGATEUR (la machine où atterrit le fichier —
            // cas UI servie par un moteur distant), sinon l'OS du binaire qui
            // tourne ; le type (desktop/cli) suit le binaire courant. L'URL est
            // construite UNIQUEMENT depuis la table blanche `release_asset` —
            // jamais depuis la requête. Non exposé au gateway (ouvre un
            // téléchargement).
            let kind = match std::env::current_exe()
                .ok()
                .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_lowercase()))
            {
                Some(exe) if exe.contains("desktop") => "desktop",
                _ => "cli",
            };
            let os = match req.get("os").and_then(Value::as_str) {
                Some(o @ ("windows" | "macos" | "linux")) => o,
                // os fourni mais hors whitelist → refus net : JAMAIS un
                // téléchargement pour une plateforme différente de celle
                // demandée (le repli silencieux sur le moteur serait une
                // surprise) ; aucun effet de bord avant ce contrôle.
                Some(other) => {
                    return json!({
                        "ok": false,
                        "error": format!("os inconnu : {other} (windows, macos ou linux attendus)"),
                    });
                }
                None => engine_os(),
            };
            let Some(asset) = release_asset(kind, os) else {
                return json!({
                    "ok": false,
                    "error": format!("plateforme inconnue : {kind}/{os}"),
                });
            };
            let url = format!(
                "https://github.com/DeamonDev888/mcp-control/releases/latest/download/{asset}"
            );
            #[cfg(windows)]
            let mut cmd = {
                let mut c = std::process::Command::new("rundll32");
                c.args(["url.dll,FileProtocolHandler", &url]);
                c
            };
            #[cfg(target_os = "macos")]
            let mut cmd = {
                let mut c = std::process::Command::new("open");
                c.arg(&url);
                c
            };
            #[cfg(all(unix, not(target_os = "macos")))]
            let mut cmd = {
                let mut c = std::process::Command::new("xdg-open");
                c.arg(&url);
                c
            };
            let opened = {
                #[cfg(any(windows, target_os = "macos", all(unix, not(target_os = "macos"))))]
                {
                    probe::probe_hidden_cmd(&mut cmd);
                    cmd.spawn().is_ok()
                }
                #[cfg(not(any(
                    windows,
                    target_os = "macos",
                    all(unix, not(target_os = "macos"))
                )))]
                {
                    false
                }
            };
            add_log(
                state,
                "info",
                &format!("[maj] téléchargement {kind} pour {os} : {asset}"),
            );
            if opened {
                json!({"ok": true, "url": url, "asset": asset, "kind": kind, "os": os})
            } else {
                // l'URL reste rendue : l'UI (vrai navigateur) fait le repli
                // window.open elle-même
                json!({
                    "ok": false,
                    "url": url,
                    "asset": asset,
                    "kind": kind,
                    "os": os,
                    "error": "pas de navigateur lançable sur cette machine — ouvre l'URL à la main",
                })
            }
        }
        "reset_state" => {
            // Réinitialisation depuis le dialog Réglages :
            // - placement : ordre des serveurs (server_order vidé)
            // - detection : découverts + imports natifs litemcp retirés (ils
            //   reviendront à la prochaine annonce/sonde), pairs oubliés
            // - oublis   : seeds retirés durablement RÉINJECTÉS tout de suite
            //   (comme au boot) + machines SSH enregistrées oubliées
            let what = req.get("what").and_then(|x| x.as_str()).unwrap_or("");
            let mut removed_servers = 0usize;
            let mut reinjected = 0usize;
            let mut cleared: Vec<&str> = Vec::new();
            if what == "placement" || what == "tout" {
                lockv(&state.settings).server_order.clear();
                cleared.push("placement");
            }
            if what == "detection" || what == "tout" {
                {
                    let mut servers = lockv(&state.servers);
                    let before = servers.len();
                    servers.retain(|s| {
                        s.origin != Origin::Discovered && !s.id.starts_with("litemcp-")
                    });
                    removed_servers += before - servers.len();
                }
                lockv(&state.peers).clear();
                cleared.push("detection");
            }
            if what == "oublis" || what == "tout" {
                let removed = {
                    let mut st = lockv(&state.settings);
                    st.ssh_hosts.clear();
                    let removed = st.removed_seeds.clone();
                    st.removed_seeds.clear();
                    removed
                };
                // réinjection immédiate (même logique que le boot) : les
                // seeds retirés durablement redeviennent visibles sans
                // attendre un redémarrage
                {
                    let mut servers = lockv(&state.servers);
                    for s in default_servers() {
                        if removed.contains(&s.id) && !servers.iter().any(|x| x.id == s.id) {
                            servers.push(s);
                            reinjected += 1;
                        }
                    }
                }
                cleared.push("oublis");
            }
            if cleared.is_empty() {
                return json!({"ok": false, "error": "what inconnu (placement | detection | oublis | tout)"});
            }
            save_config(state);
            add_log(
                state,
                "info",
                &format!(
                    "[reset] {} — {} serveur(s) retiré(s), {} seed(s) réinjecté(s)",
                    cleared.join("+"),
                    removed_servers,
                    reinjected
                ),
            );
            json!({
                "ok": true,
                "cleared": cleared,
                "removed": removed_servers,
                "reinjected": reinjected,
            })
        }
        "set_autostart" => {
            let enable = req.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true);
            lockv(&state.settings).autostart = enable;
            apply_autostart(state, enable);
            save_config(state);
            json!({"ok": true, "autostart": enable, "active": autostart_active()})
        }
        // Autoboot PAR SERVEUR : « ce serveur est relancé automatiquement au
        // démarrage de MCP Control (et après un crash) ». Persisté dans
        // config.json, indépendant de l'autostart OS de l'application.
        "set_auto_restart" => {
            let Some(id) = id else {
                return json!({"ok": false, "error": "id manquant"});
            };
            let enable = req.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true);
            let (label, found) = {
                let mut servers = lockv(&state.servers);
                match servers.iter_mut().find(|s| s.id == id) {
                    Some(srv) => {
                        srv.auto_restart = enable;
                        // réactiver l'autoboot après des échecs doit repartir de zéro
                        if enable {
                            lockv(&state.spawn_fails).remove(&id);
                        }
                        (srv.label.clone(), true)
                    }
                    None => (String::new(), false),
                }
            };
            if !found {
                return json!({"ok": false, "error": "serveur inconnu"});
            }
            save_config(state);
            add_log(
                state,
                "info",
                &format!(
                    "[{}] autoboot {}",
                    label,
                    if enable {
                        "activé (relance au démarrage et après crash)"
                    } else {
                        "désactivé (ne sera pas relancé automatiquement)"
                    }
                ),
            );
            json!({"ok": true, "id": id, "auto_restart": enable})
        }
        "set_favorite" => {
            // Favori (⭐) par serveur : la ligne est épinglée en haut de sa
            // section dans l'explorateur, quel que soit le tri. Persisté dans
            // config.json, indépendant de l'autoboot et de l'ordre posé au
            // glisser-déposer (l'épingle s'applique À L'AFFICHAGE).
            let Some(id) = id else {
                return json!({"ok": false, "error": "id manquant"});
            };
            let enable = req.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true);
            let (label, found) = {
                let mut servers = lockv(&state.servers);
                match servers.iter_mut().find(|s| s.id == id) {
                    Some(srv) => {
                        srv.favorite = enable;
                        (srv.label.clone(), true)
                    }
                    None => (String::new(), false),
                }
            };
            if !found {
                return json!({"ok": false, "error": "serveur inconnu"});
            }
            save_config(state);
            add_log(
                state,
                "info",
                &format!(
                    "[{}] favori {}",
                    label,
                    if enable { "épinglé" } else { "retiré" }
                ),
            );
            json!({"ok": true, "id": id, "favorite": enable})
        }
        "ssh_discover" => {
            let ip = req
                .get("ip")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            // user : fourni par la requête, sinon la machine enregistrée
            let user = req
                .get("user")
                .and_then(|x| x.as_str())
                .map(|u| u.trim().to_string())
                .filter(|u| !u.is_empty())
                .or_else(|| {
                    lockv(&state.settings)
                        .ssh_hosts
                        .iter()
                        .find(|h| h.ip == ip)
                        .map(|h| h.user.clone())
                });
            let Some(user) = user else {
                return json!({"ok": false, "error": "utilisateur SSH requis (ou enregistre la machine)"});
            };
            if ip.is_empty() {
                return json!({"ok": false, "error": "ip requise"});
            }
            // l'action peut prendre ~30 s (netstat distant + sonde des ports) :
            // on la détache pour ne pas bloquer le serveur HTTP, l'UI suit via
            // le journal et un rechargement d'état.
            let st = state.clone();
            let ip2 = ip.clone();
            std::thread::spawn(move || {
                ssh_discover(&st, &ip2, &user);
            });
            json!({"ok": true, "started": true, "note": "découverte SSH lancée — vois le journal, l'état se recharge"})
        }
        "import_peer" => {
            let ip = req.get("ip").and_then(|x| x.as_str()).unwrap_or("");
            if ip.is_empty() {
                return json!({"ok": false, "error": "ip manquante"});
            }
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
            add_log(
                state,
                "info",
                &format!(
                    "[{}] tool '{}' {} ({} actifs)",
                    sid,
                    tool,
                    if now_disabled {
                        "DÉSACTIVÉ"
                    } else {
                        "réactivé"
                    },
                    count
                ),
            );
            json!({"ok": true, "disabled": now_disabled, "tools_actifs": count})
        }
        "install_store" => {
            let sid = req.get("id").and_then(|x| x.as_str()).unwrap_or("");
            match find_store_item(sid) {
                Some(it) => {
                    // args surchargés depuis la requête (ex. dossier du serveur Filesystem)
                    let mut args: Vec<String> = match req.get("args").and_then(|x| x.as_array()) {
                        Some(a) if !a.is_empty() => a
                            .iter()
                            .filter_map(|i| i.as_str().map(String::from))
                            .collect(),
                        _ => it.args.iter().map(|a| a.to_string()).collect(),
                    };
                    if args.is_empty() {
                        args = it.args.iter().map(|a| a.to_string()).collect();
                    }
                    // Filesystem : le dossier exposé doit être choisi EXPLICITEMENT —
                    // jamais un chemin personnel par défaut dans un binaire distribué.
                    if it.id == "filesystem" && filesystem_dossier_manquant(&args) {
                        return json!({"ok": false, "error": "précisez le dossier à exposer : {\"action\":\"install_store\",\"id\":\"filesystem\",\"args\":[\"-y\",\"@modelcontextprotocol/server-filesystem\",\"C:/chemin/autorisé\"]}"});
                    }
                    let id = format!("store-{}", it.id);
                    let already = {
                        let mut servers = lockv(&state.servers);
                        let already = servers.iter().any(|s| s.id == id);
                        if !already {
                            servers.push(Server {
                                id: id.clone(),
                                label: it.name.to_string(),
                                transport: Transport::Stdio {
                                    program: it.program.to_string(),
                                    args: args.clone(),
                                    workdir: None,
                                },
                                origin: Origin::Manual,
                                enabled: true,
                                state: SState::Down,
                                auto_restart: true,
                                favorite: false,
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
                                start_cmd: Some(StartCmd {
                                    program: it.program.to_string(),
                                    args,
                                    workdir: None,
                                }),
                                tools_disabled: vec![],
                            });
                        }
                        already
                    };
                    if already {
                        return json!({"ok": true, "added": 0, "note": "déjà installé"});
                    }
                    save_config(state);
                    add_log(state, "info", &format!("[boutique] installé : {}", it.name));
                    poll_one(state, &id);
                    let st = {
                        let servers = lockv(&state.servers);
                        servers
                            .iter()
                            .find(|s| s.id == id)
                            .map(|s| s.state.as_str().to_string())
                            .unwrap_or_default()
                    };
                    json!({"ok": true, "added": 1, "id": id, "state": st})
                }
                None => json!({"ok": false, "error": "item inconnu"}),
            }
        }
        "import_mcp" => {
            let raw = req.get("text").and_then(|x| x.as_str()).unwrap_or("");
            import_mcp_text(state, raw)
        }
        _ => json!({"ok": false, "error": format!("action inconnue: {}", action)}),
    }
}
