// Builders JSON de l API : état, détail, runtime + décision LAN.
// (split v0.7.20 : manage.rs reste la racine — API stable via réexport)
use super::*;

pub(super) fn now_compact() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64 % 100_000_000)
        .unwrap_or(0)
}

pub fn start_cmd_for(srv: &Server) -> Option<StartCmd> {
    match &srv.transport {
        Transport::Stdio {
            program,
            args,
            workdir,
        } => Some(StartCmd {
            program: program.clone(),
            args: args.clone(),
            workdir: workdir.clone(),
        }),
        Transport::Http { .. } => srv.start_cmd.clone(),
    }
}

// ---------- sérialisation UI ----------

pub(super) fn server_json(s: &Server, managed: bool) -> Value {
    let (ttype, extra) = match &s.transport {
        Transport::Http { host, port, path } => {
            ("http", json!({"host": host, "port": port, "path": path}))
        }
        Transport::Stdio {
            program,
            args,
            workdir,
        } => (
            "stdio",
            json!({
                "program": program, "args": args, "workdir": workdir,
            }),
        ),
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
        "favorite": s.favorite,
        "sse": s.sse, "sse_endpoint": s.sse_endpoint, "ws": s.ws, "lang": s.lang,
        "framework": s.framework,
        "via_ssh": s.via_ssh,
        // contrôle du process : maintenu par CE moteur MCP Control (children) ?
        "managed": managed,
        // commande de lancement (Start/Stop) — panneau Démarrage de l'UI
        "start_cmd": s.start_cmd.as_ref().map(|c| json!({
            "program": c.program, "args": c.args, "workdir": c.workdir,
        })),
    })
}

/// Ids des process actuellement maintenus par MCP Control (copie —
/// évite d'imbriquer le lock children sous le lock servers).
pub(super) fn managed_ids(state: &Arc<AppState>) -> std::collections::HashSet<String> {
    lockv(&state.children).keys().cloned().collect()
}

/// Décision pure « le LAN est-il joignable ? » — testable sans réseau.
/// Un VPN fait répondre SA passerelle alors que le LAN est coupé : si tous les
/// serveurs distants échouent en réseau, on conclut LAN coupé même si le ping
/// de la passerelle passe.
pub fn lan_decision(gw: Option<bool>, distants_ko: bool) -> Option<bool> {
    match (gw, distants_ko) {
        (Some(true), true) => Some(false),
        (None, true) => Some(false),
        (g, _) => g,
    }
}

/// Mesure si le LAN répond et publie le résultat.
/// Deux signaux, car un VPN fait répondre SA passerelle tout en coupant le LAN :
/// 1. ping des passerelles par défaut (aucune ne répond = réseau coupé) ;
/// 2. état des serveurs DISTANTS : s'ils échouent tous avec une erreur réseau
///    (ssh/connect/timeout/injoignable), c'est le LAN qui est coupé, pas eux.
pub fn refresh_lan_state(state: &Arc<AppState>) {
    let gw = probe::lan_reachable();
    let distants_ko = {
        let servers = lockv(&state.servers);
        let distants: Vec<(bool, Option<String>)> = servers
            .iter()
            .filter(|s| s.enabled && matches!(&s.transport, Transport::Http { host: Some(_), .. }))
            .map(|s| (s.state == SState::Up, s.last_error.clone()))
            .collect();
        !distants.is_empty()
            && distants.iter().all(|(up, err)| {
                !up && err
                    .as_deref()
                    .map(|e| {
                        let e = e.to_lowercase();
                        e.contains("ssh")
                            || e.contains("connect")
                            || e.contains("timed out")
                            || e.contains("injoignable")
                            || e.contains("unreachable")
                    })
                    .unwrap_or(false)
            })
    };
    let reachable = lan_decision(gw, distants_ko);
    let mut slot = lockv(&state.lan_ok);
    let changed = *slot != reachable;
    *slot = reachable;
    drop(slot);
    if changed {
        match reachable {
            Some(false) => add_log(
                state,
                "warn",
                "réseau local injoignable (passerelle muette et serveurs distants en échec réseau) — \
                 VPN actif ou connexion coupée : les serveurs distants ne peuvent pas être sondés",
            ),
            Some(true) => add_log(state, "info", "réseau local joignable"),
            None => {}
        }
    }
}

