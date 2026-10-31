// Chargement/sauvegarde de config.json, parsing des serveurs et réglages, serveurs par défaut.
// (split v0.7.20 : manage.rs reste la racine — API stable via réexport)
use super::*;

pub(super) fn default_servers() -> Vec<Server> {
    // Commandes de lancement réelles (launcher MCP MONITOR v7.3) : chaque serveur
    // est démarré via `node <workdir>/bin/launch.cjs` depuis son workdir.
    // Ces seeds embarquent les commandes pour que l'app puisse lancer/arrêter
    // les serveurs même sans config.json (exe distribué, premier démarrage).
    const BASE: &str = "C:/Users/Deamon/Desktop/Backup/Serveur MCP";
    let mk = |id: &str, label: &str, port: u16, enabled: bool, workdir: &str| {
        let wd = if workdir.starts_with("C:/") {
            workdir.to_string()
        } else {
            format!("{}/{}", BASE, workdir)
        };
        Server {
            id: id.to_string(),
            label: label.to_string(),
            transport: Transport::Http {
                host: None,
                port,
                path: "/mcp".to_string(),
            },
            origin: Origin::Seed,
            enabled,
            auto_restart: true,
            favorite: false,
            state: if enabled {
                SState::Down
            } else {
                SState::Disabled
            },
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
                program: "node".to_string(),
                args: vec![format!("{}/bin/launch.cjs", wd)],
                workdir: Some(wd.clone()),
            }),
            tools_disabled: vec![],
        }
    };
    vec![
        mk("seed-overmind", "Overmind", 3099, true, "Workflow"),
        mk(
            "seed-discord",
            "serveur_discord",
            3141,
            true,
            "serveur_discord",
        ),
        mk(
            "seed-postgresql",
            "serveur_PostGreSQL",
            5433,
            true,
            "serveur_PostGreSQL",
        ),
        mk(
            "seed-debats",
            "debats",
            3100,
            false,
            "C:/SierraChart/ACS_Source/BTCacsil/debats-mcp-server",
        ),
        mk("seed-x", "X", 3142, true, "X"),
        mk("seed-news-btc", "NEWS_BTC", 3143, false, "NEWS_BTC"),
        mk("seed-news", "NEWS", 3144, false, "NEWS"),
        // Démo LiteMCP (lib Rust MCP stateless) : binaire produit par
        // `cargo build --release --example hello` du dépôt litemcp. Sert de
        // banc d'essai du fingerprint framework (badge « LiteMCP ») — si le
        // binaire manque, le mécanisme « 3 échecs → auto-restart off » s'applique.
        Server {
            id: "seed-litemcp".to_string(),
            label: "LiteMCP (démo)".to_string(),
            transport: Transport::Http {
                host: None,
                port: 8080,
                path: "/mcp".to_string(),
            },
            origin: Origin::Seed,
            enabled: true,
            auto_restart: true,
            favorite: false,
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
                program: "C:/Users/Deamon/Desktop/litemcp/target/release/examples/hello.exe"
                    .to_string(),
                args: vec![],
                workdir: Some("C:/Users/Deamon/Desktop/litemcp".to_string()),
            }),
            tools_disabled: vec![],
        },
    ]
}

/// Résolution du chemin de config au runtime (voir main.rs) :
/// 1. override explicite (MCC_CONFIG)
/// 2. dossier de l'exe / config.json s'il existe (installé : l'app a SA config)
/// 3. CARGO_MANIFEST_DIR/config.json s'il existe (dev local — l'exe de dev vit
///    dans target/, l'installé à côté de sa config ; un exe installé ne doit
///    JAMAIS réécrire la config du dossier de dev)
/// 4. dossier de l'exe / config.json (premier boot d'une install fraîche)
pub fn resolve_config_path(manifest_dir: &str, override_env: Option<String>) -> String {
    if let Some(p) = override_env {
        if !p.is_empty() {
            return p;
        }
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from(manifest_dir));
    let exe_config = exe_dir.join("config.json");
    if exe_config.exists() {
        return exe_config.to_string_lossy().to_string();
    }
    let dev_path = std::path::Path::new(manifest_dir).join("config.json");
    if dev_path.exists() {
        return dev_path.to_string_lossy().to_string();
    }
    exe_config.to_string_lossy().to_string()
}

