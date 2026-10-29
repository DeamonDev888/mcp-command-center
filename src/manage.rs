use crate::probe;
use crate::types::{lockv, AppState, LogEntry, Origin, SState, Server, StartCmd, Transport};
use serde_json::{json, Map, Value};
#[cfg_attr(not(windows), allow(unused_imports))] // Stdio : chemins cfg(windows)
use std::process::{Command, Stdio};
use std::sync::Arc;

const MAX_LOG: usize = 400;

pub fn add_log(state: &AppState, level: &str, msg: &str) {
    let ts = crate::types::now_ts();
    {
        let mut log = lockv(&state.log);
        log.push_back(LogEntry {
            ts,
            level: level.to_string(),
            msg: msg.to_string(),
        });
        while log.len() > MAX_LOG {
            log.pop_front();
        }
    }
    println!("[{}] {}", level, msg);
    write_log_file(&state.config_path, ts, level, msg);
}

/// Journal disque `mcc.log` (à côté du config.json) : la partie HTTP tourne
/// parfois sans AUCUNE console (autostart `--hidden`, fenêtre masquée dans le
/// tray, binaire fenêtré) — sans fichier, aucun support ni historique possible
/// (audit 2026-10-19 F7). Rotation brute à 512 Ko (une génération `.old` suffit).
fn write_log_file(config_path: &str, ts: u64, level: &str, msg: &str) {
    use std::io::Write;
    if config_path.is_empty() {
        return;
    }
    let path = std::path::Path::new(config_path)
        .with_file_name("mcc.log")
        .to_string_lossy()
        .to_string();
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 512 * 1024 {
            let _ = std::fs::rename(&path, format!("{}.old", path));
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "[{}] [{}] {}", crate::types::iso_utc(ts), level, msg);
    }
}

// ---------- config ----------

fn default_servers() -> Vec<Server> {
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

fn from_json(s: &Value) -> Option<Server> {
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

fn parse_settings(v: &Value, mut st: crate::types::Settings) -> crate::types::Settings {
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

// ---------- boutique MCP (catalogue + import, porté de l'app Hermes desktop :
// mcp-doc.ts pour le parsing des configs, mcp-import.ts pour « collez n'importe quoi ») ----------

pub struct StoreItem {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub cat: &'static str,
    pub kind: &'static str, // npm | python
    pub program: &'static str,
    pub args: &'static [&'static str],
}

pub const STORE: &[StoreItem] = &[
    StoreItem {
        id: "filesystem",
        name: "Filesystem",
        desc: "Lecture/écriture de fichiers dans un dossier autorisé (dossier demandé à l'installation)",
        cat: "Fichiers",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-filesystem"],
    },
    StoreItem {
        id: "gdrive",
        name: "Google Drive",
        desc: "Recherche et lecture des fichiers Google Drive",
        cat: "Fichiers",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-gdrive"],
    },
    StoreItem {
        id: "git",
        name: "Git",
        desc: "Lecture et manipulation de dépôts git (status, diff, log, commit)",
        cat: "Dev",
        kind: "python",
        program: "uvx",
        args: &["mcp-server-git"],
    },
    StoreItem {
        id: "github",
        name: "GitHub",
        desc: "API GitHub : repos, issues, PR (GITHUB_TOKEN requis en env)",
        cat: "Dev",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-github"],
    },
    StoreItem {
        id: "memory",
        name: "Memory",
        desc: "Graph de connaissances persistant (mémoire longue)",
        cat: "Utilitaires",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-memory"],
    },
    StoreItem {
        id: "seqthink",
        name: "Sequential Thinking",
        desc: "Raisonnement structuré étape par étape",
        cat: "Utilitaires",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-sequential-thinking"],
    },
    StoreItem {
        id: "everything",
        name: "Everything",
        desc: "Serveur de démo/test qui expose tous les comportements MCP",
        cat: "Utilitaires",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-everything"],
    },
    StoreItem {
        id: "time",
        name: "Time",
        desc: "Heure et conversions de fuseaux horaires",
        cat: "Utilitaires",
        kind: "python",
        program: "uvx",
        args: &["mcp-server-time"],
    },
    StoreItem {
        id: "fetch",
        name: "Fetch",
        desc: "Récupération de pages web et conversion en markdown",
        cat: "Web",
        kind: "python",
        program: "uvx",
        args: &["mcp-server-fetch"],
    },
    StoreItem {
        id: "puppeteer",
        name: "Puppeteer",
        desc: "Automatisation de navigateur : screenshots, clics, scraping",
        cat: "Web",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-puppeteer"],
    },
    StoreItem {
        id: "brave",
        name: "Brave Search",
        desc: "Recherche web Brave (BRAVE_API_KEY requis en env)",
        cat: "Web",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-brave-search"],
    },
    StoreItem {
        id: "postgres",
        name: "PostgreSQL",
        desc: "Accès lecture seule à une base PostgreSQL (URL en args)",
        cat: "Données",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-postgres"],
    },
    StoreItem {
        id: "sqlite",
        name: "SQLite",
        desc: "Requêtes et analyse d'une base SQLite locale",
        cat: "Données",
        kind: "python",
        program: "uvx",
        args: &["mcp-server-sqlite", "--db-path", "test.db"],
    },
    StoreItem {
        id: "slack",
        name: "Slack",
        desc: "Lecture/écriture de canaux Slack (tokens requis en env)",
        cat: "Données",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-slack"],
    },
    StoreItem {
        id: "playwright",
        name: "Playwright",
        desc: "Automatisation de navigateur moderne (Microsoft) : navigation, clics, screenshots, snapshots d'accessibilité",
        cat: "Web",
        kind: "npm",
        program: "npx",
        args: &["-y", "@playwright/mcp@latest"],
    },
    StoreItem {
        id: "context7",
        name: "Context7",
        desc: "Documentation à jour des libs et frameworks injectée dans le contexte de l'agent (Upstash)",
        cat: "Dev",
        kind: "npm",
        program: "npx",
        args: &["-y", "@upstash/context7-mcp"],
    },
    StoreItem {
        id: "duckduckgo",
        name: "DuckDuckGo Search",
        desc: "Recherche web DuckDuckGo sans clé API + extraction de contenu de pages",
        cat: "Web",
        kind: "python",
        program: "uvx",
        args: &["duckduckgo-mcp-server"],
    },
    StoreItem {
        id: "kubernetes",
        name: "Kubernetes",
        desc: "Gestion d'un cluster k8s via kubectl (kubeconfig local requis)",
        cat: "Dev",
        kind: "npm",
        program: "npx",
        args: &["-y", "mcp-server-kubernetes"],
    },
    StoreItem {
        id: "supabase",
        name: "Supabase",
        desc: "Bases, tables et edge functions Supabase (OAuth navigateur au premier probe, ou SUPABASE_ACCESS_TOKEN en env)",
        cat: "Données",
        kind: "npm",
        program: "npx",
        args: &["-y", "@supabase/mcp-server-supabase@latest"],
    },
    StoreItem {
        id: "mongodb",
        name: "MongoDB",
        desc: "Requêtes et analyse MongoDB (MDB_MCP_CONNECTION_STRING requis en env)",
        cat: "Données",
        kind: "npm",
        program: "npx",
        args: &["-y", "mongodb-mcp-server"],
    },
    StoreItem {
        id: "stripe",
        name: "Stripe",
        desc: "Paiements et clients Stripe (STRIPE_SECRET_KEY en env, ou --api-key en args)",
        cat: "Données",
        kind: "npm",
        program: "npx",
        args: &["-y", "@stripe/mcp", "--tools=all"],
    },
    StoreItem {
        id: "notion",
        name: "Notion",
        desc: "API Notion officielle (env OPENAPI_MCP_HEADERS = JSON avec Authorization Bearer + Notion-Version)",
        cat: "Données",
        kind: "npm",
        program: "npx",
        args: &["-y", "@notionhq/mcp-server"],
    },
    StoreItem {
        id: "firecrawl",
        name: "Firecrawl",
        desc: "Scraping et extraction web (FIRECRAWL_API_KEY en env ; mode sans clé disponible côté Firecrawl)",
        cat: "Web",
        kind: "npm",
        program: "npx",
        args: &["-y", "firecrawl-mcp"],
    },
    StoreItem {
        id: "exa",
        name: "Exa Search",
        desc: "Recherche web sémantique (EXA_API_KEY requis en env)",
        cat: "Web",
        kind: "npm",
        program: "npx",
        args: &["-y", "exa-mcp-server"],
    },
    StoreItem {
        id: "blender",
        name: "Blender",
        desc: "Pilote une session Blender vivante — addon BlenderMCP requis dans Blender (repo ahujasid/blender-mcp, pin du catalogue Hermes)",
        cat: "Création",
        kind: "python",
        program: "uvx",
        args: &["blender-mcp==1.6.4"],
    },
    StoreItem {
        id: "figma",
        name: "Figma (Framelink)",
        desc: "Contexte design Figma pour générer du code (FIGMA_API_KEY requis en env) — l'officiel distant est OAuth/HTTPS, pas encore sondable ici",
        cat: "Design",
        kind: "npm",
        program: "npx",
        args: &["-y", "figma-developer-mcp", "--stdio"],
    },
    StoreItem {
        id: "n8n",
        name: "n8n",
        desc: "Docs des nodes n8n hors de la boîte ; gestion d'instance via N8N_API_URL + N8N_API_KEY en env",
        cat: "Automatisation",
        kind: "npm",
        program: "npx",
        args: &["-y", "n8n-mcp"],
    },
    StoreItem {
        id: "todoist",
        name: "Todoist",
        desc: "Tâches Todoist officiel (Doist) : projets, tâches, échéances (TODOIST_API_KEY requis en env)",
        cat: "Productivité",
        kind: "npm",
        program: "npx",
        args: &["-y", "@doist/todoist-mcp"],
    },
    StoreItem {
        id: "taskmaster",
        name: "Task Master",
        desc: "Décompose une spec en tâches, gère les dépendances et délègue aux agents (task-master-ai ; ANTHROPIC_API_KEY requis en env)",
        cat: "Productivité",
        kind: "npm",
        program: "npx",
        args: &["-y", "--package=task-master-ai", "task-master-ai"],
    },
];

/// Item de boutique par id — principal, puis aitmpl, puis glama.
fn find_store_item(sid: &str) -> Option<&'static StoreItem> {
    STORE
        .iter()
        .find(|s| s.id == sid)
        .or_else(|| {
            crate::store_aitmpl::STORE_AITMPL
                .iter()
                .find(|s| s.id == sid)
        })
        .or_else(|| crate::store_glama::STORE_GLAMA.iter().find(|s| s.id == sid))
        .or_else(|| {
            crate::store_github::STORE_GITHUB
                .iter()
                .find(|s| s.id == sid)
        })
}

/// Tous les items (principal + aitmpl + glama).
fn all_store_items() -> impl Iterator<Item = &'static StoreItem> {
    STORE
        .iter()
        .chain(crate::store_aitmpl::STORE_AITMPL.iter())
        .chain(crate::store_glama::STORE_GLAMA.iter())
        .chain(crate::store_github::STORE_GITHUB.iter())
}

pub fn store_json() -> Value {
    json!(all_store_items()
        .map(|s| json!({
            "id": s.id, "name": s.name, "desc": s.desc,
            "cat": s.cat, "kind": s.kind,
            "program": s.program, "args": s.args,
        }))
        .collect::<Vec<_>>())
}

/// Porté de mcp-import.ts : "@modelcontextprotocol/server-filesystem" → "filesystem".
fn cleanup_name(base: &str) -> String {
    let mut b = base.to_string();
    if let Some(slash) = b.rfind('/') {
        b = b[slash + 1..].to_string();
    }
    b = regex_like_strip(&b, &["cjs", "js", "mjs", "py", "ts"]);
    for p in ["mcp-server-", "server-", "mcp-"] {
        if b.starts_with(p) {
            b = b[p.len()..].to_string();
        }
    }
    for s in ["-mcp-server", "-mcp", "-server"] {
        if b.ends_with(s) {
            b = b[..b.len() - s.len()].to_string();
        }
    }
    if b.is_empty() {
        base.to_string()
    } else {
        b
    }
}

fn regex_like_strip(name: &str, exts: &[&str]) -> String {
    for e in exts {
        if let Some(stripped) = name.strip_suffix(&format!(".{}", e)) {
            return stripped.to_string();
        }
    }
    name.to_string()
}

/// Porté de mcp-doc.ts : accepte {"mcpServers": {...}}, {"mcp_servers": {...}}
/// ou une map brute name → entry. Retourne (nom, entry).
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

fn entry_to_server(id: String, name: &str, entry: &Value) -> Option<Server> {
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
            tools_disabled: vec![],
        });
    }
    None
}

