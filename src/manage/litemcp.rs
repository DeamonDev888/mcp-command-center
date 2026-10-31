// Détection litemcp : registre %TEMP%, annonces, import natif, boucle de sondage.
// (split v0.7.20 : manage.rs reste la racine — API stable via réexport)
use super::*;

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
pub(super) fn apply_announcement(state: &Arc<AppState>, ann: &Value) -> bool {
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
                favorite: false,
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