/// Charge le journal runtime persisté (backoff d'autoboot + historique des
/// crashs et redémarrages) — appelé au boot : le compteur d'échecs survit
/// ainsi au redémarrage de l'application.
pub fn load_runtime(path: &str) -> crate::runtime::RuntimeLog {
    let Ok(file) = std::fs::read_to_string(path) else {
        return crate::runtime::RuntimeLog::new();
    };
    let Ok(v) = serde_json::from_str::<Value>(&file) else {
        return crate::runtime::RuntimeLog::new();
    };
    v.get("runtime")
        .map(crate::runtime::from_json)
        .unwrap_or_default()
}

/// Historique runtime formaté pour l'API /api/runtime : une liste plate
/// d'événements (du plus récent au plus ancien) + un résumé par serveur.
pub fn runtime_json(state: &Arc<AppState>) -> Value {
    let rt = lockv(&state.runtime);
    let mut events: Vec<crate::runtime::RuntimeEvent> = Vec::new();
    let mut summary: Vec<Value> = Vec::new();
    let mut ids: Vec<&String> = rt.keys().collect();
    ids.sort();
    for id in ids {
        let Some(s) = rt.get(id) else { continue };
        events.extend(s.events.iter().cloned());
        summary.push(json!({
            "id": id,
            "label": s.events.last().map(|e| e.label.clone()).unwrap_or_else(|| id.clone()),
            "fails": s.fails,
            "restarts": s.restarts,
            "crashes": s.crashes,
            "backoff_secs": s.backoff_secs(),
            "last_success_ts": s.last_success_ts,
        }));
    }
    events.sort_by_key(|a| std::cmp::Reverse(a.ts));
    json!({
        "events": events.iter().take(200).map(|e| e.to_json()).collect::<Vec<_>>(),
        "servers": summary,
    })
}
pub fn load_config(path: &str) -> (Vec<Server>, crate::types::Settings) {
    let defaults = default_servers();
    let settings = crate::types::Settings::default();
    let file = match std::fs::read_to_string(path) {
        Ok(f) => f,
        Err(_) => return (defaults, settings),
    };
    let v: Value = match serde_json::from_str(&file) {
        Ok(v) => v,
        Err(_) => return (defaults, settings),
    };
    let mut servers = defaults;
    if let Some(arr) = v.get("servers").and_then(|x| x.as_array()) {
        for s in arr {
            let id = s
                .get("id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if id.is_empty() {
                continue;
            }
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
            if let Transport::Stdio {
                program,
                args,
                workdir,
            } = s.transport.clone()
            {
                if !program.is_empty() {
                    s.start_cmd = Some(StartCmd {
                        program,
                        args,
                        workdir,
                    });
                }
            }
        }
    }
    let settings = parse_settings(&v, settings);
    // « Retirer durablement » : les seeds explicitement retirés ne sont pas réinjectés
    let removed: Vec<String> = settings.removed_seeds.clone();
    servers.retain(|s| !(s.origin == Origin::Seed && removed.contains(&s.id)));
    (servers, settings)
}

#[allow(unused_must_use)]
pub(super) fn apply_json(srv: &mut Server, s: &Value) {
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
                if let Some(v) = s.get("via_ssh").and_then(|x| x.as_bool()) {
                    srv.via_ssh = v;
                }
            }
        } else if t == "stdio" {
            let args: Vec<String> = s
                .get("args")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|i| i.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            srv.transport = Transport::Stdio {
                program: s
                    .get("program")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                args,
                workdir: s.get("workdir").and_then(|x| x.as_str()).map(String::from),
            };
        }
    }
    // commande de lancement optionnelle pour un serveur HTTP (Start/Stop par MCP Control)
    if let Some(program) = s.get("program").and_then(|x| x.as_str()) {
        if !program.is_empty() {
            let args: Vec<String> = s
                .get("args")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|i| i.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            srv.start_cmd = Some(StartCmd {
                program: program.to_string(),
                args,
                workdir: s.get("workdir").and_then(|x| x.as_str()).map(String::from),
            });
        }
    }
    if let Some(l) = s.get("label").and_then(|x| x.as_str()) {
        srv.label = l.to_string();
    }
    if let Some(arr) = s.get("tools_disabled").and_then(|x| x.as_array()) {
        srv.tools_disabled = arr
            .iter()
            .filter_map(|i| i.as_str().map(String::from))
            .collect();
    }
}