fn parse_url_parts(u: &str) -> Option<(String, u16, String)> {
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

fn push_imported(state: &Arc<AppState>, name: String, program: String, args: Vec<String>) -> Value {
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
fn name_from_url(host: &str) -> String {
    let labels: Vec<&str> = host.split('.').filter(|l| !l.is_empty()).collect();
    let mut it: Vec<&str> = labels.to_vec();
    while it.len() > 1 && ["api", "mcp", "www"].contains(&it[0]) {
        it.remove(0);
    }
    it.first()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "server".to_string())
}

/// Nombre de tools actifs (tools_disabled écartés).
pub fn tools_actifs(s: &Server) -> usize {
    s.tools
        .iter()
        .filter(|t| !s.tools_disabled.contains(&t.name))
        .count()
}

/// Nombre de sondes menées en parallèle par cycle. Les serveurs lents (SSH,
/// process MCP muet) ne doivent pas retarder tout le monde : avec 30 serveurs
/// dont 15 en SSH, un cycle séquentiel dépassait largement le refresh de 15 s.
const POLL_WIDTH: usize = 8;

/// Lit une annonce du registre litemcp (`%TEMP%/litemcp-registry/<nom>.json`).
/// Écrites par les serveurs litemcp intégrés (ex : moltbook) au bind réussi —
/// contient exe/args/cwd/port : tout pour Start/Stop natifs sans scan ni
/// saisie manuelle. Aucune erreur : fichier absent/corrompu = annonce absente.
fn read_litemcp_registry(name: &str) -> Option<Value> {
    let mut base = std::env::temp_dir();
    if base.as_os_str().is_empty() {
        base = std::path::PathBuf::from("/tmp");
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return None;
    }
    let path = base.join("litemcp-registry").join(format!("{name}.json"));
    let body = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&body).ok()
}

/// Applique une annonce litemcp à l'état : création si aucun serveur HTTP
/// LOCAL du même nom n'existe (exe/args/cwd deviennent la commande de
/// lancement → Start/Stop/autoboot natifs), sinon resync du port et
/// **adoption** — un serveur découvert par scan (sans commande) reçoit celui
/// de l'annonce et est promu `manual` (un discovered n'est pas persisté, la
/// commande serait perdue au reboot). Un serveur avec déjà une commande la
/// garde : l'annonce ne réécrit jamais un choix de l'utilisateur.
/// Renvoie true si l'état a changé.
fn apply_announcement(state: &Arc<AppState>, ann: &Value) -> bool {
    let port = ann.get("port").and_then(|x| x.as_u64()).unwrap_or(0) as u16;
    let exe = ann.get("exe").and_then(|x| x.as_str()).unwrap_or("");
    if port == 0 || exe.is_empty() {
        return false;
    }
    let args: Vec<String> = ann
        .get("args")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|i| i.as_str().map(String::from))
                .skip(1) // args[0] = chemin de l'exe lui-même
                .collect()
        })
        .unwrap_or_default();
    let workdir = ann.get("cwd").and_then(|x| x.as_str()).map(String::from);
    let label = ann
        .get("name")
        .and_then(|x| x.as_str())
        .unwrap_or("litemcp")
        .to_string();
    let version = ann
        .get("version")
        .and_then(|x| x.as_str())
        .map(String::from);
    let ts = ann.get("ts").and_then(|x| x.as_u64()).unwrap_or(0);

    let mut servers = lockv(&state.servers);
    let existing = servers.iter_mut().find(|s| {
        matches!(&s.transport, Transport::Http { host: None, .. })
            && s.label.eq_ignore_ascii_case(&label)
    });
    match existing {
        Some(srv) => {
            let mut changed = false;
            // resync du port si le serveur a changé de port d'écoute
            if let Transport::Http {
                port: ref mut p, ..
            } = srv.transport
            {
                if *p != port {
                    add_log(
                        state,
                        "info",
                        &format!("[litemcp] {} : port {} -> {} (annonce)", srv.id, *p, port),
                    );
                    *p = port;
                    changed = true;
                }
            }
            // adoption : découvert par scan sans commande → commande native
            if srv.start_cmd.is_none() {
                srv.start_cmd = Some(StartCmd {
                    program: exe.to_string(),
                    args,
                    workdir,
                });
                // un discovered n'est pas persisté : promotion en manual pour
                // que la commande adoptée survive au redémarrage
                srv.origin = Origin::Manual;
                if srv.framework.is_none() {
                    srv.framework = Some("litemcp".into());
                }
                if version.is_some() && srv.version.is_none() {
                    srv.version = version;
                }
                add_log(
                    state,
                    "info",
                    &format!(
                        "[litemcp] adoption : {} reçoit sa commande de lancement",
                        srv.id
                    ),
                );
                changed = true;
            } else if version.is_some() && srv.version.is_none() {
                srv.version = version;
                changed = true;
            }
            let seen = srv.last_seen_ts.unwrap_or(0);
            if ts > seen {
                srv.last_seen_ts = Some(ts);
                changed = true;
            }
            changed
        }
        None => {
            let id = format!("litemcp-{label}");
            add_log(
                state,
                "info",
                &format!("[litemcp] import natif : {} (port {port})", label),
            );
            servers.push(Server {
                id,
                label,
                transport: Transport::Http {
                    host: None,
                    port,
                    path: "/mcp".into(),
                },
                origin: Origin::Manual,
                enabled: true,
                state: SState::Down,
                pid: None,
                bound_host: None,
                server_name: None,
                version,
                protocol: None,
                sse: false,
                sse_endpoint: false,
                ws: false,
                lang: Some("rust".into()),
                framework: Some("litemcp".into()),
                via_ssh: false,
                tools: vec![],
                latency_ms: None,
                last_seen_ts: Some(ts),
                last_error: None,
                start_cmd: Some(StartCmd {
                    program: exe.to_string(),
                    args,
                    workdir,
                }),
                auto_restart: true,
                tools_disabled: vec![],
            });
            true
        }
    }
}

/// Import auto des serveurs litemcp annoncés (`%TEMP%/litemcp-registry/`).
/// Appelé à chaque cycle de `poll_all` : le CC et le serveur litemcp restent
/// synchronisés (port, commande, vie) sans intervention.
pub fn import_litemcp_registry(state: &Arc<AppState>) {
    let names = ["moltbook"];
    for name in names {
        let Some(ann) = read_litemcp_registry(name) else {
            continue;
        };
        if apply_announcement(state, &ann) {
            save_config(state);
        }
    }
}

pub fn poll_all(state: &Arc<AppState>) {
    // Import natif des serveurs litemcp (moltbook) : l'annonce écrite au bind
    // fournit exe/args/cwd/port — gestion Start/Stop/autoboot native, même
    // hors plage de scan (moltbook 8787 est hors 3000-3200).
    import_litemcp_registry(state);
    // priorité : l'ordre choisi par glisser-déposer gouverne l'ordre de sondage
    // (utile au boot où le premier cycle est séquentiel) — ids absents en dernier
    let order = lockv(&state.settings).server_order.clone();
    let mut ids: Vec<String> = lockv(&state.servers).iter().map(|s| s.id.clone()).collect();
    ids.sort_by_key(|id| order.iter().position(|x| x == id).unwrap_or(usize::MAX));
    // Sondes en parallèle par paquets de POLL_WIDTH. Chaque poll_one prend ses
    // verrous brièvement et relâche avant d'écrire au journal ; les spawns
    // concurrents restent protégés par `state.spawning`. Résultat : un cycle
    // tient dans le refresh même avec des serveurs distants lents.
    for paquet in ids.chunks(POLL_WIDTH) {
        std::thread::scope(|scope| {
            for id in paquet {
                scope.spawn(move || poll_one(state, id.as_str()));
            }
        });
    }
    save_cache(state);
}

// ---------- démarrage automatique avec l'OS (bouton ON/OFF de la barre de titre) ----------

/// Clé/valeur du registre Run (Windows) ou fichiers autostart (Linux/macOS).
#[cfg_attr(not(windows), allow(dead_code))] // utilisé par le registre Run (Windows)
const AUTOSTART_RUN_NAME: &str = "MCP Control";

/// Arguments de l'entrée de démarrage automatique (compilé hors tests seulement :
/// en build test le corps réel est neutralisé, la fonction deviendrait du code mort).
#[cfg(not(test))]
fn autostart_entry_args() -> String {
    let exe = std::env::current_exe()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    format!("\"{}\" --hidden", exe)
}

/// L'entrée de démarrage automatique existe-t-elle actuellement ?
pub fn autostart_active() -> bool {
    #[cfg(windows)]
    {
        let mut c = Command::new("reg");
        probe::probe_hidden_cmd(&mut c);
        c.args([
            "query",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/v",
            AUTOSTART_RUN_NAME,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        autostart_file_path().map(|p| p.exists()).unwrap_or(false)
    }
}

/// Applique la préférence : crée ou supprime l'entrée de démarrage automatique.
/// Windows : registre HKCU …\Run. Linux : ~/.config/autostart/*.desktop.
/// macOS : ~/Library/LaunchAgents/*.plist.
pub fn apply_autostart(state: &Arc<AppState>, enable: bool) {
    // les tests ne doivent JAMAIS toucher le registre/fichiers de l'OS réels
    #[cfg(test)]
    {
        let _ = (state, enable);
    }
    // corps réel séparé : le garde cfg(test) ne laisse rien d'inatteignable
    // (l'ancien `return` en build test déclenchait unreachable_code — clippy)
    #[cfg(not(test))]
    apply_autostart_os(state, enable);
}

/// Corps réel de l'alignement autostart (compilé hors tests seulement).
#[cfg(not(test))]
#[allow(unused_must_use)]
fn apply_autostart_os(state: &Arc<AppState>, enable: bool) {
    #[cfg_attr(not(windows), allow(unused_variables))] // lu dans le bloc cfg(windows)
    let args = autostart_entry_args();
    #[cfg(windows)]
    {
        let root = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        let mut c = Command::new("reg");
        probe::probe_hidden_cmd(&mut c);
        let status = if enable {
            c.args([
                "add",
                root,
                "/v",
                AUTOSTART_RUN_NAME,
                "/t",
                "REG_SZ",
                "/d",
                &args,
                "/f",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        } else {
            c.args(["delete", root, "/v", AUTOSTART_RUN_NAME, "/f"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        };
        if status.map(|s| s.success()).unwrap_or(false) {
            add_log(
                state,
                "info",
                &format!(
                    "[autostart] entrée de démarrage {}",
                    if enable { "créée" } else { "supprimée" }
                ),
            );
        } else {
            add_log(
                state,
                "warn",
                "[autostart] reg a échoué — l'entrée n'a pas changé",
            );
        }
    }
    #[cfg(not(windows))]
    {
        let Some(path) = autostart_file_path() else {
            return;
        };
        if enable {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir);
            }
            let exe = std::env::current_exe()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            let content = if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                format!(
                    "[Desktop Entry]
Type=Application
Name=MCP Control
Exec=\"{}\" --hidden
Hidden=false
X-GNOME-Autostart-enabled=true
",
                    exe
                )
            } else {
                format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\"><dict>
<key>Label</key><string>com.deamondev.mcp-control</string>
<key>ProgramArguments</key><array>
<string>{}</string><string>--hidden</string>
</array>
<key>RunAtLoad</key><true/>
</dict></plist>
",
                    exe
                )
            };
            std::fs::write(&path, content);
            add_log(state, "info", "[autostart] entrée de démarrage créée");
        } else if path.exists() {
            std::fs::remove_file(&path);
            add_log(state, "info", "[autostart] entrée de démarrage supprimée");
        }
    }
}

/// Chemin du fichier autostart (Linux : XDG ; macOS : LaunchAgent). Windows :
/// registre — None (géré par reg).
#[cfg(not(windows))]
fn autostart_file_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    #[cfg(target_os = "macos")]
    {
        Some(
            std::path::PathBuf::from(home)
                .join("Library/LaunchAgents/com.deamondev.mcp-control.plist"),
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        Some(std::path::PathBuf::from(home).join(".config/autostart/mcp-control-desktop.desktop"))
    }
}

/// Au boot : aligne l'entrée sur la préférence persistée (créée au premier
/// lancement — settings.autostart vaut true par défaut).
pub fn sync_autostart_at_boot(state: &Arc<AppState>) {
    let want = lockv(&state.settings).autostart;
    if autostart_active() != want {
        apply_autostart(state, want);
    }
}

// ---------- vérification de mise à jour (release GitHub) ----------

/// Comparaison numérique par segments : « 0.10.0 » > « 0.9.9 » (pas lexicographique).
/// Retourne vrai si `latest` est STRICTEMENT plus récente que `current`.
pub fn version_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim()
            .trim_start_matches('v')
            .split('.')
            .map(|p| p.trim().parse::<u64>().unwrap_or(0))
            .collect()
    };
    let (a, b) = (parse(latest), parse(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return x > y;
        }
    }
    false
}

