// Cache de démarrage (dernier état connu) — state-cache.json.
// (split v0.7.20 : manage.rs reste la racine — API stable via réexport)
use super::*;

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