pub(super) fn from_json(s: &Value) -> Option<Server> {
    let id = s.get("id").and_then(|x| x.as_str())?.to_string();
    let label = s
        .get("label")
        .and_then(|x| x.as_str())
        .unwrap_or(&id)
        .to_string();
    let transport = match s.get("transport").and_then(|x| x.as_str()) {
        Some("stdio") => Transport::Stdio {
            program: s
                .get("program")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            args: s
                .get("args")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|i| i.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            workdir: s.get("workdir").and_then(|x| x.as_str()).map(String::from),
        },
        _ => Transport::Http {
            host: s.get("host").and_then(|x| x.as_str()).map(String::from),
            port: s.get("port").and_then(|x| x.as_u64())? as u16,
            path: s
                .get("path")
                .and_then(|x| x.as_str())
                .unwrap_or("/mcp")
                .to_string(),
        },
    };
    let enabled = s.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true);
    // Autoboot par serveur : défaut true (comportement historique), false = le
    // serveur n'est pas relancé automatiquement (bouton ⚡ de l'UI).
    let auto_restart = s
        .get("auto_restart")
        .and_then(|x| x.as_bool())
        .unwrap_or(true);
    let via_ssh = s.get("via_ssh").and_then(|x| x.as_bool()).unwrap_or(false);
    // commande de lancement optionnelle (HTTP lancé par MCP Control)
    let start_cmd = {
        let program = s
            .get("program")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if program.is_empty() {
            None
        } else {
            Some(StartCmd {
                program,
                args: s
                    .get("args")
                    .and_then(|x| x.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|i| i.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                workdir: s.get("workdir").and_then(|x| x.as_str()).map(String::from),
            })
        }
    };
    Some(Server {
        id,
        label,
        transport,
        origin: Origin::Manual,
        enabled,
        favorite: s.get("favorite").and_then(|x| x.as_bool()).unwrap_or(false),
        auto_restart,
        state: if enabled {
            SState::Down
        } else {
            SState::Disabled
        },
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
        via_ssh,
        tools: vec![],
        latency_ms: None,
        last_seen_ts: None,
        last_error: None,
        start_cmd,
        tools_disabled: vec![],
    })
}

pub(super) fn parse_settings(v: &Value, mut st: crate::types::Settings) -> crate::types::Settings {
    if let Some(s) = v.get("settings") {
        if let Some(n) = s.get("notify") {
            st.notify = crate::notify::from_json(n);
        }
        if let Some(x) = s.get("refresh_secs").and_then(|x| x.as_u64()) {
            // clamp : 0 = busy-loop CPU, trop long = état périmé à l'écran
            st.refresh_secs = x.clamp(5, 3600);
        }
        if let Some(x) = s.get("scan_from").and_then(|x| x.as_u64()) {
            st.scan_from = x as u16;
        }
        if let Some(x) = s.get("scan_to").and_then(|x| x.as_u64()) {
            st.scan_to = x as u16;
        }
        if let Some(x) = s.get("ui_port").and_then(|x| x.as_u64()) {
            st.ui_port = x as u16;
        }
        if let Some(x) = s.get("ui_bind").and_then(|x| x.as_str()) {
            st.ui_bind = x.to_string();
        }
        if let Some(x) = s.get("network_enabled").and_then(|x| x.as_bool()) {
            st.network_enabled = x;
        }
        if let Some(arr) = s.get("network_ports").and_then(|x| x.as_array()) {
            let ports: Vec<u16> = arr
                .iter()
                .filter_map(|x| x.as_u64())
                .filter(|p| *p >= 1 && *p <= 65535)
                .map(|p| p as u16)
                .collect();
            if !ports.is_empty() {
                st.network_ports = ports;
            }
        }
        if let Some(x) = s.get("api_access").and_then(|x| x.as_str()) {
            st.api_access = if x.is_empty() {
                None
            } else {
                Some(x.to_string())
            };
        }
        if let Some(arr) = s.get("allowed_origins").and_then(|x| x.as_array()) {
            let o: Vec<String> = arr
                .iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect();
            if !o.is_empty() {
                st.allowed_origins = o;
            }
        }
        if let Some(arr) = s.get("removed_seeds").and_then(|x| x.as_array()) {
            st.removed_seeds = arr
                .iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect();
        }
        if let Some(x) = s.get("autostart").and_then(|x| x.as_bool()) {
            st.autostart = x;
        }
        if let Some(arr) = s.get("server_order").and_then(|x| x.as_array()) {
            st.server_order = arr
                .iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect();
        }
        if let Some(arr) = s.get("ssh_hosts").and_then(|x| x.as_array()) {
            st.ssh_hosts = arr
                .iter()
                .filter_map(|h| {
                    Some(crate::types::SshHost {
                        ip: h.get("ip")?.as_str()?.to_string(),
                        user: h.get("user")?.as_str()?.to_string(),
                    })
                })
                .collect();
        }
    }
    st
}