/// Interroge la dernière release GitHub. `curl` d'abord (léger, présent sur
/// macOS/Linux/Windows 10+) avec repli PowerShell — l'ancien code n'utilisait
/// QUE PowerShell : la vérification était silencieusement inopérante partout
/// ailleurs (audit 2026-10-24). `Ok(Some(tag))` = tag lu ; `Ok(None)` = réponse
/// sans tag ; `Err` = réseau/API/outil indisponible — la distinction permet au
/// bouton « Vérifier » de ne plus prétendre « à jour » quand le check a raté.
fn github_latest_tag() -> Result<Option<String>, &'static str> {
    let mut c = Command::new("curl");
    probe::probe_hidden_cmd(&mut c);
    if let Ok(out) = c
        .args([
            "-fsSL",
            "-m",
            "10",
            "-H",
            "User-Agent: mcp-control",
            "https://api.github.com/repos/DeamonDev888/mcp-control/releases/latest",
        ])
        .output()
    {
        if out.status.success() {
            if let Ok(v) = serde_json::from_slice::<Value>(&out.stdout) {
                if let Some(tag) = v.get("tag_name").and_then(|x| x.as_str()) {
                    return Ok(Some(tag.to_string()));
                }
            }
        }
    }
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-Command",
        "(Invoke-RestMethod -TimeoutSec 10 -Headers @{ 'User-Agent' = 'mcp-control' }           'https://api.github.com/repos/DeamonDev888/mcp-control/releases/latest').tag_name",
    ]);
    probe::probe_hidden_cmd(&mut cmd);
    let Ok(out) = cmd.output() else {
        return Err("réseau injoignable (curl et PowerShell en échec)");
    };
    if !out.status.success() {
        return Err("réseau injoignable (curl et PowerShell en échec)");
    }
    let tag = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if tag.is_empty() || tag.len() > 30 {
        return Ok(None);
    }
    Ok(Some(tag))
}

/// Issue d'une vérification de release GitHub — distinguée pour que le bouton
/// « Vérifier » des réglages puisse dire « à jour (vérifié à l'instant) »,
/// « vX disponible » ou « vérification impossible (réseau ?) » au lieu du
/// silence d'avant, qui faisait croire à un clic mort.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheck {
    /// Release plus récente lue et enregistrée dans update_latest.
    Available(String),
    /// API GitHub lue : rien de plus récent que la version installée.
    UpToDate,
    /// Réseau/API injoignable ou réponse illisible — aucun verdict.
    Unreachable,
}

/// Vérifie la dernière release GitHub : `update_latest` = tag SI plus récent
/// que l'installée ; vidé si l'installée redevient la dernière (sinon le badge
/// « mise à jour » resterait affiché avec une dispo périmée). Côté journal,
/// silence hors ligne — le poller horaire ne doit pas spammer : le verdict est
/// RENVOYÉ et l'action `check_update` le traduit pour l'UI.
pub fn check_update(state: &Arc<AppState>) -> UpdateCheck {
    let tag = match github_latest_tag() {
        Ok(Some(t)) => t,
        Ok(None) | Err(_) => return UpdateCheck::Unreachable,
    };
    let current = env!("CARGO_PKG_VERSION");
    if version_newer(&tag, current) {
        // VERROUILLAGE (audit 2026-10-25) : ne PAS tenir update_latest pendant
        // add_log (qui prend le verrou log). state_json prend log PUIS
        // update_latest → un AB-BA était possible : check_update tenait
        // update_latest et attendait log pendant que state_json tenait log et
        // attendait update_latest. On libère update_latest AVANT de logger.
        let should_log = {
            let mut u = lockv(&state.update_latest);
            if u.as_deref() != Some(tag.as_str()) {
                *u = Some(tag.clone());
                true
            } else {
                false
            }
        };
        if should_log {
            add_log(
                state,
                "info",
                &format!(
                    "mise à jour disponible : v{} (installée : v{}) — github.com/DeamonDev888/mcp-control/releases",
                    tag, current
                ),
            );
        }
        return UpdateCheck::Available(tag);
    }
    // L'installée redevient la dernière : on vide le badge.
    {
        let mut u = lockv(&state.update_latest);
        if u.is_some() {
            *u = None;
        }
    }
    UpdateCheck::UpToDate
}

// ---------- cache de démarrage (dernier état connu) ----------

/// state-cache.json à côté du config.json.
fn cache_path_of(config_path: &str) -> String {
    std::path::Path::new(config_path)
        .with_file_name("state-cache.json")
        .to_string_lossy()
        .to_string()
}

/// Dernier cache sérialisé (écriture skippée si inchangé — 1 cycle = 0 I/O).
fn cache_last_slot() -> &'static std::sync::Mutex<Option<String>> {
    static SLOT: std::sync::OnceLock<std::sync::Mutex<Option<String>>> = std::sync::OnceLock::new();
    SLOT.get_or_init(|| std::sync::Mutex::new(None))
}