pub fn state_json(state: &Arc<AppState>) -> Value {
    let managed = managed_ids(state);
    // chaque verrou est pris et relâché isolément pour éviter de maintenir
    // 5 mutex simultanément (réduit la contention et tout risque d'inversion d'ordre).
    let (order, autostart, settings_val) = {
        let settings = lockv(&state.settings);
        (
            settings.server_order.clone(),
            settings.autostart,
            json!({
                "refresh_secs": settings.refresh_secs,
                "ui_port": settings.ui_port,
                "ui_bind": settings.ui_bind,
                "scan_from": settings.scan_from,
                "scan_to": settings.scan_to,
                "network_enabled": settings.network_enabled,
                "network_ports": settings.network_ports,
                "ssh_hosts": settings.ssh_hosts.iter().map(|h| json!({
                    "ip": h.ip, "user": h.user
                })).collect::<Vec<_>>(),
                // config des notifications (toast/webhook) : l'UI locale en a
                // besoin pour son panneau Réglages → Notifications. JAMAIS
                // exposée au gateway MCP (l'URL peut porter un Authorization).
                "notify": crate::notify::to_json(&settings.notify),
            }),
        )
    };
    let (list, up, total, tools) = {
        let servers = lockv(&state.servers);
        // affichage : ordre préféré (glisser-déposer), non listés en dernier (stable)
        let mut list: Vec<(usize, Value)> = servers
            .iter()
            .map(|s| {
                let pos = order.iter().position(|x| x == &s.id).unwrap_or(usize::MAX);
                (pos, server_json(s, managed.contains(&s.id)))
            })
            .collect();
        list.sort_by_key(|(pos, _)| *pos);
        let list: Vec<Value> = list.into_iter().map(|(_, v)| v).collect();
        let up = servers.iter().filter(|s| s.state == SState::Up).count();
        let total = servers.len();
        // ne compte que les tools des serveurs effectivement UP (les tools mis en
        // cache d'un serveur DOWN/PORT-ONLY restent visibles dans le panneau détail
        // mais ne gonflent pas le compteur d'outils disponibles).
        let tools: usize = servers
            .iter()
            .filter(|s| s.state == SState::Up)
            .map(tools_actifs)
            .sum();
        (list, up, total, tools)
    };
    let logarr: Vec<Value> = {
        let log = lockv(&state.log);
        log.iter()
            .rev()
            .take(80)
            .map(|e| {
                json!({
                    "ts": e.ts, "level": e.level, "msg": e.msg
                })
            })
            .collect()
    };
    let peers_arr: Vec<Value> = {
        let peers = lockv(&state.peers);
        peers
            .values()
            .map(|p| {
                json!({
                    "ip": p.ip, "ui_port": p.ui_port, "last_seen": p.last_seen,
                    "stats": p.servers.get("stats"),
                    "servers": p.servers.get("servers"),
                })
            })
            .collect()
    };
    let latest = lockv(&state.update_latest).clone();
    let lan = *lockv(&state.lan_ok);
    json!({
        "servers": list, "log": logarr,
        "latest_version": latest,
        "update_available": latest.is_some(),
        "autostart": autostart,
        // None = pas encore mesuré ; false = réseau local coupé (VPN le plus souvent)
        "lan_ok": lan,
        "stats": {"up": up, "total": total, "tools": tools},
        "settings": settings_val,
        "peers": peers_arr,
    })
}

pub fn detail_json(state: &Arc<AppState>, id: &str) -> Value {
    let managed = managed_ids(state);
    let servers = lockv(&state.servers);
    match servers.iter().find(|s| s.id == id) {
        Some(s) => {
            let mut o = match server_json(s, managed.contains(&s.id)) {
                Value::Object(o) => o,
                _ => Map::new(),
            };
            let tools: Vec<Value> = s
                .tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name, "desc": t.desc,
                        "disabled": s.tools_disabled.contains(&t.name),
                    })
                })
                .collect();
            o.insert("tools_list".into(), json!(tools));
            Value::Object(o)
        }
        None => json!({"ok": false, "error": "serveur inconnu"}),
    }
}