#[allow(unused_must_use)]
pub fn save_config(state: &AppState) {
    if state.config_path.is_empty() {
        return;
    }
    let servers = lockv(&state.servers).clone();
    let settings = lockv(&state.settings).clone();
    let mut arr = Vec::new();
    for s in &servers {
        // les serveurs découverts (scan local/réseau) ne sont PAS persistés :
        // redécouverts au boot — SAUF les via_ssh : leur re-découverte exige une
        // connexion SSH réussie, on garde donc le résultat.
        if s.origin == Origin::Discovered && !s.via_ssh {
            continue;
        }
        let mut o = Map::new();
        o.insert("id".into(), json!(s.id));
        o.insert("label".into(), json!(s.label));
        o.insert("enabled".into(), json!(s.enabled));
        // Autoboot par serveur : on n'écrit que la DÉSACTIVATION (le défaut est
        // true) — config.json reste lisible et le bouton ⚡ survit au redémarrage.
        if !s.auto_restart {
            o.insert("auto_restart".into(), json!(false));
        }
        // Favori (⭐) : on n'écrit que l'ACTIVATION (le défaut est false) —
        // config.json reste lisible et l'épingle survit au redémarrage.
        if s.favorite {
            o.insert("favorite".into(), json!(true));
        }
        if !s.tools_disabled.is_empty() {
            o.insert("tools_disabled".into(), json!(s.tools_disabled));
        }
        match &s.transport {
            Transport::Http { host, port, path } => {
                o.insert("transport".into(), json!("http"));
                if let Some(h) = host {
                    o.insert("host".into(), json!(h));
                }
                o.insert("port".into(), json!(port));
                o.insert("path".into(), json!(path));
                if s.via_ssh {
                    o.insert("via_ssh".into(), json!(true));
                }
                // commande de lancement (Start/Stop par MCP Control)
                if let Some(c) = &s.start_cmd {
                    o.insert("program".into(), json!(c.program));
                    o.insert("args".into(), json!(c.args));
                    if let Some(w) = &c.workdir {
                        o.insert("workdir".into(), json!(w));
                    }
                }
            }
            Transport::Stdio {
                program,
                args,
                workdir,
            } => {
                o.insert("transport".into(), json!("stdio"));
                o.insert("program".into(), json!(program));
                o.insert("args".into(), json!(args));
                if let Some(w) = workdir {
                    o.insert("workdir".into(), json!(w));
                }
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
            "api_access": settings.api_access,
            "allowed_origins": settings.allowed_origins,
            "removed_seeds": settings.removed_seeds,
            "server_order": settings.server_order,
            "autostart": settings.autostart,
            "notify": crate::notify::to_json(&settings.notify),
            "ssh_hosts": settings
                .ssh_hosts
                .iter()
                .map(|h| json!({"ip": h.ip, "user": h.user}))
                .collect::<Vec<_>>(),
        },
        // runtime : backoff d'autoboot + historique des crashs (persiste)
        "runtime": crate::runtime::to_json(&lockv(&state.runtime)),
    });
    let pretty = serde_json::to_string_pretty(&doc).unwrap_or_default();
    let tmp = format!("{}.tmp", state.config_path);
    // Écriture atomique tmp+rename — les erreurs sont LOGUÉES (une écriture ratée
    // silencieuse = ajout perdu au reboot sans aucun indice).
    if let Err(e) = std::fs::write(&tmp, &pretty) {
        add_log(
            state,
            "error",
            &format!("config : écriture impossible ({}): {}", tmp, e),
        );
        return;
    }
    if let Err(e) = std::fs::rename(&tmp, &state.config_path) {
        add_log(
            state,
            "error",
            &format!(
                "config : remplacement impossible ({}): {}",
                state.config_path, e
            ),
        );
    }
}
