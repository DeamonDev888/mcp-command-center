// Découverte : scan de ports, auto-scan (netstat), scan réseau, dédoublonnage.
// (split v0.7.20 : manage.rs reste la racine — API stable via réexport)
use super::*;

#[allow(unused_must_use)]
pub fn poll_one(state: &Arc<AppState>, id: &str) {
    let (transport, enabled) = {
        let servers = lockv(&state.servers);
        match servers.iter().find(|s| s.id == id) {
            Some(s) => (s.transport.clone(), s.enabled),
            None => return,
        }
    };
    if !enabled {
        return;
    }
    match transport {
        Transport::Http { host, port, path } => {
            // serveurs via_ssh : la sonde tourne DEPUIS la machine (curl interne
            // sur 127.0.0.1 via la connexion ssh) — l'extérieur est souvent
            // bloqué (bind localhost, ufw)
            let via_ssh = lockv(&state.servers)
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.via_ssh)
                .unwrap_or(false);
            let mut res = if via_ssh {
                match &host {
                    Some(h) => {
                        let user = lockv(&state.settings)
                            .ssh_hosts
                            .iter()
                            .find(|x| x.ip == *h)
                            .map(|x| x.user.clone());
                        match user {
                            Some(u) => probe::ssh_probe_server(h, &u, port, &path, 6),
                            None => probe::McpProbe::fail(
                                "machine SSH non enregistrée — réenregistre-la dans Machines"
                                    .to_string(),
                                true,
                            ),
                        }
                    }
                    None => probe::McpProbe::fail("serveur via SSH sans hôte".to_string(), true),
                }
            } else {
                match &host {
                    Some(h) => probe::probe_http_host(h, port, &path, 6000),
                    None => probe::probe_http(port, &path, 6000),
                }
            };
            // Auto-start : serveur HTTP LOCAL éteint + commande de lancement configurée
            // (jamais pour un serveur distant, host.is_none() gate)
            if !res.mcp_up && host.is_none() {
                let (can_auto, child_alive) = {
                    let servers = lockv(&state.servers);
                    let mut children = lockv(&state.children);
                    let alive = match children.get_mut(id) {
                        Some(h) => h.child.try_wait().ok().flatten().is_none(),
                        None => false,
                    };
                    let can = servers
                        .iter()
                        .find(|s| s.id == id)
                        .map(|s| s.start_cmd.is_some() && s.auto_restart)
                        .unwrap_or(false);
                    (can, alive)
                };
                // BACKOFF : après un échec, on attend (5 s, 10 s, 20 s…) avant la
                // prochaine tentative — et l'état est PERSISTÉ (le compteur
                // d'échecs survit au redémarrage de l'app).
                let now = crate::types::now_ts();
                let backoff_ok = {
                    let mut rt = lockv(&state.runtime);
                    let entry = rt.entry(id.to_string()).or_default();
                    entry.can_attempt(now)
                };
                if can_auto
                    && !child_alive
                    && backoff_ok
                    && lockv(&state.spawning).insert(id.to_string())
                {
                    // garde posée : le poller et une action API ne peuvent pas
                    // doubler le spawn du même serveur (course refresh vs poll)
                    let cmd = {
                        let servers = lockv(&state.servers);
                        servers
                            .iter()
                            .find(|s| s.id == id)
                            .and_then(|s| s.start_cmd.clone())
                    };
                    if let Some(cmd) = cmd {
                        match probe::spawn_process(&cmd.program, &cmd.args, cmd.workdir.as_deref())
                        {
                            Ok(h) => {
                                let pid = h.child.id();
                                lockv(&state.children).insert(id.to_string(), h);
                                lockv(&state.spawn_fails).remove(id);
                                {
                                    let mut servers = lockv(&state.servers);
                                    if let Some(srv) = servers.iter_mut().find(|s| s.id == id) {
                                        srv.pid = Some(pid);
                                    }
                                }
                                add_log(
                                    state,
                                    "info",
                                    &format!("[{}] http démarré (pid {}) — re-sonde", id, pid),
                                );
                                runtime_event(state, id, "start-ok", &format!("pid {}", pid));
                                // Re-probe borné : le launcher peut builder avant d'écouter
                                // (bin/launch.cjs fait `npm run build`), jusqu'à 30 s.
                                let mut res2 = probe::probe_http(port, &path, 2000);
                                let deadline =
                                    std::time::Instant::now() + std::time::Duration::from_secs(30);
                                while !res2.mcp_up && std::time::Instant::now() < deadline {
                                    std::thread::sleep(std::time::Duration::from_millis(1000));
                                    res2 = probe::probe_http(port, &path, 2000);
                                }
                                if res2.mcp_up && res2.pid.is_none() {
                                    res2.pid = Some(pid);
                                }
                                res = res2;
                            }
                            Err(e) => {
                                note_spawn_failure(state, id);
                                add_log(
                                    state,
                                    "warn",
                                    &format!("[{}] démarrage HTTP KO : {}", id, e),
                                )
                            }
                        }
                    }
                    // libère la garde anti-doublon quoi qu'il arrive
                    lockv(&state.spawning).remove(id);
                }
            }
            let mut log_event: Option<(&'static str, String)> = None;
            {
                let mut servers = lockv(&state.servers);
                if let Some(srv) = servers.iter_mut().find(|s| s.id == id) {
                    if res.mcp_up {
                        let changed = srv.server_name != res.name || srv.state != SState::Up;
                        srv.state = SState::Up;
                        srv.bound_host = res.host.clone();
                        srv.server_name = res.name.clone();
                        srv.version = res.version.clone();
                        srv.protocol = res.protocol.clone();
                        srv.sse = res.sse;
                        srv.sse_endpoint = res.sse_endpoint;
                        srv.ws = res.ws;
                        srv.lang = res.lang.clone();
                        srv.framework = res.framework.clone();
                        srv.tools = res.tools.clone();
                        srv.latency_ms = Some(res.latency_ms);
                        srv.last_seen_ts = Some(crate::types::now_ts());
                        srv.last_error = None;
                        if res.pid.is_some() {
                            srv.pid = res.pid;
                        }
                        if changed {
                            log_event = Some((
                                "info",
                                format!(
                                    "[{}] UP : {} v{} — {} tools ({} ms)",
                                    srv.label,
                                    res.name.as_deref().unwrap_or("?"),
                                    res.version.as_deref().unwrap_or("?"),
                                    tools_actifs(srv),
                                    res.latency_ms
                                ),
                            ));
                        }
                    } else {
                        let was_up = srv.state == SState::Up;
                        // port fermé = DOWN ; port ouvert sans MCP = PORT-ONLY (zombie)
                        srv.state = if res.port_open {
                            SState::PortOnly
                        } else {
                            SState::Down
                        };
                        srv.last_error = res.error.clone();
                        srv.latency_ms = None;
                        if !res.port_open && host.is_none() {
                            srv.pid = None;
                        }
                        if was_up {
                            let st_lbl = if res.port_open { "PORT-ONLY" } else { "DOWN" };
                            log_event = Some((
                                "warn",
                                format!(
                                    "[{}] {} : {}",
                                    srv.label,
                                    st_lbl,
                                    res.error.as_deref().unwrap_or("?")
                                ),
                            ));
                            // NOTIFICATION d'incident (desktop + webhook) :
                            // déclenchée une seule fois, à la transition.
                            let lbl = srv.label.clone();
                            let det = res.error.clone().unwrap_or_else(|| "?".to_string());
                            notify_incident(state, &lbl, st_lbl, &det);
                        }
                    }
                }
            }
            if let Some((lvl, msg)) = log_event {
                add_log(state, lvl, &msg);
            }
        }
        Transport::Stdio { .. } => {
            // Le child stdio peut être MORT : on note le crash (code de sortie)
            // et on garde l'information pour l'historique runtime.
            let (alive, exit_code) = {
                let mut children = lockv(&state.children);
                let mut still = Vec::new();
                let mut code = None;
                for (cid, h) in children.iter_mut() {
                    if cid == id {
                        match h.child.try_wait() {
                            Ok(Some(st)) => {
                                still.push(cid.clone());
                                code = st.code();
                            }
                            Ok(None) => {}
                            Err(_) => still.push(cid.clone()),
                        }
                    }
                }
                for cid in still {
                    children.remove(&cid);
                }
                (children.contains_key(id), code)
            };
            if let Some(code) = exit_code {
                let detail = match code {
                    0 => "arrêté proprement (code 0)".to_string(),
                    c => format!("crash : code de sortie {c}"),
                };
                add_log(state, "warn", &format!("[{id}] {detail}"));
                runtime_event(state, id, "crash", &detail);
            }
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
                // (re)démarrage auto avec handshake complet — même garde anti-doublon
                if !lockv(&state.spawning).insert(id.to_string()) {
                    return;
                }
                let cmd = {
                    let mut servers = lockv(&state.servers);
                    servers
                        .iter_mut()
                        .find(|s| s.id == id)
                        .and_then(|s| s.start_cmd.clone())
                };
                if let Some(cmd) = cmd {
                    match probe::probe_stdio(
                        &cmd.program,
                        &cmd.args,
                        cmd.workdir.as_deref(),
                        15000,
                        true,
                    ) {
                        Ok((p, handle)) => {
                            if let Some(h) = handle {
                                lockv(&state.children).insert(id.to_string(), h);
                            }
                            lockv(&state.spawn_fails).remove(id);
                            let mut servers = lockv(&state.servers);
                            if let Some(srv) = servers.iter_mut().find(|s| s.id == id) {
                                srv.state = SState::Up;
                                srv.pid = p.pid;
                                srv.server_name = p.name.clone();
                                srv.version = p.version.clone();
                                srv.protocol = p.protocol.clone();
                                srv.sse = p.sse;
                                srv.sse_endpoint = p.sse_endpoint;
                                srv.lang = p.lang.clone();
                                srv.framework = p.framework.clone();
                                srv.tools = p.tools.clone();
                                srv.latency_ms = Some(p.latency_ms);
                                srv.last_seen_ts = Some(crate::types::now_ts());
                                srv.last_error = None;
                            }
                            drop(servers);
                            add_log(
                                state,
                                "info",
                                &format!(
                                    "[{}] stdio démarré (pid {}) : {} — {} tools",
                                    id,
                                    p.pid.map(|x| x.to_string()).unwrap_or_else(|| "?".into()),
                                    p.name.as_deref().unwrap_or("?"),
                                    p.tools.len()
                                ),
                            );
                            runtime_event(state, id, "start-ok", "stdio");
                        }
                        Err(e) => {
                            let mut servers = lockv(&state.servers);
                            if let Some(srv) = servers.iter_mut().find(|s| s.id == id) {
                                srv.state = SState::Down;
                                srv.last_error = Some(e.clone());
                                srv.pid = None;
                            }
                            drop(servers);
                            note_spawn_failure(state, id);
                            add_log(state, "warn", &format!("[{}] stdio KO : {}", id, e));
                        }
                    }
                }
                // libère la garde anti-doublon quoi qu'il arrive
                lockv(&state.spawning).remove(id);
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
        None => {
            let s = lockv(&state.settings);
            (s.scan_from, s.scan_to)
        }
    };
    add_log(
        state,
        "info",
        &format!("scan {}..{} sur [::1]/127.0.0.1…", from, to),
    );
    let mut found = 0u32;
    let mut open: Vec<(u16, String)> = Vec::new();
    for port in from..=to {
        if probe::port_open("::1", port, std::time::Duration::from_millis(150)) {
            open.push((port, "::1".into()));
        } else if probe::port_open("127.0.0.1", port, std::time::Duration::from_millis(150)) {
            open.push((port, "127.0.0.1".into()));
        }
    }
    open.sort();
    open.dedup();
    let own_ui = lockv(&state.settings).ui_port;
    let known: Vec<u16> = lockv(&state.servers)
        .iter()
        .filter_map(|s| match &s.transport {
            Transport::Http { host, port, .. }
                if host.as_deref().map(crate::is_loopback_bind).unwrap_or(true) =>
            {
                Some(*port)
            }
            _ => None,
        })
        .collect();
    for (port, host) in open {
        if port == own_ui || known.contains(&port) {
            continue;
        }
        if let Some((path, p)) = probe::probe_mcp_any_path(port, 4000) {
            let id = format!("disc-{}", port);
            let label = p.name.clone().unwrap_or_else(|| format!("port-{}", port));
            let mut servers = lockv(&state.servers);
            if !servers.iter().any(|s| {
                s.id == id
                    || matches!(&s.transport,
                        Transport::Http { host, port: sp, .. }
                        if *sp == port
                            && host.as_deref().map(crate::is_loopback_bind).unwrap_or(true))
            }) {
                found += 1;
                servers.push(Server {
                    id: id.clone(),
                    label,
                    transport: Transport::Http {
                        host: None,
                        port,
                        path: path.to_string(),
                    },
                    origin: Origin::Discovered,
                    enabled: true,
                    state: SState::Up,
                    auto_restart: true,
                    favorite: false,
                    pid: p.pid,
                    bound_host: p.host.clone().or(Some(host)),
                    server_name: p.name.clone(),
                    version: p.version.clone(),
                    protocol: p.protocol.clone(),
                    sse: p.sse,
                    sse_endpoint: p.sse_endpoint,
                    ws: p.ws,
                    lang: p.lang.clone(),
                    framework: p.framework.clone(),
                    via_ssh: false,
                    tools: p.tools,
                    latency_ms: Some(p.latency_ms),
                    last_seen_ts: Some(crate::types::now_ts()),
                    last_error: None,
                    start_cmd: None,
                    tools_disabled: vec![],
                });
                drop(servers);
                add_log(
                    state,
                    "info",
                    &format!(
                        "[scan] serveur MCP découvert sur port {} : {} v{}",
                        port,
                        p.name.as_deref().unwrap_or("?"),
                        p.version.as_deref().unwrap_or("?")
                    ),
                );
            }
        }
    }
    add_log(
        state,
        "info",
        &format!("scan terminé : {} nouveaux serveurs MCP", found),
    );
}

/// Découverte locale — candidats = ports en écoute MOINS les serveurs locaux
/// déjà connus et notre propre port UI. Pur, testé.
pub(super) fn discovery_candidates(ports: &[u16], known_local: &[u16], own_ui: u16) -> Vec<u16> {
    ports
        .iter()
        .copied()
        .filter(|p| *p != own_ui && !known_local.contains(p))
        .collect()
}

/// AUTO-SCAN (boot + bouton ⟳) : sonde TOUS les ports en écoute de la machine
/// — netstat/ss, l'OS sait déjà ce qui écoute, aucune plage à deviner. Un
/// serveur sur 8787 est trouvé même avec scan_from/scan_to = 3000-3200 (le cas
/// qui rendait des serveurs invisibles). Les répondeurs deviennent des serveurs
/// découverts (non persistés : re-découverts à chaque boot, comme le range scan).
#[allow(unused_must_use)]
pub fn scan_listening(state: &Arc<AppState>) {
    let ports = probe::local_listening_ports();
    let own_ui = lockv(&state.settings).ui_port;
    let known_local: Vec<u16> = lockv(&state.servers)
        .iter()
        .filter_map(|s| match &s.transport {
            Transport::Http { host, port, .. }
                if host.as_deref().map(crate::is_loopback_bind).unwrap_or(true) =>
            {
                Some(*port)
            }
            _ => None,
        })
        .collect();
    let candidates = discovery_candidates(&ports, &known_local, own_ui);
    add_log(
        state,
        "info",
        &format!(
            "scan local : {} port(s) en écoute, {} inconnu(s) à sonder (tous ports — aucune plage)",
            ports.len(),
            candidates.len()
        ),
    );
    let mut found = 0u32;
    // sondes PARALLÈLES : un service silencieux (ports RPC Windows & co)
    // encaisse la connexion sans jamais répondre et coûte son timeout entier
    // (4 s) — en séquentiel, une trentaine de ports bavards paralysait la
    // découverte des minutes durant. Ajouts sérialisés ensuite (verrou servers).
    // Chaque port est sondé sur les chemins MCP usuels (/mcp, /, /api/mcp).
    let handles: Vec<_> = candidates
        .into_iter()
        .map(|port| std::thread::spawn(move || (port, probe::probe_mcp_any_path(port, 4000))))
        .collect();
    for h in handles {
        let (port, probe_res) = match h.join() {
            Ok(x) => x,
            Err(_) => continue,
        };
        let Some((path, p)) = probe_res else {
            continue;
        };
        let id = format!("disc-{}", port);
        let mut servers = lockv(&state.servers);
        if servers.iter().any(|s| {
            s.id == id
                || matches!(&s.transport,
                    Transport::Http { host, port: sp, .. }
                    if *sp == port
                        && host.as_deref().map(crate::is_loopback_bind).unwrap_or(true))
        }) {
            continue;
        }
        let label = p.name.clone().unwrap_or_else(|| format!("port-{}", port));
        servers.push(Server {
            id: id.clone(),
            label,
            transport: Transport::Http {
                host: None,
                port,
                path: path.to_string(),
            },
            origin: Origin::Discovered,
            enabled: true,
            state: SState::Up,
            auto_restart: true,
            favorite: false,
            pid: p.pid,
            bound_host: p.host.clone(),
            server_name: p.name.clone(),
            version: p.version.clone(),
            protocol: p.protocol.clone(),
            sse: p.sse,
            sse_endpoint: p.sse_endpoint,
            ws: p.ws,
            lang: p.lang.clone(),
            framework: p.framework.clone(),
            via_ssh: false,
            tools: p.tools,
            latency_ms: Some(p.latency_ms),
            last_seen_ts: Some(crate::types::now_ts()),
            last_error: None,
            start_cmd: None,
            tools_disabled: vec![],
        });
        drop(servers);
        found += 1;
        add_log(
            state,
            "info",
            &format!(
                "[scan] serveur MCP découvert sur port {} : {} v{}",
                port,
                p.name.as_deref().unwrap_or("?"),
                p.version.as_deref().unwrap_or("?")
            ),
        );
    }
    add_log(
        state,
        "info",
        &format!("scan local terminé : {} nouveau(x) serveur(s) MCP", found),
    );
}

// ---------- scan réseau local (MCP distants) ----------

#[allow(unused_must_use)]
pub fn scan_network(state: &Arc<AppState>, range: Option<(u16, u16)>) {
    let (from, to) = match range {
        Some(r) => r,
        None => {
            let s = lockv(&state.settings);
            (s.scan_from, s.scan_to)
        }
    };
    let prefixes = network_prefixes();
    if prefixes.is_empty() {
        add_log(state, "warn", "scan réseau : aucune interface IPv4 trouvée");
        return;
    }
    let total_found = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    for (prefix, self_ip) in &prefixes {
        add_log(
            state,
            "info",
            &format!(
                "scan réseau {}.1-254 (self={}) ports {}..{} (parallèle)",
                prefix, self_ip, from, to
            ),
        );
        let (found, _tested) = scan_network_range(state, prefix, self_ip, from, to);
        total_found.fetch_add(found, std::sync::atomic::Ordering::Relaxed);
    }
    let found = total_found.load(std::sync::atomic::Ordering::Relaxed);
    add_log(
        state,
        "info",
        &format!(
            "scan réseau terminé : {} nouveau(x) serveur(s) MCP distant(s)",
            found
        ),
    );
    prune_dead_network_servers(state);
    merge_duplicate_servers(state);
}

/// FUSION des doublons historiques : les imports répétés (SSH, scan) ont pu
/// créer plusieurs entrées pour le MÊME serveur (même nom annoncé, même
/// machine, port changé). On ne garde que la plus RÉCENTE (last_seen_ts max).
///
/// Périmètre volontairement RESTREINT (audit 2026-10-26, P1) : seules les
/// entrées DÉCOUVERTES (origin Discovered), HTTP avec hôte explicite et nom
/// annoncé non vide sont fusionnables. Jamais un serveur manuel, seed ou
/// importé d'un pair ; jamais une entrée sans nom annoncé — l'ancien repli
/// sur le label fusionnait deux serveurs distincts (deux instances légitimes
/// sur des ports différents, ou deux ajouts sans label) et la suppression
/// était persistée.
pub(super) fn merge_duplicate_servers(state: &Arc<AppState>) -> usize {
    let mut servers = lockv(&state.servers);
    let before = servers.len();
    // clé = nom ANNONCÉ normalisé + hôte (identité faible : le port peut avoir changé)
    let mut groups: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    for (i, s) in servers.iter().enumerate() {
        if s.origin != Origin::Discovered {
            continue;
        }
        let Transport::Http { host: Some(h), .. } = &s.transport else {
            continue;
        };
        // seul le nom annoncé (serverInfo.name) est une identité fiable :
        // un label (fabriqué « ip:port » ou saisi par l'utilisateur) ne doit
        // JAMAIS servir de clé de fusion
        let Some(name) = s
            .server_name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
        else {
            continue;
        };
        let key = format!("{}@{}", name.to_lowercase(), h);
        groups.entry(key).or_default().push(i);
    }
    let mut to_remove: Vec<usize> = Vec::new();
    for (_key, mut idxs) in groups {
        if idxs.len() < 2 {
            continue;
        }
        // le plus récemment vu gagne
        idxs.sort_by_key(|i| std::cmp::Reverse(servers[*i].last_seen_ts.unwrap_or(0)));
        to_remove.extend(idxs[1..].iter().copied());
    }
    if !to_remove.is_empty() {
        to_remove.sort_unstable();
        to_remove.dedup();
        for i in to_remove.iter().rev() {
            servers.remove(*i);
        }
    }
    let removed = before - servers.len();
    drop(servers);
    if removed > 0 {
        add_log(
            state,
            "info",
            &format!("[réseau] {removed} doublon(s) de serveur fusionné(s)"),
        );
    }
    removed
}

/// NETTOYAGE : après un scan complet, un serveur réseau découvert dont la
/// machine n'a pas répondu pendant CE scan est un fantôme (hôte éteint, IP
/// recyclée). On ne supprime que ce qui est **découvert via le réseau**
/// (`net-<ip>.<port>`) — jamais un serveur manuel, seed ou importé d'un pair
/// (une machine éteinte ne doit pas effacer sa configuration).
///
/// Le scan marque les entrées qu'il a vues (`last_seen_ts` rafraîchi) : le
/// prune retire celles qui n'ont pas été rafraîchies pendant le scan.
pub(super) fn prune_dead_network_servers(state: &Arc<AppState>) {
    // fenêtre du scan : on ne purge que ce qui n'a pas été vu depuis 2 min
    let horizon = crate::types::now_ts() - 120;
    let mut servers = lockv(&state.servers);
    let before = servers.len();
    servers.retain(|s| {
        let is_net = s.id.starts_with("net-")
            && matches!(&s.transport, Transport::Http { host: Some(_), .. });
        if !is_net {
            return true;
        }
        // entrée réseau : gardée si elle a été vue récemment (le scan ou le
        // poll l'a sondée) ; sinon c'est un fantôme de machine disparue
        matches!(s.last_seen_ts, Some(ts) if ts > horizon)
    });
    let removed = before - servers.len();
    drop(servers);
    if removed > 0 {
        add_log(
            state,
            "info",
            &format!("[réseau] {} entrée(s) fantôme(s) retirée(s)", removed),
        );
    }
}

/// Toutes les IPv4 locales (toutes interfaces), dédupliquées.
/// Sources : ipconfig (Windows) / hostname -I (unix) + fallback UDP (interface de routage).
/// Le fallback UDP seul peut renvoyer l'IP VPN — d'où le parse des interfaces en priorité.
pub fn local_ips() -> Vec<String> {
    let mut ips = std::collections::BTreeSet::new();
    let output = if cfg!(windows) {
        let mut c = Command::new("ipconfig");
        probe::hidden(&mut c).output()
    } else {
        let mut c = Command::new("hostname");
        probe::hidden(&mut c).arg("-I").output()
    };
    if let Ok(o) = output {
        let text = String::from_utf8_lossy(&o.stdout).to_string();
        if cfg!(windows) {
            // ne prendre que les lignes "IPv4" (exclut masques et lignes "Passerelle par défaut"/"Default Gateway")
            // — le marqueur "ipv4" existe en FR ("Adresse IPv4") comme en EN ("IPv4 Address")
            for line in text.lines() {
                if !line.to_lowercase().contains("ipv4") {
                    continue;
                }
                for tok in line.split_whitespace() {
                    let tok = tok.trim_end_matches(',').trim_start_matches('.');
                    if crate::probe::looks_like_ip(tok) {
                        ips.insert(tok.to_string());
                    }
                }
            }
        } else {
            // `hostname -I` sort des IP brutes séparées par espaces — SANS marqueur
            // "ipv4" : l'ancien parse rejetait tout et le fallback UDP ne renvoyait
            // qu'une seule interface. On parse chaque token.
            for tok in text.split_whitespace() {
                if crate::probe::looks_like_ip(tok) {
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
pub fn scan_network_range(
    state: &Arc<AppState>,
    prefix: &str,
    self_ip: &str,
    from: u16,
    to: u16,
) -> (u32, u32) {
    let ports: Vec<u16> = (from..=to).collect();
    let known_hosts_ports: Vec<(Option<String>, u16)> = lockv(&state.servers)
        .iter()
        .filter_map(|s| match &s.transport {
            Transport::Http { host, port, .. } => Some((host.clone(), *port)),
            _ => None,
        })
        .collect();

    use std::sync::atomic::{AtomicU32, Ordering};
    let found = std::sync::Arc::new(AtomicU32::new(0));
    let tested = std::sync::Arc::new(AtomicU32::new(0));
    let mut handles = Vec::new();
    for last in 1u16..=254 {
        let ip = format!("{}.{}", prefix, last);
        if ip == self_ip {
            continue;
        } // ne pas se scanner soi-même (scan loopback déjà couvert)
        let ports = ports.clone();
        let known = known_hosts_ports.clone();
        let state = state.clone();
        let found = found.clone();
        let tested = tested.clone();
        handles.push(std::thread::spawn(move || {
            let mut hit = false;
            for port in &ports {
                if !probe::port_open(&ip, *port, std::time::Duration::from_millis(400)) {
                    continue;
                }
                hit = true;
                if known
                    .iter()
                    .any(|(h, p)| h.as_deref() == Some(ip.as_str()) && p == port)
                {
                    continue;
                }
                let p = probe::probe_http_host(&ip, *port, "/mcp", 3000);
                if p.mcp_up {
                    found.fetch_add(1, Ordering::Relaxed);
                    let label = p.name.clone().unwrap_or_else(|| format!("{}:{}", ip, port));
                    let mut servers = lockv(&state.servers);
                    // DÉ-DUPLICATION : un serveur déjà connu (même hôte+port) est
                    // ignoré ; un serveur qui a CHANGÉ DE PORT (même nom, même
                    // hôte) remplace l'entrée fantôme au lieu de créer un doublon.
                    let known: Vec<(Option<String>, Option<String>, u16)> = servers
                        .iter()
                        .filter_map(|s| match &s.transport {
                            Transport::Http { host, port, .. } => {
                                Some((s.server_name.clone(), host.clone(), *port))
                            }
                            _ => None,
                        })
                        .collect();
                    let decision = crate::validate::dedup_decide(
                        &known,
                        p.name.as_deref(),
                        Some(ip.as_str()),
                        *port,
                    );
                    let target_id = match decision {
                        crate::validate::DedupDecision::Duplicate => {
                            // déjà connu : on met à jour ses métadonnées (framework,
                            // tools) sans créer d'entrée
                            if let Some(s) = servers.iter_mut().find(|s| {
                                matches!(&s.transport,
                                    Transport::Http { host: Some(h), port: sp, .. } if h == &ip && sp == port)
                            }) {
                                s.server_name = p.name.clone();
                                s.version = p.version.clone();
                                s.framework = p.framework.clone();
                                s.lang = p.lang.clone();
                                s.tools = p.tools;
                                s.latency_ms = Some(p.latency_ms);
                                s.last_seen_ts = Some(crate::types::now_ts());
                            }
                            drop(servers);
                            continue;
                        }
                        crate::validate::DedupDecision::Replace(idx) => {
                            // l'ancien port est fantôme : on le remplace
                            let old = servers.remove(idx);
                            let id = old.id;
                            add_log(
                                &state,
                                "info",
                                &format!(
                                    "[réseau] {} a changé de port ({} → {}) — entrée fantôme remplacée",
                                    label,
                                    known.get(idx).map(|(_, _, p)| *p).unwrap_or(0),
                                    port
                                ),
                            );
                            id
                        }
                        crate::validate::DedupDecision::Add => format!("net-{}.{}", ip, port),
                    };
                    if !servers.iter().any(|s| {
                        matches!(&s.transport,
                        Transport::Http { host: Some(h), port: sp, .. } if h == &ip && sp == port)
                    }) {
                        servers.push(Server {
                            id: target_id,
                            label,
                            transport: Transport::Http {
                                host: Some(ip.clone()),
                                port: *port,
                                path: "/mcp".into(),
                            },
                            origin: Origin::Discovered,
                            enabled: true,
                            state: SState::Up,
                            auto_restart: true,
                            favorite: false,
                            pid: None,
                            bound_host: Some(ip.clone()),
                            server_name: p.name.clone(),
                            version: p.version.clone(),
                            protocol: p.protocol.clone(),
                            sse: p.sse,
                            sse_endpoint: p.sse_endpoint,
                            ws: p.ws,
                            lang: p.lang.clone(),
                            framework: p.framework.clone(),
                            via_ssh: false,
                            tools: p.tools,
                            latency_ms: Some(p.latency_ms),
                            last_seen_ts: Some(crate::types::now_ts()),
                            last_error: None,
                            start_cmd: None,
                            tools_disabled: vec![],
                        });
                        drop(servers);
                        add_log(
                            &state,
                            "info",
                            &format!(
                                "[réseau] serveur MCP distant découvert : {}:{} — {} v{}",
                                ip,
                                port,
                                p.name.as_deref().unwrap_or("?"),
                                p.version.as_deref().unwrap_or("?")
                            ),
                        );
                    }
                }
            }
            if hit {
                tested.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    (
        found.load(Ordering::Relaxed),
        tested.load(Ordering::Relaxed),
    )
}