/// Enregistre l'état courant (champs d'affichage : nom, version, tools, via_ssh…)
/// dans le cache de démarrage. Écriture atomique, skippée si rien n'a changé.
pub fn save_cache(state: &Arc<AppState>) {
    if state.config_path.is_empty() {
        return;
    }
    let path = cache_path_of(&state.config_path);
    let servers = lockv(&state.servers);
    let arr: Vec<Value> = servers
        .iter()
        .map(|s| {
            json!({
                "id": s.id, "state": s.state.as_str(),
                "server_name": s.server_name, "version": s.version, "protocol": s.protocol,
                "sse": s.sse, "sse_endpoint": s.sse_endpoint, "ws": s.ws, "lang": s.lang,
                "framework": s.framework, "via_ssh": s.via_ssh,
                "latency_ms": s.latency_ms, "last_seen": s.last_seen_ts,
                "last_error": s.last_error,
                "tools": s.tools.iter()
                    .map(|t| json!({"name": t.name, "desc": t.desc}))
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    drop(servers);
    let doc = json!({
        "app": env!("CARGO_PKG_VERSION"),
        "saved_at": crate::types::now_ts(),
        "servers": arr,
    });
    let body = serde_json::to_string(&doc).unwrap_or_default();
    {
        let mut slot = cache_last_slot().lock().unwrap_or_else(|p| p.into_inner());
        if slot.as_deref() == Some(body.as_str()) {
            return;
        }
        *slot = Some(body.clone());
    }
    let tmp = format!("{}.tmp", path);
    if let Err(e) = std::fs::write(&tmp, &body) {
        add_log(
            state,
            "warn",
            &format!("cache : écriture impossible ({}): {}", tmp, e),
        );
        return;
    }
    let _ = std::fs::rename(&tmp, &path);
}

/// Au boot, avant la première sonde : restaure les champs d'affichage des
/// serveurs connus (par id, serveurs activés seulement) — l'UI est peuplée
/// instantanément, les sondes confirment/corrigent en arrière-plan.
pub fn load_cache(state: &Arc<AppState>) {
    if state.config_path.is_empty() {
        return;
    }
    let path = cache_path_of(&state.config_path);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    let Some(arr) = v.get("servers").and_then(|x| x.as_array()) else {
        return;
    };
    let mut applied = 0usize;
    let mut servers = lockv(&state.servers);
    for e in arr {
        let Some(id) = e.get("id").and_then(|x| x.as_str()) else {
            continue;
        };
        let Some(srv) = servers.iter_mut().find(|s| s.id == id) else {
            continue;
        };
        if !srv.enabled {
            continue;
        }
        srv.state = match e.get("state").and_then(|x| x.as_str()).unwrap_or("DOWN") {
            "UP" => SState::Up,
            "PORT-ONLY" => SState::PortOnly,
            "DISABLED" => SState::Disabled,
            _ => SState::Down,
        };
        let get_str = |k: &str| e.get(k).and_then(|x| x.as_str()).map(String::from);
        srv.server_name = get_str("server_name").or(srv.server_name.take());
        srv.version = get_str("version").or(srv.version.take());
        srv.protocol = get_str("protocol").or(srv.protocol.take());
        srv.sse = e.get("sse").and_then(|x| x.as_bool()).unwrap_or(srv.sse);
        srv.sse_endpoint = e
            .get("sse_endpoint")
            .and_then(|x| x.as_bool())
            .unwrap_or(srv.sse_endpoint);
        srv.ws = e.get("ws").and_then(|x| x.as_bool()).unwrap_or(srv.ws);
        srv.lang = get_str("lang").or(srv.lang.take());
        srv.framework = get_str("framework").or(srv.framework.take());
        srv.via_ssh = e
            .get("via_ssh")
            .and_then(|x| x.as_bool())
            .unwrap_or(srv.via_ssh);
        srv.latency_ms = e
            .get("latency_ms")
            .and_then(|x| x.as_u64())
            .or(srv.latency_ms.take());
        srv.last_seen_ts = e
            .get("last_seen")
            .and_then(|x| x.as_u64())
            .or(srv.last_seen_ts.take());
        srv.last_error = get_str("last_error").or(srv.last_error.take());
        if let Some(tools) = e.get("tools").and_then(|x| x.as_array()) {
            if srv.tools.is_empty() {
                srv.tools = tools
                    .iter()
                    .filter_map(|t| {
                        Some(crate::types::ToolInfo {
                            name: t.get("name")?.as_str()?.to_string(),
                            desc: t
                                .get("desc")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                        })
                    })
                    .collect();
            }
        }
        applied += 1;
    }
    drop(servers);
    if applied > 0 {
        add_log(
            state,
            "info",
            &format!(
                "cache de démarrage : {} serveur(s) restauré(s) — les sondes confirment en arrière-plan",
                applied
            ),
        );
    }
}

/// Filesystem : le dossier exposé doit être présent — il faut au moins les
/// options + le package + UN chemin (le nom du package contient un « / », la
/// longueur est donc le critère fiable), et le dernier arg doit ressembler à un chemin.
fn filesystem_dossier_manquant(args: &[String]) -> bool {
    args.len() < 3
        || args
            .last()
            .map(|a| !a.contains('/') && !a.contains('\\'))
            .unwrap_or(true)
}

/// Compte un échec de spawn auto pour l'id ; à partir de 3 échecs consécutifs,
/// désactive l'auto-restart (sinon boucle infinie d'essais + warnings sur une
/// machine cliente où les chemins des seeds n'existent pas).
/// Enregistre un événement runtime (crash, redémarrage, succès…) et le
/// persiste — l'historique survit au redémarrage de l'application.
/// NOTIFICATION d'incident : déclenchée sur une transition UP → DOWN/PORT-ONLY.
/// Deux canaux : toast desktop (Windows) et webhook HTTP. Le délai minimum
/// (`min_interval_secs`) évite le spam quand un serveur oscille.
fn notify_incident(app: &Arc<AppState>, label: &str, new_state: &str, detail: &str) {
    let cfg = lockv(&app.settings).notify.clone();
    if !cfg.desktop && cfg.webhook_url.trim().is_empty() {
        return;
    }
    if !cfg.wants(new_state) {
        return;
    }
    // anti-spam : pas deux notifications du même serveur dans l'intervalle
    let now = crate::types::now_ts();
    {
        let mut rt = lockv(&app.runtime);
        let entry = rt.entry(format!("notify:{label}")).or_default();
        if entry.last_attempt_ts != 0
            && now.saturating_sub(entry.last_attempt_ts) < cfg.min_interval_secs
        {
            return;
        }
        entry.last_attempt_ts = now;
    }
    if cfg.desktop {
        let title = format!("MCP — {label}");
        let msg = format!("{new_state} : {detail}");
        // un toast raté ne doit JAMAIS faire tomber le poller
        if let Err(e) = crate::notify::desktop_toast(&title, &msg) {
            add_log(app, "warn", &format!("[notif] desktop : {e}"));
        }
    }
    if !cfg.webhook_url.trim().is_empty() {
        match crate::notify::send_webhook(&cfg, label, new_state, detail) {
            Ok(()) => add_log(
                app,
                "info",
                &format!("[notif] webhook envoyé : {label} {new_state}"),
            ),
            Err(e) => add_log(app, "warn", &format!("[notif] webhook : {e}")),
        }
    }
}
fn runtime_event(state: &AppState, id: &str, kind: &str, detail: &str) {
    let label = lockv(&state.servers)
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.label.clone())
        .unwrap_or_else(|| id.to_string());
    {
        let mut rt = lockv(&state.runtime);
        let entry = rt.entry(id.to_string()).or_default();
        match kind {
            "crash" => entry.crashes += 1,
            "restart" | "recovered" => entry.restarts += 1,
            _ => {}
        }
        if kind == "recovered" || kind == "start-ok" {
            entry.fails = 0;
            entry.last_success_ts = crate::types::now_ts();
        }
        entry.push_event(crate::runtime::RuntimeEvent {
            ts: crate::types::now_ts(),
            id: id.to_string(),
            label: label.clone(),
            kind: kind.to_string(),
            detail: detail.to_string(),
        });
    }
    save_config(state);
}
fn note_spawn_failure(state: &AppState, id: &str) {
    let (fails, backoff, disable) = {
        let mut rt = lockv(&state.runtime);
        let entry = rt.entry(id.to_string()).or_default();
        entry.fails += 1;
        entry.last_attempt_ts = crate::types::now_ts();
        let label = lockv(&state.servers)
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.label.clone())
            .unwrap_or_else(|| id.to_string());
        entry.push_event(crate::runtime::RuntimeEvent {
            ts: crate::types::now_ts(),
            id: id.to_string(),
            label: label.clone(),
            kind: "start-fail".into(),
            detail: format!("{} échec(s) consécutif(s)", entry.fails),
        });
        (
            entry.fails,
            entry.backoff_secs(),
            crate::runtime::should_disable_autoboot(entry.fails),
        )
    };
    if disable {
        {
            let mut servers = lockv(&state.servers);
            if let Some(srv) = servers.iter_mut().find(|s| s.id == id) {
                srv.auto_restart = false;
            }
        }
        add_log(
            state,
            "warn",
            &format!(
                "[{}] auto-restart désactivé après {} échecs de démarrage consécutifs",
                id, fails
            ),
        );
    } else {
        add_log(
            state,
            "warn",
            &format!(
                "[{}] échec de démarrage #{} — nouvelle tentative dans {} s",
                id, fails, backoff
            ),
        );
    }
    // PERSISTANCE : le compteur d'échecs et le backoff doivent survivre au
    // redémarrage de l'app (sinon la boucle d'échecs repart de zéro).
    save_config(state);
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
fn discovery_candidates(ports: &[u16], known_local: &[u16], own_ui: u16) -> Vec<u16> {
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
fn merge_duplicate_servers(state: &Arc<AppState>) -> usize {
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
fn prune_dead_network_servers(state: &Arc<AppState>) {
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

// ---------- découverte des pairs MCP Control ----------

#[allow(unused_must_use)]
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

// ---------- actions ----------

#[allow(unused_must_use)]
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

fn now_compact() -> u64 {
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

fn server_json(s: &Server, managed: bool) -> Value {
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
fn managed_ids(state: &Arc<AppState>) -> std::collections::HashSet<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AppState, Settings};
    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[test]
    fn store_catalogue_sain() {
        // ids et commandes uniques (install_store clé les serveurs sur "store-<id>"),
        // kinds et programmes connus
        let mut ids = std::collections::HashSet::new();
        let mut cmds: std::collections::HashMap<(&str, Vec<&str>), &str> =
            std::collections::HashMap::new();
        for it in all_store_items() {
            assert!(
                !it.id.is_empty() && !it.name.is_empty() && !it.desc.is_empty(),
                "{}: champs vides",
                it.id
            );
            assert!(ids.insert(it.id), "id store dupliqué : {}", it.id);
            let cmd_key = (it.program, it.args.to_vec());
            if let Some(prev_id) = cmds.insert(cmd_key, it.id) {
                panic!(
                    "commande store dupliquée entre '{}' et '{}' : {} {:?}",
                    prev_id, it.id, it.program, it.args
                );
            }
            assert!(
                it.kind == "npm" || it.kind == "python",
                "{}: kind inconnu {}",
                it.id,
                it.kind
            );
            assert!(
                it.program == "npx" || it.program == "uvx",
                "{}: programme inconnu {}",
                it.id,
                it.program
            );
            assert!(!it.args.is_empty(), "{}: args vides", it.id);
        }
        assert!(
            STORE.len() >= 29,
            "le catalogue principal doit garder les 14 originaux + les populaires"
        );
        assert!(
            !crate::store_aitmpl::STORE_AITMPL.is_empty(),
            "le catalogue aitmpl ne doit pas être vide"
        );
    }

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
    fn config_absente_renvoie_les_seeds() {
        let (servers, st) = load_config(&tmp_config("inexistant_aucune_chance.json", None));
        assert_eq!(servers.len(), 8);
        assert_eq!(servers[0].id, "seed-overmind");
        assert_eq!(st.refresh_secs, 15);
        for id in ["seed-debats", "seed-news-btc", "seed-news"] {
            let s = servers.iter().find(|s| s.id == id).unwrap();
            assert!(!s.enabled, "{} doit etre desactive par defaut", id);
            assert_eq!(s.state, SState::Disabled);
        }
        for id in [
            "seed-overmind",
            "seed-discord",
            "seed-postgresql",
            "seed-x",
            "seed-litemcp",
        ] {
            assert!(servers.iter().find(|s| s.id == id).unwrap().enabled);
        }
    }

    #[test]
    fn seed_retire_durablement_nest_pas_reinjecte() {
        // « Retirer durablement » : settings.removed_seeds filtre les seeds
        // réinjectés par défaut au chargement de la config
        let cfg = "{\"settings\":{\"removed_seeds\":[\"seed-news\"]}}";
        let (servers, st) = load_config(&tmp_config("cfg_seed_retire.json", Some(cfg)));
        assert!(
            servers.iter().all(|s| s.id != "seed-news"),
            "seed-news retiré durablement ne doit pas être réinjecté"
        );
        assert!(
            servers.iter().any(|s| s.id == "seed-litemcp"),
            "les autres seeds restent"
        );
        assert_eq!(st.removed_seeds, vec!["seed-news".to_string()]);
    }

    #[test]
    fn remove_durable_consigne_le_seed_dans_la_config() {
        // Roundtrip complet : action API « remove durable » → config.json
        // → rechargement : le seed n'est plus là
        let path = tmp_config("cfg_remove_durable.json", None);
        let state = Arc::new(AppState {
            servers: Mutex::new(
                default_servers()
                    .into_iter()
                    .filter(|s| s.id == "seed-news")
                    .collect(),
            ),
            log: Mutex::new(VecDeque::new()),
            settings: Mutex::new(Settings::default()),
            children: Mutex::new(std::collections::HashMap::new()),
            config_path: path.clone(),
            peers: Mutex::new(std::collections::HashMap::new()),
            allowed_origins: Mutex::new(Vec::new()),
            allowed_hosts: Mutex::new(Vec::new()),
            spawn_fails: Mutex::new(std::collections::HashMap::new()),
            runtime: Mutex::new(crate::runtime::RuntimeLog::new()),
            spawning: Mutex::new(std::collections::HashSet::new()),
            update_latest: Mutex::new(None),
            lan_ok: Mutex::new(None),
        });
        let j = handle_action(
            &state,
            "{\"action\":\"remove\",\"id\":\"seed-news\",\"durable\":true}",
        );
        assert_eq!(j["ok"], serde_json::json!(true));
        let (servers, st) = load_config(&path);
        assert!(
            servers.iter().all(|s| s.id != "seed-news"),
            "le seed ne doit pas revenir au rechargement"
        );
        assert_eq!(st.removed_seeds, vec!["seed-news".to_string()]);
    }

    #[test]
    fn version_newer_compare_numeriquement() {
        assert!(version_newer("0.7.1", "0.7.0"));
        assert!(version_newer("v0.8.0", "0.7.9"), "préfixe v toléré");
        assert!(
            version_newer("0.10.0", "0.9.9"),
            "numérique, pas lexicographique"
        );
        assert!(version_newer("1.0", "0.9.9"), "longueurs différentes");
        assert!(
            !version_newer("0.6.4", "0.7.0"),
            "la release publiée est plus ancienne"
        );
        assert!(
            !version_newer("0.7.0", "0.7.0"),
            "égale = pas de mise à jour"
        );
        assert!(
            !version_newer("abc", "0.7.0"),
            "tag non numérique = pas plus récent"
        );
    }

    #[test]
    fn autostart_est_persiste_et_expose() {
        let path = tmp_config(
            "cfg_autostart.json",
            Some("{\"settings\":{\"autostart\":false}}"),
        );
        let (_, st) = load_config(&path);
        assert!(!st.autostart, "autostart: false lu depuis la config");
        let state = test_state();
        {
            let mut s = lockv(&state.settings);
            s.autostart = false;
        }
        let d = state_json(&state);
        assert_eq!(d["autostart"], serde_json::json!(false));
        // l'action reflète la valeur demandée — apply_autostart est neutralisé
        // sous cfg(test) (jamais d'écriture OS depuis les tests)
        let j = handle_action(&state, "{\"action\":\"set_autostart\",\"enabled\":true}");
        assert_eq!(j["ok"], serde_json::json!(true));
        assert!(lockv(&state.settings).autostart);
        // NB : j["active"] reflète le registre/fichier RÉEL — non testé (effet OS)
    }

    #[test]
    fn reorder_fixe_l_ordre_d_affichage_et_de_sondage() {
        let path = tmp_config("cfg_reorder.json", None);
        let mk = |ids: &[&str]| {
            Arc::new(AppState {
                servers: Mutex::new(
                    ids.iter()
                        .map(|id| Server {
                            id: id.to_string(),
                            label: id.to_string(),
                            transport: Transport::Http {
                                host: None,
                                port: 1,
                                path: "/mcp".into(),
                            },
                            origin: Origin::Seed,
                            enabled: true,
                            auto_restart: false,
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
                            start_cmd: None,
                            tools_disabled: vec![],
                        })
                        .collect(),
                ),
                log: Mutex::new(VecDeque::new()),
                settings: Mutex::new(Settings::default()),
                children: Mutex::new(std::collections::HashMap::new()),
                config_path: path.clone(),
                peers: Mutex::new(std::collections::HashMap::new()),
                allowed_origins: Mutex::new(Vec::new()),
                allowed_hosts: Mutex::new(Vec::new()),
                spawn_fails: Mutex::new(std::collections::HashMap::new()),
                runtime: Mutex::new(crate::runtime::RuntimeLog::new()),
                spawning: Mutex::new(std::collections::HashSet::new()),
                update_latest: Mutex::new(None),
                lan_ok: Mutex::new(None),
            })
        };
        let state = mk(&["a", "b", "c"]);
        // 1. reorder avec ids inconnus filtrés + ordre appliqué
        let j = handle_action(
            &state,
            "{\"action\":\"reorder\",\"ids\":[\"c\",\"zzz\",\"b\"]}",
        );
        assert_eq!(j["ok"], serde_json::json!(true));
        assert_eq!(
            lockv(&state.settings).server_order,
            vec!["c".to_string(), "b".to_string(), "a".to_string()],
            "inconnu filtré, manquant (a) ajouté en fin"
        );
        // 2. l'affichage suit l'ordre
        let d = state_json(&state);
        let ids: Vec<String> = d["servers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(ids, vec!["c", "b", "a"]);
        // 3. remove purge l'ordre
        handle_action(&state, "{\"action\":\"remove\",\"id\":\"c\"}");
        assert_eq!(
            lockv(&state.settings).server_order,
            vec!["b".to_string(), "a".to_string()]
        );
    }

    #[test]
    fn server_order_est_persiste_et_recharge() {
        let path = tmp_config(
            "cfg_ordre.json",
            Some("{\"settings\":{\"server_order\":[\"b\",\"a\"]}}"),
        );
        let (_, st) = load_config(&path);
        assert_eq!(st.server_order, vec!["b".to_string(), "a".to_string()]);
    }

    #[test]
    fn cache_roundtrip_restaure_le_dernier_etat() {
        let path = tmp_config("cfg_cache.json", None);
        let mk = |state_etat: SState, tools: bool| {
            Arc::new(AppState {
                servers: Mutex::new(vec![Server {
                    id: "seed-overmind".into(),
                    label: "Overmind".into(),
                    transport: Transport::Http {
                        host: None,
                        port: 3099,
                        path: "/mcp".into(),
                    },
                    origin: Origin::Seed,
                    enabled: true,
                    auto_restart: true,
                    state: state_etat,
                    pid: None,
                    bound_host: Some("127.0.0.1".into()),
                    server_name: if tools {
                        Some("OverMind-MCP".into())
                    } else {
                        None
                    },
                    version: if tools { Some("3.10.5".into()) } else { None },
                    protocol: None,
                    sse: false,
                    sse_endpoint: false,
                    ws: false,
                    lang: Some("node".into()),
                    framework: None,
                    via_ssh: false,
                    tools: if tools {
                        vec![
                            crate::types::ToolInfo {
                                name: "a".into(),
                                desc: "premier".into(),
                            },
                            crate::types::ToolInfo {
                                name: "b".into(),
                                desc: String::new(),
                            },
                        ]
                    } else {
                        vec![]
                    },
                    latency_ms: if tools { Some(3) } else { None },
                    last_seen_ts: if tools { Some(1_700_000_000) } else { None },
                    last_error: None,
                    start_cmd: None,
                    tools_disabled: vec![],
                }]),
                log: Mutex::new(VecDeque::new()),
                settings: Mutex::new(Settings::default()),
                children: Mutex::new(std::collections::HashMap::new()),
                config_path: path.clone(),
                peers: Mutex::new(std::collections::HashMap::new()),
                allowed_origins: Mutex::new(Vec::new()),
                allowed_hosts: Mutex::new(Vec::new()),
                spawn_fails: Mutex::new(std::collections::HashMap::new()),
                runtime: Mutex::new(crate::runtime::RuntimeLog::new()),
                spawning: Mutex::new(std::collections::HashSet::new()),
                update_latest: Mutex::new(None),
                lan_ok: Mutex::new(None),
            })
        };
        // 1. état riche -> cache
        save_cache(&mk(SState::Up, true));
        // 2. nouvel état vide (comme au boot) -> load
        let fresh = mk(SState::Down, false);
        load_cache(&fresh);
        let s = lockv(&fresh.servers).first().unwrap().clone();
        assert_eq!(
            s.state,
            SState::Up,
            "l'état UP est restauré avant la première sonde"
        );
        assert_eq!(s.server_name.as_deref(), Some("OverMind-MCP"));
        assert_eq!(s.version.as_deref(), Some("3.10.5"));
        assert_eq!(s.tools.len(), 2);
        assert_eq!(s.tools[0].name, "a");
        assert_eq!(s.latency_ms, Some(3));
        assert_eq!(s.last_seen_ts, Some(1_700_000_000));
        // 3. serveur désactivé : le cache ne le touche pas
        let dis = mk(SState::Down, false);
        {
            let mut sv = lockv(&dis.servers);
            sv.first_mut().unwrap().enabled = false;
        }
        load_cache(&dis);
        let s2 = lockv(&dis.servers).first().unwrap().clone();
        assert_eq!(
            s2.state,
            SState::Down,
            "serveur désactivé non restauré en UP"
        );
    }

    #[test]
    fn seeds_ont_une_commande_de_lancement() {
        let servers = default_servers();
        for s in &servers {
            let cmd = s
                .start_cmd
                .as_ref()
                .unwrap_or_else(|| panic!("{} doit avoir une start_cmd", s.id));
            if s.id == "seed-litemcp" {
                // démo litemcp : binaire Rust release, pas un launcher node
                assert!(
                    cmd.program
                        .ends_with("/litemcp/target/release/examples/hello.exe"),
                    "seed-litemcp doit lancer hello.exe, pas {}",
                    cmd.program
                );
            } else {
                assert_eq!(cmd.program, "node");
                assert!(
                    cmd.args
                        .first()
                        .map(|a| a.ends_with("/bin/launch.cjs"))
                        .unwrap_or(false),
                    "{} doit lancer son bin/launch.cjs",
                    s.id
                );
            }
            assert!(cmd.workdir.is_some(), "{} doit avoir un workdir", s.id);
        }
        let ov = servers.iter().find(|s| s.id == "seed-overmind").unwrap();
        let cmd = ov.start_cmd.as_ref().unwrap();
        assert!(cmd.args[0].ends_with("/Workflow/bin/launch.cjs"));
        assert!(cmd.workdir.as_deref().unwrap().ends_with("/Workflow"));
        let lm = servers.iter().find(|s| s.id == "seed-litemcp").unwrap();
        assert!(
            matches!(&lm.transport, Transport::Http { port: 8080, .. }),
            "seed-litemcp doit viser le port 8080"
        );
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
            Transport::Http { host, port, path } => {
                assert!(host.is_none());
                assert_eq!(*port, 4567);
                assert_eq!(path, "/x");
            }
            _ => panic!("transport http attendu"),
        }
    }

    #[test]
    fn load_config_reconstruit_start_cmd_stdio_persiste() {
        // LE bug v0.1 : un stdio recharge du disque sans start_cmd n'etait jamais demarre au boot
        let cfg = "{\"servers\":[{\"id\":\"man-s\",\"label\":\"S\",\"transport\":\"stdio\",\"program\":\"node\",\"args\":[\"a.js\",\"--p\",\"1\"],\"workdir\":\"C:/w\",\"enabled\":true}]}";
        let (servers, _) = load_config(&tmp_config("cfg_stdio.json", Some(cfg)));
        let s = servers.iter().find(|s| s.id == "man-s").unwrap();
        let cmd = s
            .start_cmd
            .as_ref()
            .expect("start_cmd doit etre reconstruit au load");
        assert_eq!(cmd.program, "node");
        assert_eq!(
            cmd.args,
            vec!["a.js".to_string(), "--p".to_string(), "1".to_string()]
        );
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
            allowed_origins: Mutex::new(Vec::new()),
            allowed_hosts: Mutex::new(Vec::new()),
            spawn_fails: Mutex::new(std::collections::HashMap::new()),
            runtime: Mutex::new(crate::runtime::RuntimeLog::new()),
            spawning: Mutex::new(std::collections::HashSet::new()),
            update_latest: Mutex::new(None),
            lan_ok: Mutex::new(None),
            config_path: path.clone(),
        };
        {
            let mut servers = lockv(&state.servers);
            servers.push(Server {
                id: "rt1".into(),
                label: "RT".into(),
                transport: Transport::Stdio {
                    program: "prog".into(),
                    args: vec!["-x".into()],
                    workdir: Some("W".into()),
                },
                origin: Origin::Manual,
                enabled: true,
                auto_restart: true,
                state: SState::Up,
                pid: Some(42),
                bound_host: None,
                server_name: Some("n".into()),
                version: Some("1".into()),
                protocol: None,
                sse: false,
                sse_endpoint: false,
                ws: false,
                lang: None,
                framework: None,
                via_ssh: false,
                tools: vec![],
                latency_ms: Some(3),
                last_seen_ts: Some(1),
                last_error: None,
                start_cmd: Some(StartCmd {
                    program: "prog".into(),
                    args: vec!["-x".into()],
                    workdir: Some("W".into()),
                }),
                tools_disabled: vec![],
            });
        }
        save_config(&state);
        let (servers, _) = load_config(&path);
        let s = servers
            .iter()
            .find(|s| s.id == "rt1")
            .expect("doit etre persiste");
        match &s.transport {
            Transport::Stdio {
                program,
                args,
                workdir,
            } => {
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
            servers: Mutex::new(vec![Server {
                id: "s".into(),
                label: "S".into(),
                transport: Transport::Http {
                    host: None,
                    port: 1,
                    path: "/mcp".into(),
                },
                origin: Origin::Seed,
                enabled: true,
                auto_restart: true,
                state: SState::Up,
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
                tools: vec![
                    crate::types::ToolInfo {
                        name: "a".into(),
                        desc: String::new(),
                    },
                    crate::types::ToolInfo {
                        name: "b".into(),
                        desc: String::new(),
                    },
                ],
                latency_ms: None,
                last_seen_ts: None,
                last_error: None,
                start_cmd: None,
                tools_disabled: vec![],
            }]),
            log: Mutex::new(VecDeque::new()),
            settings: Mutex::new(Settings::default()),
            children: Mutex::new(std::collections::HashMap::new()),
            peers: Mutex::new(std::collections::HashMap::new()),
            allowed_origins: Mutex::new(Vec::new()),
            allowed_hosts: Mutex::new(Vec::new()),
            spawn_fails: Mutex::new(std::collections::HashMap::new()),
            runtime: Mutex::new(crate::runtime::RuntimeLog::new()),
            spawning: Mutex::new(std::collections::HashSet::new()),
            update_latest: Mutex::new(None),
            lan_ok: Mutex::new(None),
            config_path: String::new(),
        };
        let arc = Arc::new(state);
        let body = serde_json::to_string(
            &serde_json::json!({"action": "toggle_tool", "id": "s", "tool": "a"}),
        )
        .unwrap();
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
            servers: Mutex::new(vec![Server {
                id: "x".into(),
                label: "X".into(),
                transport: Transport::Http {
                    host: None,
                    port: 9,
                    path: "/mcp".into(),
                },
                origin: Origin::Seed,
                enabled: true,
                auto_restart: true,
                state: SState::Up,
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
                tools: vec![
                    crate::types::ToolInfo {
                        name: "a".into(),
                        desc: String::new(),
                    },
                    crate::types::ToolInfo {
                        name: "b".into(),
                        desc: String::new(),
                    },
                    crate::types::ToolInfo {
                        name: "c".into(),
                        desc: String::new(),
                    },
                ],
                latency_ms: None,
                last_seen_ts: None,
                last_error: None,
                start_cmd: None,
                tools_disabled: vec!["a".into(), "c".into()],
            }]),
            log: Mutex::new(VecDeque::new()),
            settings: Mutex::new(Settings::default()),
            children: Mutex::new(std::collections::HashMap::new()),
            peers: Mutex::new(std::collections::HashMap::new()),
            allowed_origins: Mutex::new(Vec::new()),
            allowed_hosts: Mutex::new(Vec::new()),
            spawn_fails: Mutex::new(std::collections::HashMap::new()),
            runtime: Mutex::new(crate::runtime::RuntimeLog::new()),
            spawning: Mutex::new(std::collections::HashSet::new()),
            update_latest: Mutex::new(None),
            lan_ok: Mutex::new(None),
            config_path: String::new(),
        };
        let servers = lockv(&state.servers);
        assert_eq!(tools_actifs(&servers[0]), 1);
    }

    #[test]
    fn state_json_compte_bon_les_stats() {
        let state = AppState {
            servers: Mutex::new(vec![
                Server {
                    id: "a".into(),
                    label: "A".into(),
                    transport: Transport::Http {
                        host: None,
                        port: 1,
                        path: "/mcp".into(),
                    },
                    origin: Origin::Seed,
                    enabled: true,
                    auto_restart: true,
                    state: SState::Up,
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
                    tools: vec![crate::types::ToolInfo {
                        name: "t".into(),
                        desc: String::new(),
                    }],
                    latency_ms: None,
                    last_seen_ts: None,
                    last_error: None,
                    start_cmd: None,
                    tools_disabled: vec![],
                },
                Server {
                    id: "b".into(),
                    label: "B".into(),
                    transport: Transport::Http {
                        host: None,
                        port: 2,
                        path: "/mcp".into(),
                    },
                    origin: Origin::Seed,
                    enabled: false,
                    auto_restart: true,
                    state: SState::Disabled,
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
                    start_cmd: None,
                    tools_disabled: vec![],
                },
                Server {
                    id: "c".into(),
                    label: "C".into(),
                    transport: Transport::Http {
                        host: None,
                        port: 3,
                        path: "/mcp".into(),
                    },
                    origin: Origin::Seed,
                    enabled: true,
                    auto_restart: true,
                    state: SState::PortOnly,
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
                    // tools en cache d'un poll précédent alors que le serveur est PORT-ONLY :
                    // ne doivent PAS gonfler stats.tools
                    tools: vec![
                        crate::types::ToolInfo {
                            name: "t_stale_1".into(),
                            desc: String::new(),
                        },
                        crate::types::ToolInfo {
                            name: "t_stale_2".into(),
                            desc: String::new(),
                        },
                    ],
                    latency_ms: None,
                    last_seen_ts: None,
                    last_error: None,
                    start_cmd: None,
                    tools_disabled: vec![],
                },
            ]),
            log: Mutex::new(VecDeque::new()),
            settings: Mutex::new(Settings::default()),
            children: Mutex::new(std::collections::HashMap::new()),
            peers: Mutex::new(std::collections::HashMap::new()),
            allowed_origins: Mutex::new(Vec::new()),
            allowed_hosts: Mutex::new(Vec::new()),
            spawn_fails: Mutex::new(std::collections::HashMap::new()),
            runtime: Mutex::new(crate::runtime::RuntimeLog::new()),
            spawning: Mutex::new(std::collections::HashSet::new()),
            update_latest: Mutex::new(None),
            lan_ok: Mutex::new(None),
            config_path: String::new(),
        };
        let j = state_json(&Arc::new(state));
        assert_eq!(j["stats"]["total"], serde_json::json!(3));
        assert_eq!(j["stats"]["up"], serde_json::json!(1));
        assert_eq!(j["stats"]["tools"], serde_json::json!(1));
        assert_eq!(j["servers"][1]["state"], serde_json::json!("DISABLED"));
        assert_eq!(j["servers"][1]["origin"], serde_json::json!("seed"));
        assert_eq!(j["servers"][2]["state"], serde_json::json!("PORT-ONLY"));
    }

    #[test]
    fn save_config_chemin_vide_ne_cree_pas_point_tmp() {
        let state = test_state();
        assert!(state.config_path.is_empty());
        save_config(&state);
        assert!(
            !std::path::Path::new(".tmp").exists(),
            "save_config avec config_path vide ne doit jamais écrire '.tmp'"
        );
    }

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            servers: Mutex::new(vec![]),
            log: Mutex::new(VecDeque::new()),
            settings: Mutex::new(Settings::default()),
            children: Mutex::new(std::collections::HashMap::new()),
            config_path: String::new(),
            peers: Mutex::new(std::collections::HashMap::new()),
            allowed_origins: Mutex::new(Vec::new()),
            allowed_hosts: Mutex::new(Vec::new()),
            spawn_fails: Mutex::new(std::collections::HashMap::new()),
            runtime: Mutex::new(crate::runtime::RuntimeLog::new()),
            spawning: Mutex::new(std::collections::HashSet::new()),
            update_latest: Mutex::new(None),
            lan_ok: Mutex::new(None),
        })
    }

    /// Fabrique un serveur HTTP pour les tests de fusion/dé-duplication.
    fn srv_http(
        id: &str,
        name: Option<&str>,
        label: &str,
        host: Option<&str>,
        port: u16,
        origin: Origin,
        seen: Option<u64>,
    ) -> Server {
        Server {
            id: id.into(),
            label: label.into(),
            transport: Transport::Http {
                host: host.map(String::from),
                port,
                path: "/mcp".into(),
            },
            origin,
            enabled: true,
            state: SState::Up,
            pid: None,
            bound_host: None,
            server_name: name.map(String::from),
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
            last_seen_ts: seen,
            last_error: None,
            start_cmd: None,
            auto_restart: false,
            tools_disabled: vec![],
        }
    }

    /// RÉGRESSION audit 2026-10-26 (P1) : la fusion ne doit toucher QUE les
    /// entrées découvertes (origin Discovered) avec un nom annoncé non vide.
    /// Deux serveurs manuels homonymes (ou sans label) sur des ports
    /// différents doivent TOUJOURS survivre — l'ancienne version fusionnait
    /// sur nom+label@hôte sans port et persistait la suppression.
    #[test]
    fn merge_ne_touche_jamais_les_entrees_manuelles_ou_sans_nom() {
        let state = test_state();
        {
            let mut servers = lockv(&state.servers);
            // deux manuels homonymes, ports différents
            servers.push(srv_http(
                "manual-a",
                Some("discord"),
                "discord",
                None,
                3000,
                Origin::Manual,
                Some(100),
            ));
            servers.push(srv_http(
                "manual-b",
                Some("discord"),
                "discord",
                None,
                3001,
                Origin::Manual,
                Some(200),
            ));
            // deux manuels SANS nom annoncé (le bug historique : label vide →
            // clé « @local » → suppression d'une entrée légitime)
            servers.push(srv_http(
                "manual-c",
                None,
                "",
                None,
                3002,
                Origin::Manual,
                Some(300),
            ));
            servers.push(srv_http(
                "manual-d",
                None,
                "",
                None,
                3003,
                Origin::Manual,
                Some(400),
            ));
            // un seed homonyme d'un manuel
            servers.push(srv_http(
                "seed-x",
                Some("discord"),
                "discord",
                None,
                3004,
                Origin::Seed,
                Some(500),
            ));
        }
        let removed = merge_duplicate_servers(&state);
        assert_eq!(
            removed, 0,
            "aucune entrée manuelle/seed ne doit être fusionnée"
        );
        assert_eq!(lockv(&state.servers).len(), 5);
    }

    #[test]
    fn merge_fusionne_les_doublons_resolus_meme_nom_meme_machine() {
        let state = test_state();
        {
            let mut servers = lockv(&state.servers);
            // même serveur découvert deux fois (port changé) : la plus
            // récemment vue gagne, l'ancienne port est un fantôme
            servers.push(srv_http(
                "net-10.0.0.5.3141",
                Some("discord"),
                "10.0.0.5:3141",
                Some("10.0.0.5"),
                3141,
                Origin::Discovered,
                Some(100),
            ));
            servers.push(srv_http(
                "net-10.0.0.5.9999",
                Some("discord"),
                "10.0.0.5:9999",
                Some("10.0.0.5"),
                9999,
                Origin::Discovered,
                Some(200),
            ));
            // même nom mais MACHINE DIFFÉRENTE : deux serveurs distincts
            servers.push(srv_http(
                "net-10.0.0.6.3141",
                Some("discord"),
                "10.0.0.6:3141",
                Some("10.0.0.6"),
                3141,
                Origin::Discovered,
                Some(300),
            ));
            // découvert SANS nom annoncé : jamais fusionné (identité inconnue)
            servers.push(srv_http(
                "net-10.0.0.5.4000",
                None,
                "10.0.0.5:4000",
                Some("10.0.0.5"),
                4000,
                Origin::Discovered,
                Some(400),
            ));
        }
        let removed = merge_duplicate_servers(&state);
        assert_eq!(removed, 1, "seul le doublon même nom+même machine part");
        let servers = lockv(&state.servers);
        assert!(
            servers.iter().any(|s| s.id == "net-10.0.0.5.9999"),
            "la plus récente est gardée"
        );
        assert!(!servers.iter().any(|s| s.id == "net-10.0.0.5.3141"));
        assert!(
            servers.iter().any(|s| s.id == "net-10.0.0.6.3141"),
            "l'autre machine survit"
        );
        assert!(
            servers.iter().any(|s| s.id == "net-10.0.0.5.4000"),
            "sans nom annoncé : jamais fusionné"
        );
    }

    #[test]
    fn lan_decision_couvre_les_cas_vpn_et_panne() {
        // passerelle qui répond, serveurs distants OK → réseau sain
        assert_eq!(lan_decision(Some(true), false), Some(true));
        // passerelle muette → coupé
        assert_eq!(lan_decision(Some(false), false), Some(false));
        // VPN : SA passerelle répond mais tous les distants échouent en réseau
        assert_eq!(lan_decision(Some(true), true), Some(false));
        // pas de passerelle détectée mais distants KO → coupé
        assert_eq!(lan_decision(None, true), Some(false));
        // pas de passerelle et distants OK → indéterminé (pas de LAN à surveiller)
        assert_eq!(lan_decision(None, false), None);
    }

    #[test]
    fn auto_restart_par_serveur_action_et_persistance() {
        // Bouton ⚡ de l'UI : action « set_auto_restart » → flag du serveur,
        // persistance dans config.json, et remise à zéro des échecs de spawn.
        let path = tmp_config("cfg_autorestart.json", Some("{\"servers\":[]}"));
        // état partagé avec un vrai chemin de config pour tester l'écriture
        let mut base = match Arc::try_unwrap(test_state()) {
            Ok(s) => s,
            Err(_) => panic!("test_state doit être unique"),
        };
        base.config_path = path.clone();
        let state = Arc::new(base);
        {
            let mut st = lockv(&state.settings);
            st.autostart = false;
        }
        {
            let mut servers = lockv(&state.servers);
            servers.push(Server {
                id: "srv-a".into(),
                label: "Serveur A".into(),
                transport: Transport::Stdio {
                    program: "node".into(),
                    args: vec![],
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
                start_cmd: Some(StartCmd {
                    program: "node".into(),
                    args: vec![],
                    workdir: None,
                }),
                auto_restart: true,
                tools_disabled: vec![],
            });
        }
        lockv(&state.spawn_fails).insert("srv-a".to_string(), 3);

        // désactivation
        let j = handle_action(
            &state,
            "{\"action\":\"set_auto_restart\",\"id\":\"srv-a\",\"enabled\":false}",
        );
        assert_eq!(j["ok"], serde_json::json!(true));
        assert!(!lockv(&state.servers)[0].auto_restart);
        let cfg = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            cfg.contains("\"auto_restart\": false"),
            "le flag doit être persisté dans config.json : {}",
            cfg
        );

        // réactivation : les échecs de spawn repartent de zéro
        lockv(&state.spawn_fails).insert("srv-a".to_string(), 3);
        let j2 = handle_action(
            &state,
            "{\"action\":\"set_auto_restart\",\"id\":\"srv-a\",\"enabled\":true}",
        );
        assert_eq!(j2["ok"], serde_json::json!(true));
        assert!(lockv(&state.servers)[0].auto_restart);
        assert!(
            lockv(&state.spawn_fails).get("srv-a").is_none(),
            "réactiver l'autoboot doit effacer le compteur d'échecs"
        );

        // serveur inconnu → erreur propre, pas de panic
        let j3 = handle_action(
            &state,
            "{\"action\":\"set_auto_restart\",\"id\":\"inconnu\",\"enabled\":true}",
        );
        assert_eq!(j3["ok"], serde_json::json!(false));
    }

    #[test]
    fn set_host_refuse_seed_et_rehost_manuel() {
        // Explorateur groupé par machine : action « set_host » (drop d'une ligne
        // sur l'en-tête d'un groupe). Un seed est refusé — la config par défaut
        // réimposerait son host au boot —, un serveur manuel est re-hosté et
        // persisté, un stdio est refusé (pas de host), erreurs propres sinon.
        let path = tmp_config("cfg_sethost.json", Some("{\"servers\":[]}"));
        let mut base = match Arc::try_unwrap(test_state()) {
            Ok(s) => s,
            Err(_) => panic!("test_state doit être unique"),
        };
        base.config_path = path.clone();
        let state = Arc::new(base);
        {
            let mut st = lockv(&state.settings);
            st.autostart = false;
        }
        {
            let mut servers = lockv(&state.servers);
            // seed HTTP local (comme un seed du launcher)
            servers.push(Server {
                id: "seed-1".into(),
                label: "Seed".into(),
                transport: Transport::Http {
                    host: None,
                    port: 3099,
                    path: "/mcp".into(),
                },
                origin: Origin::Seed,
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
                start_cmd: None,
                auto_restart: true,
                tools_disabled: vec![],
            });
            // serveur manuel HTTP importé d'une machine (host à déplacer)
            servers.push(Server {
                id: "man-1".into(),
                label: "Manuel".into(),
                transport: Transport::Http {
                    host: Some("192.168.40.9".into()),
                    port: 3141,
                    path: "/mcp".into(),
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
                via_ssh: true,
                tools: vec![],
                latency_ms: None,
                last_seen_ts: None,
                last_error: None,
                start_cmd: None,
                auto_restart: true,
                tools_disabled: vec![],
            });
            // stdio : jamais concerné par un re-host
            servers.push(Server {
                id: "man-2".into(),
                label: "Stdio".into(),
                transport: Transport::Stdio {
                    program: "node".into(),
                    args: vec![],
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
                lang: Some("node".into()),
                framework: None,
                via_ssh: false,
                tools: vec![],
                latency_ms: None,
                last_seen_ts: None,
                last_error: None,
                start_cmd: None,
                auto_restart: true,
                tools_disabled: vec![],
            });
        }

        // 1. re-host d'un seed → refus, host inchangé (reste local/None)
        let j = handle_action(
            &state,
            "{\"action\":\"set_host\",\"id\":\"seed-1\",\"host\":\"10.9.9.9\"}",
        );
        assert_eq!(j["ok"], serde_json::json!(false));
        {
            let servers = lockv(&state.servers);
            let Transport::Http { host, .. } = &servers[0].transport else {
                panic!("transport Http attendu");
            };
            assert_eq!(host.as_deref(), None, "le seed garde host:None (local)");
        }

        // 2. re-host d'un manuel → ok, ancien host rapporté, persisté
        let j2 = handle_action(
            &state,
            "{\"action\":\"set_host\",\"id\":\"man-1\",\"host\":\"10.9.9.9\"}",
        );
        assert_eq!(j2["ok"], serde_json::json!(true));
        assert_eq!(j2["old_host"], serde_json::json!("192.168.40.9"));
        {
            let servers = lockv(&state.servers);
            let Transport::Http { host, .. } = &servers[1].transport else {
                panic!("transport Http attendu");
            };
            assert_eq!(host.as_deref(), Some("10.9.9.9"));
        }
        let cfg = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            cfg.contains("10.9.9.9"),
            "le nouveau host doit être persisté dans config.json : {}",
            cfg
        );

        // 3. stdio → refus (pas de host possible)
        let j3 = handle_action(
            &state,
            "{\"action\":\"set_host\",\"id\":\"man-2\",\"host\":\"10.9.9.9\"}",
        );
        assert_eq!(j3["ok"], serde_json::json!(false));

        // 4. serveur inconnu → erreur propre
        let j4 = handle_action(
            &state,
            "{\"action\":\"set_host\",\"id\":\"inconnu\",\"host\":\"10.9.9.9\"}",
        );
        assert_eq!(j4["ok"], serde_json::json!(false));

        // 5. host vide/blanc → erreur propre
        let j5 = handle_action(
            &state,
            "{\"action\":\"set_host\",\"id\":\"man-1\",\"host\":\"  \"}",
        );
        assert_eq!(j5["ok"], serde_json::json!(false));
    }

    #[test]
    fn set_start_cmd_refuse_seed_et_distant_persiste_le_local() {
        // Le bouton ▶/■ apparaît si start_cmd existe : action « set_start_cmd »
        // pour poser la commande d'un local découvert par scan. Seeds refusés
        // (la config par défaut réécraserait), distants refusés (le process
        // appartient à sa machine hôte), locaux OK + persisté.
        let path = tmp_config("cfg_setstartcmd.json", Some("{\"servers\":[]}"));
        let mut base = match Arc::try_unwrap(test_state()) {
            Ok(s) => s,
            Err(_) => panic!("test_state doit être unique"),
        };
        base.config_path = path.clone();
        let state = Arc::new(base);
        {
            let mut st = lockv(&state.settings);
            st.autostart = false;
        }
        {
            let mut servers = lockv(&state.servers);
            // local sans commande (découvert par scan de ports)
            servers.push(Server {
                id: "scan-1".into(),
                label: "port-4242".into(),
                transport: Transport::Http {
                    host: None,
                    port: 4242,
                    path: "/mcp".into(),
                },
                origin: Origin::Discovered,
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
                start_cmd: None,
                auto_restart: true,
                tools_disabled: vec![],
            });
            // distant importé (jamais startable ici)
            servers.push(Server {
                id: "dist-1".into(),
                label: "distant".into(),
                transport: Transport::Http {
                    host: Some("10.1.2.3".into()),
                    port: 8080,
                    path: "/mcp".into(),
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
                via_ssh: true,
                tools: vec![],
                latency_ms: None,
                last_seen_ts: None,
                last_error: None,
                start_cmd: None,
                auto_restart: true,
                tools_disabled: vec![],
            });
            // seed local (commande figée par la config par défaut)
            servers.push(Server {
                id: "seed-9".into(),
                label: "seed".into(),
                transport: Transport::Http {
                    host: None,
                    port: 3099,
                    path: "/mcp".into(),
                },
                origin: Origin::Seed,
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
                start_cmd: None,
                auto_restart: true,
                tools_disabled: vec![],
            });
        }

        // 1. distant → refus
        let j = handle_action(
            &state,
            "{\"action\":\"set_start_cmd\",\"id\":\"dist-1\",\"program\":\"node\",\"args\":[\"srv.js\"]}",
        );
        assert_eq!(j["ok"], serde_json::json!(false));

        // 2. seed → refus
        let j2 = handle_action(
            &state,
            "{\"action\":\"set_start_cmd\",\"id\":\"seed-9\",\"program\":\"node\"}",
        );
        assert_eq!(j2["ok"], serde_json::json!(false));

        // 3. programme vide → erreur propre
        let j3 = handle_action(
            &state,
            "{\"action\":\"set_start_cmd\",\"id\":\"scan-1\",\"program\":\"  \"}",
        );
        assert_eq!(j3["ok"], serde_json::json!(false));

        // 4. local découvert → OK : start_cmd posé (args + workdir) et persisté
        let j4 = handle_action(
            &state,
            "{\"action\":\"set_start_cmd\",\"id\":\"scan-1\",\"program\":\"node\",\"args\":[\"server.js\",\"--port\",\"4242\"],\"workdir\":\"C:/tmp\"}",
        );
        assert_eq!(j4["ok"], serde_json::json!(true));
        {
            let servers = lockv(&state.servers);
            let cmd = servers[0]
                .start_cmd
                .as_ref()
                .expect("start_cmd doit exister");
            assert_eq!(cmd.program, "node");
            assert_eq!(
                servers[0].origin,
                Origin::Manual,
                "le découvert doit être promu manual pour survivre au reboot"
            );
            assert_eq!(
                cmd.args,
                vec![
                    "server.js".to_string(),
                    "--port".to_string(),
                    "4242".to_string()
                ]
            );
            assert_eq!(cmd.workdir.as_deref(), Some("C:/tmp"));
        }
        let cfg = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            cfg.contains("server.js"),
            "la commande doit être persistée dans config.json : {}",
            cfg
        );
    }

    #[test]
    fn annonce_litemcp_importe_adopte_et_resynchronise() {
        // Persistance MCP Control <-> litemcp : une annonce (%TEMP%/
        // litemcp-registry/<nom>.json écrite par la lib au bind) crée le
        // serveur avec sa commande native, ADOpte un serveur découvert par
        // scan (sans commande → promotion manual), resynchronise le port,
        // et ne réécrit jamais une commande déjà posée.
        let path = tmp_config("cfg_litemcp.json", Some("{\"servers\":[]}"));
        let mut base = match Arc::try_unwrap(test_state()) {
            Ok(s) => s,
            Err(_) => panic!("test_state doit être unique"),
        };
        base.config_path = path.clone();
        let state = Arc::new(base);
        {
            let mut st = lockv(&state.settings);
            st.autostart = false;
        }
        {
            let mut servers = lockv(&state.servers);
            // découvert par scan : même nom, sans commande (cas moltbook réel)
            servers.push(Server {
                id: "disc-8787".into(),
                label: "moltbook".into(),
                transport: Transport::Http {
                    host: None,
                    port: 3166,
                    path: "/mcp".into(),
                },
                origin: Origin::Discovered,
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
                lang: Some("rust".into()),
                framework: Some("litemcp".into()),
                via_ssh: false,
                tools: vec![],
                latency_ms: None,
                last_seen_ts: None,
                last_error: None,
                start_cmd: None,
                auto_restart: true,
                tools_disabled: vec![],
            });
            // même serveur mais avec une commande déjà posée (choix utilisateur)
            servers.push(Server {
                id: "man-cmd".into(),
                label: "deja-cmd".into(),
                transport: Transport::Http {
                    host: None,
                    port: 9001,
                    path: "/mcp".into(),
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
                    program: "python".into(),
                    args: vec!["serve.py".into()],
                    workdir: None,
                }),
                auto_restart: true,
                tools_disabled: vec![],
            });
        }

        // 1. création : annonce d'un serveur absent de la liste
        let j = serde_json::json!({
            "name": "nouveau", "version": "1.2.3", "port": 9999, "pid": 42,
            "exe": "C:/apps/nouveau.exe", "args": ["C:/apps/nouveau.exe", "--bind", "9999"],
            "cwd": "C:/apps", "ts": 1000
        });
        assert!(
            apply_announcement(&state, &j),
            "une annonce d'inconnu doit créer le serveur"
        );
        {
            let servers = lockv(&state.servers);
            let s = servers
                .iter()
                .find(|s| s.id == "litemcp-nouveau")
                .expect("créé");
            assert_eq!(s.origin, Origin::Manual);
            assert_eq!(s.version.as_deref(), Some("1.2.3"));
            let cmd = s.start_cmd.as_ref().expect("commande native");
            assert_eq!(cmd.program, "C:/apps/nouveau.exe");
            assert_eq!(
                cmd.args,
                vec!["--bind".to_string(), "9999".to_string()],
                "args[0] (l'exe) doit être ignoré"
            );
            assert_eq!(cmd.workdir.as_deref(), Some("C:/apps"));
        }

        // 2. adoption : disc-8787 (moltbook, port 3166 sans commande) reçoit
        //    la commande de l'annonce + resync du port 8787 + promotion manual
        let j2 = serde_json::json!({
            "name": "moltbook", "version": "0.3.0", "port": 8787, "pid": 25044,
            "exe": "C:/ws/target/debug/moltbook-mcp.exe", "args": ["C:/ws/target/debug/moltbook-mcp.exe"],
            "cwd": "C:/ws", "ts": 2000
        });
        assert!(apply_announcement(&state, &j2));
        {
            let servers = lockv(&state.servers);
            let s = servers
                .iter()
                .find(|s| s.id == "disc-8787")
                .expect("existant");
            assert_eq!(s.origin, Origin::Manual, "adoption = promotion manual");
            let Transport::Http { port, .. } = s.transport else {
                panic!("http")
            };
            assert_eq!(port, 8787, "port resynchronisé depuis l'annonce");
            let cmd = s.start_cmd.as_ref().expect("commande adoptée");
            assert_eq!(cmd.program, "C:/ws/target/debug/moltbook-mcp.exe");
            assert!(cmd.args.is_empty(), "moltbook tourne sans argument");
            assert_eq!(
                cmd.workdir.as_deref(),
                Some("C:/ws"),
                "cwd = là où vivent les état json"
            );
            assert_eq!(s.version.as_deref(), Some("0.3.0"));
        }

        // 3. pas de réécriture : deja-cmd garde SA commande
        let j3 = serde_json::json!({
            "name": "deja-cmd", "version": "2.0", "port": 9001, "pid": 7,
            "exe": "C:/autre/autre.exe", "args": ["C:/autre/autre.exe"], "cwd": "C:/autre", "ts": 3000
        });
        assert!(
            apply_announcement(&state, &j3),
            "le ts plus récent compte comme changement"
        );
        {
            let servers = lockv(&state.servers);
            let s = servers
                .iter()
                .find(|s| s.id == "man-cmd")
                .expect("existant");
            let cmd = s
                .start_cmd
                .as_ref()
                .expect("la commande de l'utilisateur est conservée");
            assert_eq!(cmd.program, "python");
            assert_eq!(cmd.args, vec!["serve.py".to_string()]);
        }

        // 4. annonce invalide (port 0 / exe vide) → ignorée, rien ne change
        let n_avant = lockv(&state.servers).len();
        let j4 = serde_json::json!({"name": "cassé", "port": 0, "exe": "x"});
        assert!(!apply_announcement(&state, &j4));
        let j5 = serde_json::json!({"name": "cassé", "port": 8080, "exe": ""});
        assert!(!apply_announcement(&state, &j5));
        assert_eq!(
            lockv(&state.servers).len(),
            n_avant,
            "aucun serveur cassé créé"
        );
    }

    #[test]
    fn set_settings_valide_borne_et_persiste() {
        // Dialog ⚙ : réglages principaux. Validation stricte : refresh clamp
        // 5-3600, scan borné 1-65535 avec from<=to, bind sur whitelist
        // (sinon loopback forcé), ports réseau 1-65535 uniquement, et
        // persistance dans config.json.
        let path = tmp_config("cfg_settings_dialog.json", Some("{\"servers\":[]}"));
        let mut base = match Arc::try_unwrap(test_state()) {
            Ok(s) => s,
            Err(_) => panic!("test_state doit être unique"),
        };
        base.config_path = path.clone();
        let state = Arc::new(base);
        {
            let mut st = lockv(&state.settings);
            st.autostart = false;
        }

        // 1. valeurs hors bornes → clamps + corrections
        let j = handle_action(
            &state,
            "{\"action\":\"set_settings\",\"refresh_secs\":99999,\"scan_from\":9000,\"scan_to\":100,\"ui_bind\":\"0.0.0.0\",\"network_enabled\":false,\"network_ports\":[5090,0,70000,8081,65536]}",
        );
        assert_eq!(j["ok"], serde_json::json!(true));
        assert_eq!(
            j["changed"],
            serde_json::json!(["refresh", "scan", "ui_bind", "network", "network_ports"])
        );
        {
            let st = lockv(&state.settings);
            assert_eq!(st.refresh_secs, 3600, "clamp haut");
            assert_eq!((st.scan_from, st.scan_to), (100, 9000), "from<=to imposé");
            assert_eq!(st.ui_bind, "0.0.0.0");
            assert!(!st.network_enabled);
            assert_eq!(st.network_ports, vec![5090, 8081], "0/70000/65536 filtrés");
        }
        let cfg = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(cfg.contains("\"refresh_secs\": 3600"), "persisté : {}", cfg);
        assert!(cfg.contains("\"network_enabled\": false"));

        // 2. bind hors whitelist → loopback forcé (jamais un bind arbitraire)
        let j2 = handle_action(
            &state,
            "{\"action\":\"set_settings\",\"ui_bind\":\"192.168.1.5\"}",
        );
        assert_eq!(j2["ok"], serde_json::json!(true));
        assert_eq!(lockv(&state.settings).ui_bind, "127.0.0.1");

        // 3. refresh trop bas → clamp bas (5 s minimum, pas de poll dément)
        let j3 = handle_action(&state, "{\"action\":\"set_settings\",\"refresh_secs\":1}");
        assert_eq!(j3["ok"], serde_json::json!(true));
        assert_eq!(lockv(&state.settings).refresh_secs, 5);

        // 4. aucun changement → changed vide, pas d'écriture inutile
        let j4 = handle_action(&state, "{\"action\":\"set_settings\",\"refresh_secs\":5}");
        assert_eq!(j4["changed"], serde_json::json!([] as [&str; 0]));
    }

    #[test]
    fn reset_state_placements_detection_et_oublis() {
        // Bouton ⚙ → Réinitialiser : placement (ordre), detection (découverts
        // + imports natifs + pairs), oublis (seeds retirés réinjectés + machines
        // SSH) — et « what » inconnu refuse proprement.
        let path = tmp_config("cfg_reset.json", Some("{\"servers\":[]}"));
        let mut base = match Arc::try_unwrap(test_state()) {
            Ok(s) => s,
            Err(_) => panic!("test_state doit être unique"),
        };
        base.config_path = path.clone();
        let state = Arc::new(base);
        {
            let mut st = lockv(&state.settings);
            st.autostart = false;
            st.server_order = vec!["a".into(), "b".into()];
            st.ssh_hosts.push(crate::types::SshHost {
                ip: "192.168.40.9".into(),
                user: "demon".into(),
            });
            // un VRAI seed retiré durablement (le premier de la config par défaut)
            let seed_id = default_servers()[0].id.clone();
            st.removed_seeds.push(seed_id);
        }
        {
            let mut servers = lockv(&state.servers);
            servers.push(Server {
                id: "disc-1".into(),
                label: "port-7000".into(),
                transport: Transport::Http {
                    host: None,
                    port: 7000,
                    path: "/mcp".into(),
                },
                origin: Origin::Discovered,
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
                start_cmd: None,
                auto_restart: true,
                tools_disabled: vec![],
            });
            servers.push(Server {
                id: "litemcp-test".into(),
                label: "test".into(),
                transport: Transport::Http {
                    host: None,
                    port: 8888,
                    path: "/mcp".into(),
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
                framework: Some("litemcp".into()),
                via_ssh: false,
                tools: vec![],
                latency_ms: None,
                last_seen_ts: None,
                last_error: None,
                start_cmd: None,
                auto_restart: true,
                tools_disabled: vec![],
            });
            servers.push(Server {
                id: "man-keep".into(),
                label: "keep".into(),
                transport: Transport::Http {
                    host: None,
                    port: 9000,
                    path: "/mcp".into(),
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
                start_cmd: None,
                auto_restart: true,
                tools_disabled: vec![],
            });
        }
        lockv(&state.peers).insert(
            "10.1.1.1".into(),
            crate::types::Peer {
                ip: "10.1.1.1".into(),
                ui_port: 5090,
                last_seen: 1,
                servers: serde_json::json!([]),
            },
        );

        // 1. placement → server_order vidé, serveurs intacts
        let j = handle_action(
            &state,
            "{\"action\":\"reset_state\",\"what\":\"placement\"}",
        );
        assert_eq!(j["ok"], serde_json::json!(true));
        assert!(lockv(&state.settings).server_order.is_empty());
        assert_eq!(lockv(&state.servers).len(), 3);

        // 2. detection → découverts + litemcp retirés, manual conservé, pairs vidés
        let j2 = handle_action(
            &state,
            "{\"action\":\"reset_state\",\"what\":\"detection\"}",
        );
        assert_eq!(j2["ok"], serde_json::json!(true));
        assert_eq!(j2["removed"], serde_json::json!(2));
        {
            let servers = lockv(&state.servers);
            assert_eq!(servers.len(), 1);
            assert_eq!(
                servers[0].id, "man-keep",
                "le manual hors litemcp est conservé"
            );
        }
        assert!(lockv(&state.peers).is_empty());

        // 3. oublis → machines SSH vidées + seed réinjecté + removed_seeds vide
        let j3 = handle_action(&state, "{\"action\":\"reset_state\",\"what\":\"oublis\"}");
        assert_eq!(j3["ok"], serde_json::json!(true));
        assert_eq!(j3["reinjected"], serde_json::json!(1));
        {
            let st = lockv(&state.settings);
            assert!(st.ssh_hosts.is_empty());
            assert!(st.removed_seeds.is_empty());
        }
        assert_eq!(
            lockv(&state.servers).len(),
            2,
            "le seed retiré est réinjecté immédiatement"
        );

        // 4. what inconnu → refus propre, rien ne change
        let n = lockv(&state.servers).len();
        let j4 = handle_action(&state, "{\"action\":\"reset_state\",\"what\":\"nimporte\"}");
        assert_eq!(j4["ok"], serde_json::json!(false));
        assert_eq!(lockv(&state.servers).len(), n);
    }

    #[test]
    fn discovery_candidates_exclut_connus_et_port_ui() {
        // ports en écoute : 5433/3000 déjà connus, 5090 = notre UI, 8787 = nouveau
        let ports = vec![3000, 5090, 5433, 8787];
        let known = vec![3000, 5433];
        assert_eq!(discovery_candidates(&ports, &known, 5090), vec![8787]);
        // notre propre port UI n'est jamais sonder, même inconnu
        assert!(discovery_candidates(&[5090], &[], 5090).is_empty());
        assert!(discovery_candidates(&[], &known, 5090).is_empty());
    }

    #[test]
    fn cleanup_name_hermes() {
        assert_eq!(
            cleanup_name("@modelcontextprotocol/server-filesystem"),
            "filesystem"
        );
        assert_eq!(cleanup_name("mcp-server-git"), "git");
        assert_eq!(cleanup_name("mcp-server-time"), "time");
    }

    #[test]
    fn parse_servers_doc_accepte_les_trois_formes() {
        let w = r#"{"mcpServers":{"fs":{"command":"npx","args":["-y","pkg"]}}}"#;
        let e = parse_servers_doc(w).unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].0, "fs");
        let snake = r#"{"mcp_servers":{"fs":{"command":"npx"}}}"#;
        assert_eq!(parse_servers_doc(snake).unwrap().len(), 1);
        let brut = r#"{"fs":{"command":"npx"}}"#;
        assert_eq!(parse_servers_doc(brut).unwrap().len(), 1);
        assert!(parse_servers_doc("not json").is_err());
    }

    #[test]
    fn import_mcp_texte_reconnait_tout() {
        let state = test_state();
        // URL (http : https est refusé explicitement, pas de TLS dans ce client)
        let j = import_mcp_text(&state, "http://mcp.linear.app/mcp");
        assert_eq!(j["ok"], json!(true));
        // commande claude
        let j2 = import_mcp_text(&state, "claude mcp add monserveur npx -y pkg");
        assert_eq!(j2["ok"], json!(true));
        // commande npx brute
        let j3 = import_mcp_text(
            &state,
            "npx -y @modelcontextprotocol/server-filesystem /tmp",
        );
        assert_eq!(j3["ok"], json!(true));
        let servers = lockv(&state.servers);
        assert!(servers.iter().any(|s| s.id == "store-linear"));
        assert!(servers.iter().any(|s| s.label.starts_with("monserveur")));
        assert!(servers
            .iter()
            .any(|s| s.id.starts_with("store-filesystem-")));
    }

    #[test]
    fn import_mcp_texte_court_ne_panique_pas() {
        // Régression audit 2026-10-24 : « claude mcp add » (3-4 tokens) indexait
        // toks[3]/toks[4] → panic du thread HTTP, aucune réponse au client.
        let state = test_state();
        for texte in ["claude mcp add", "claude mcp add monserveur"] {
            let j = import_mcp_text(&state, texte);
            assert_eq!(
                j["ok"],
                json!(false),
                "'{texte}' doit être refusé proprement (pas de panic)"
            );
        }
        assert!(lockv(&state.servers).is_empty());
    }

    #[test]
    fn import_https_est_refuse_avec_message_clair() {
        // Régression audit : une URL https finissait en sonde plaintext illisible (DOWN permanent)
        let state = test_state();
        let j = import_mcp_text(&state, "https://mcp.linear.app/mcp");
        assert_eq!(j["ok"], json!(false));
        assert!(j["error"].as_str().unwrap().contains("TLS"));
        assert!(lockv(&state.servers).is_empty());
    }

    #[test]
    fn install_store_filesystem_exige_un_dossier_explicite() {
        // Régression audit : le catalogue embarquait C:/Users/<user> en dur (profil entier exposé)
        let state = test_state();
        // sans args → refus AVANT tout spawn (poll_one jamais atteint)
        let j = handle_action(&state, r#"{"action":"install_store","id":"filesystem"}"#);
        assert_eq!(j["ok"], json!(false));
        assert!(lockv(&state.servers).is_empty());
        // critère pur : le nom du package contient un « / » — seule la LONGUEUR protège
        let pkg = "-y".to_string();
        let nm = "@modelcontextprotocol/server-filesystem".to_string();
        assert!(filesystem_dossier_manquant(&[pkg.clone(), nm.clone()]));
        assert!(!filesystem_dossier_manquant(&[
            pkg,
            nm,
            "C:/data".to_string()
        ]));
        assert!(filesystem_dossier_manquant(&[
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
            "relatif-sans-separateur".to_string()
        ]));
    }

    #[test]
    fn settings_defaut_loopback_et_api_access_lu_de_la_config() {
        // Régression audit : le bind par défaut 0.0.0.0 rendait l'API contrôlable du LAN
        let (_, st) = load_config(&tmp_config("cfg_defaut_audit.json", None));
        assert_eq!(st.ui_bind, "127.0.0.1");
        assert!(st.api_access.is_none());
        assert!(st.allowed_origins.is_empty());
        let cfg = r#"{"servers":[],"settings":{"ui_bind":"0.0.0.0","api_access":"valeur-partagee-pair","allowed_origins":["http://mon-host.local:5090"]}}"#;
        let (_, st2) = load_config(&tmp_config("cfg_access.json", Some(cfg)));
        assert_eq!(st2.ui_bind, "0.0.0.0");
        assert_eq!(st2.api_access.as_deref(), Some("valeur-partagee-pair"));
        assert_eq!(
            st2.allowed_origins,
            vec!["http://mon-host.local:5090".to_string()]
        );
    }

    #[test]
    fn name_from_url_hermes() {
        assert_eq!(name_from_url("mcp.linear.app"), "linear");
        assert_eq!(name_from_url("api.github.com"), "github");
        assert_eq!(name_from_url("example.com"), "example");
    }

    #[test]
    fn detect_lang_stdio_par_commande() {
        let none: Vec<String> = vec![];
        assert_eq!(
            crate::probe::detect_lang_stdio("node", &none).as_deref(),
            Some("node")
        );
        assert_eq!(
            crate::probe::detect_lang_stdio("npx", &none).as_deref(),
            Some("node")
        );
        assert_eq!(
            crate::probe::detect_lang_stdio("uvx", &none).as_deref(),
            Some("python")
        );
        assert_eq!(
            crate::probe::detect_lang_stdio("C:/Python311/python.exe", &none).as_deref(),
            Some("python")
        );
        assert_eq!(
            crate::probe::detect_lang_stdio("cargo", &none).as_deref(),
            Some("rust")
        );
        assert_eq!(
            crate::probe::detect_lang_stdio("java", &none).as_deref(),
            Some("java")
        );
        assert_eq!(
            crate::probe::detect_lang_stdio("dotnet", &none).as_deref(),
            Some("dotnet")
        );
        assert_eq!(
            crate::probe::detect_lang_stdio("mybinary", &["server.py".to_string()]).as_deref(),
            Some("python")
        );
        assert_eq!(
            crate::probe::detect_lang_stdio("mybinary", &["index.js".to_string()]).as_deref(),
            Some("node")
        );
        assert_eq!(crate::probe::detect_lang_stdio("strange", &none), None);
    }

    #[test]
    fn detect_lang_http_par_entetes() {
        let mk = |server: &str, powered: &str| crate::probe::RespHead {
            content_type: String::new(),
            server: server.to_string(),
            powered_by: powered.to_string(),
            session_id: String::new(),
        };
        assert_eq!(
            crate::probe::detect_lang_http(&mk("uvicorn", ""), None).as_deref(),
            Some("python")
        );
        assert_eq!(
            crate::probe::detect_lang_http(&mk("", "Express"), None).as_deref(),
            Some("node")
        );
        assert_eq!(
            crate::probe::detect_lang_http(&mk("hyper", ""), None).as_deref(),
            Some("rust")
        );
        assert_eq!(
            crate::probe::detect_lang_http(&mk("Kestrel", ""), None).as_deref(),
            Some("dotnet")
        );
        assert_eq!(
            crate::probe::detect_lang_http(&mk("Jetty(11)", ""), None).as_deref(),
            Some("java")
        );
        assert_eq!(crate::probe::detect_lang_http(&mk("", ""), None), None);
        assert_eq!(
            crate::probe::detect_lang_http(&mk("", ""), Some("my-rust-server")).as_deref(),
            Some("rust")
        );
    }

    #[test]
    fn resolve_config_path_override_gagne() {
        let p = resolve_config_path("C:/nimporte", Some("D:/ma/config.json".to_string()));
        assert_eq!(p, "D:/ma/config.json");
    }

    #[test]
    fn resolve_config_path_dev_existant_gagne() {
        let dir = std::env::temp_dir().join("mcpcc_resolve_dev");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.json"), "{}").unwrap();
        let p = resolve_config_path(&dir.to_string_lossy(), None);
        assert_eq!(p, dir.join("config.json").to_string_lossy().to_string());
    }

    #[test]
    fn resolve_config_path_sans_config_pointe_vers_exe() {
        let dir = std::env::temp_dir().join("mcpcc_resolve_vide");
        std::fs::create_dir_all(&dir).unwrap();
        let _ = std::fs::remove_file(dir.join("config.json"));
        let p = resolve_config_path(&dir.to_string_lossy(), None);
        assert!(
            p.ends_with("config.json"),
            "doit finir par config.json, got {}",
            p
        );
        assert!(
            !p.starts_with(&dir.to_string_lossy().to_string()),
            "ne doit PAS pointer vers le manifest (machine cliente), got {}",
            p
        );
    }

    #[test]
    fn http_avec_start_cmd_charge_et_persiste() {
        let cfg = r#"{"servers":[{"id":"h1","label":"H","transport":"http","port":3210,"path":"/mcp","program":"node","args":["srv.js","--p","3210"],"workdir":"C:/s","enabled":true}]}"#;
        let path = tmp_config("cfg_http_cmd.json", Some(cfg));
        let (servers, _) = load_config(&path);
        let s = servers.iter().find(|s| s.id == "h1").unwrap();
        let cmd = s
            .start_cmd
            .as_ref()
            .expect("start_cmd http doit être chargé du json");
        assert_eq!(cmd.program, "node");
        assert_eq!(
            cmd.args,
            vec!["srv.js".to_string(), "--p".to_string(), "3210".to_string()]
        );
        assert_eq!(cmd.workdir.as_deref(), Some("C:/s"));
        // round-trip save/reload
        let state = AppState {
            servers: Mutex::new(servers.clone()),
            log: Mutex::new(VecDeque::new()),
            settings: Mutex::new(Settings::default()),
            children: Mutex::new(std::collections::HashMap::new()),
            config_path: path.clone(),
            peers: Mutex::new(std::collections::HashMap::new()),
            allowed_origins: Mutex::new(Vec::new()),
            allowed_hosts: Mutex::new(Vec::new()),
            spawn_fails: Mutex::new(std::collections::HashMap::new()),
            runtime: Mutex::new(crate::runtime::RuntimeLog::new()),
            spawning: Mutex::new(std::collections::HashSet::new()),
            update_latest: Mutex::new(None),
            lan_ok: Mutex::new(None),
        };
        save_config(&state);
        let (servers2, _) = load_config(&path);
        let s2 = servers2.iter().find(|s| s.id == "h1").unwrap();
        let cmd2 = s2
            .start_cmd
            .as_ref()
            .expect("start_cmd http doit survivre au round-trip");
        assert_eq!(cmd2.program, "node");
        assert_eq!(
            cmd2.args,
            vec!["srv.js".to_string(), "--p".to_string(), "3210".to_string()]
        );
    }
}
