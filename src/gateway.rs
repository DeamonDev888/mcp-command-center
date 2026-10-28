//! Gateway MCP — la console devient elle-même un serveur MCP.
//!
//! Un client MCP (ZCode, Claude Desktop, MCP Inspector…) qui POSTe sur
//! `/mcp` (même port que l'UI) obtient **toute la surface de l'UI en tools** :
//!
//! - **supervision** (lecture) : `list_servers`, `server_detail`,
//!   `list_server_tools`, `journal`, `runtime_history`, `list_peers`,
//!   `store_catalog`, `list_frameworks`, `get_settings` ;
//! - **supervision** (actions) : chaque bouton de l'UI a son outil —
//!   `toggle_server`, `start_server`, `stop_server`, `add_server`,
//!   `remove_server`, `scan_ports`, `scan_network`, `discover_peers`,
//!   `set_start_command`, `set_host`, `toggle_tool`, `set_auto_restart`,
//!   `reorder_servers`, `import_config`, `install_store_server`, `refresh`,
//!   `check_update`, `ssh_save`, `ssh_forget`, `ssh_discover` — tous
//!   adossés au même dispatcher que l'UI (`manage::handle_action`) : une
//!   seule sémantique, une seule validation, un seul journal ;
//! - **gateway** : `call_server_tool(server, tool, arguments)` proxyfie un
//!   `tools/call` vers n'importe quel serveur HTTP détecté (UP), en
//!   respectant l'interrupteur par outil de l'UI (`tools_disabled`) ;
//! - **ressources** : `mcp-control://servers` (aperçu),
//!   `mcp-control://server/<id>` (détail par serveur détecté),
//!   `mcp-control://journal`, `mcp-control://settings` — reconstruites à
//!   chaque changement de détection.
//!
//! Volontairement **non exposés** (surface de confiance de l'app) :
//! `set_settings` (code d'accès, bind, origines — c'est le sas d'auth du
//! gateway lui-même), `set_autostart`, `set_notify` (webhook), `reset_state`
//! et `reset_runtime` (destructifs), `open_path` (ouvre l'explorateur).
//! Ces réglages restent des gestes UI locaux.
//!
//! Le serveur MCP est propulsé par [LiteMCP](https://crates.io/crates/litemcp)
//! (spec 2026-07-28 stateless, résultat `resultType` + `_meta` sur chaque
//! réponse) : les tools sont enregistrés dans un `ServerBuilder`, et le
//! **module `gateway` de LiteMCP 0.9** fournit la mécanique du registre
//! vivant — le snapshot courant vit derrière un `RwLock` et est échangé
//! atomiquement quand la détection change. Plus de mécanique locale : c'est
//! la lib publiée qui fait le travail.
//!
//! Transport : le moteur possède déjà son propre socket durci (Host/Origin,
//! code d'accès hors loopback, plafonds de corps) — le gateway s'y monte en
//! `POST /mcp` plutôt que d'ouvrir un second port. Les règles de transport de
//! LiteMCP sont reflétées : `Content-Type: application/json` exigé (415),
//! négociation `Accept` (SSE mono-événement si le client ne veut que ça),
//! `Mcp-Method`/`Mcp-Name` vérifiés contre le corps (SEP-2243), notifications
//! → `202` sans corps, GET/DELETE → `405`.

use crate::manage;
use crate::probe;
use crate::types::{lockv, AppState, SState, Transport};
use litemcp::error::ToolError;
use litemcp::jsonrpc::ErrorObject;
use litemcp::protocol::{ResourceContents, ToolAnnotations};
use litemcp::server::McpHeaders;
use litemcp::zod;
use litemcp::{Gateway as LiteGateway, ServerBuilder, ServerCore};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

/// Timeout d'un `tools/call` proxifié vers un serveur détecté.
const PROXY_TIMEOUT_MS: u64 = 20_000;

/// Numérotation des requêtes JSON-RPC sortantes du proxy (une par appel).
static PROXY_RPC_ID: AtomicU64 = AtomicU64::new(1);

/// Le gateway courant — le moteur est **instance unique** (run_headless se
/// reconnecte à un moteur existant au lieu d'en lancer un second), un slot
/// process-global reflète donc exactement le gateway du moteur vivant.
static GATEWAY: OnceLock<Arc<Gateway>> = OnceLock::new();

/// Un serveur MCP vivant : le snapshot `ServerCore` est reconstruit et
/// échangé quand la détection change (ressources par serveur). Les outils
/// de supervision, eux, sont statiques — toute leur donnée est lue en direct
/// dans `AppState` à chaque appel, jamais figée dans le snapshot.
///
/// Le mécanisme est celui de **`litemcp::Gateway`** (module publié dans
/// litemcp 0.9) : le builder vit derrière un verrou réentrant, chaque
/// `edit` reconstruit un noyau neuf puis l'échange atomiquement — les
/// requêtes en vol finissent sur le snapshot où elles ont commencé, deux
/// reconstructions concurrentes sont sérialisées (aucune mutation perdue)
/// et le catalogue ne passe jamais par un état à moitié reconstruit.
pub type Gateway = LiteGateway;

/// Nombre d'outils publiés par un noyau — via son propre `tools/list`
/// (l'API 0.8 n'expose pas d'introspection directe ; un dispatch local ne
/// coûte rien au boot).
pub fn tool_count(core: &ServerCore) -> usize {
    core.dispatch(br#"{"jsonrpc":"2.0","id":0,"method":"tools/list"}"#)
        .and_then(|reply| serde_json::from_slice::<Value>(&reply).ok())
        .and_then(|v| v["result"]["tools"].as_array().map(Vec::len))
        .unwrap_or(0)
}

/// Construit le gateway du moteur et l'installe dans le slot global.
/// À appeler **une fois** par moteur (run_headless).
pub fn install(state: &Arc<AppState>) -> Arc<Gateway> {
    // `litemcp::Gateway` prend un `ServerBuilder` et publie le premier
    // snapshot — la reconstruction passe désormais par son API `edit`.
    let gw = Arc::new(LiteGateway::new(build_builder(state)));
    let _ = GATEWAY.set(Arc::clone(&gw));
    gw
}

/// Le gateway du moteur (None avant `install` — les tests qui construisent
/// un `AppState` sans moteur n'ont pas de gateway).
pub fn current() -> Option<&'static Arc<Gateway>> {
    GATEWAY.get()
}

/// Reconstruit le snapshot (catalogue des ressources par serveur détecté).
/// Aucun coût visible : le noyau reconstruit en microsecondes, les requêtes
/// en vol finissent sur l'instantané où elles ont commencé.
pub fn refresh(state: &Arc<AppState>) {
    let Some(gw) = GATEWAY.get() else {
        return;
    };
    // Reconstruction + échange atomique via le module officiel : le snapshot
    // publié ne peut jamais être plus ancien que l'état reconstruit.
    let st = Arc::clone(state);
    gw.edit(move |_builder| build_builder(&st));
}

/// Empreinte de l'état qui nourrit le snapshot : ids, états, interrupteurs,
/// outils (nombre + désactivés) de chaque serveur. Change dès que la
/// détection ou un toggle change → le thread de fond reconstruit le
/// snapshot sans avoir à croiser les CallSites de mutation.
pub fn fingerprint(state: &Arc<AppState>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut lines: Vec<String> = {
        let servers = lockv(&state.servers);
        servers
            .iter()
            .map(|s| {
                format!(
                    "{}|{}|{}|{}|{}",
                    s.id,
                    s.state.as_str(),
                    s.enabled,
                    s.tools.len(),
                    s.tools_disabled.len()
                )
            })
            .collect()
    };
    lines.sort();
    for line in &lines {
        line.hash(&mut h);
    }
    h.finish()
}

// ---------- construction du serveur MCP ----------

/// Échec d'une action : le dispatcher de l'UI répond `{ok:false,error}` —
/// traduit en `ToolError` classifié (`invalid_input`, message lisible).
fn do_action(state: &Arc<AppState>, req: Value) -> Result<Value, ToolError> {
    let v = manage::handle_action(state, &req.to_string());
    match v.get("ok").and_then(Value::as_bool) {
        Some(true) => {
            // la surface détectée vient de changer → snapshot à jour tout de
            // suite (les ressources reflètent le nouveau serveur sans attendre
            // le thread de fond)
            refresh(state);
            Ok(v)
        }
        _ => {
            let msg = v
                .get("error")
                .and_then(Value::as_str)
                .map(String::from)
                .unwrap_or_else(|| format!("échec de l'action : {v}"));
            Err(ToolError::invalid_input(msg))
        }
    }
}

/// Les arguments validés de l'outil deviennent le corps de `/api/action` :
/// mêmes clés, mêmes sémantiques que la page UI.
fn action_req(action: &str, args: &Value) -> Value {
    let mut req = serde_json::Map::new();
    req.insert("action".into(), json!(action));
    if let Some(map) = args.as_object() {
        for (k, v) in map {
            req.insert(k.clone(), v.clone());
        }
    }
    Value::Object(req)
}

fn read_only(title: &'static str) -> ToolAnnotations {
    ToolAnnotations::new().title(title).read_only(true)
}

fn modifies(title: &'static str) -> ToolAnnotations {
    ToolAnnotations::new().title(title)
}

fn destructive(title: &'static str) -> ToolAnnotations {
    ToolAnnotations::new().title(title).destructive(true)
}

/// Construit un instantané complet du gateway : outils de supervision
/// (statiques, données lues en direct) + ressources par serveur détecté.
fn build_builder(state: &Arc<AppState>) -> ServerBuilder {
    let builder = ServerBuilder::new("mcp-control", env!("CARGO_PKG_VERSION")).on_call({
        let st = Arc::clone(state);
        move |ev| {
            // une ligne par message dispatché : le journal de l'UI raconte
            // aussi ce que font les clients MCP (audit trail gratuit)
            manage::add_log(
                &st,
                "info",
                &format!(
                    "[gateway] {} {} → {:?} en {} ms",
                    ev.method,
                    ev.name.unwrap_or("-"),
                    ev.status,
                    ev.duration.as_millis()
                ),
            );
        }
    });
    let builder = read_tools(builder, state);
    let builder = action_tools(builder, state);
    let builder = proxy_tools(builder, state);
    dynamic_resources(builder, state)
}

// ---------- outils de lecture (l'état est lu EN DIRECT à chaque appel) ----------

fn read_tools(b: ServerBuilder, state: &Arc<AppState>) -> ServerBuilder {
    // list_servers : le miroir de l'écran principal.
    let b = b
        .annotate("list_servers", read_only("Lister les serveurs"))
        .tool_typed(
            "list_servers",
            "Liste tous les serveurs MCP supervisés avec leur état fin (UP répond au \
             handshake MCP, PORT-ONLY port ouvert mais pas MCP, DOWN port fermé, \
             DISABLED en pause), leurs badges (langage/transport/framework) et les \
             compteurs globaux. C'est le premier outil à appeler.",
            zod::object().opt(
                "state",
                zod::describe(
                    "filtre d'état : up, down, port_only ou disabled",
                    zod::enumeration(&["up", "down", "port_only", "disabled"]),
                ),
            ),
            {
                let st = Arc::clone(state);
                move |args| {
                    let full = manage::state_json(&st);
                    let mut servers = full["servers"].clone();
                    if let Some(want) = args.get("state").and_then(Value::as_str) {
                        let want = want.to_ascii_uppercase();
                        let want = match want.as_str() {
                            "PORT_ONLY" => "PORT-ONLY",
                            w => w,
                        };
                        servers = servers
                            .as_array()
                            .map(|arr| {
                                Value::Array(
                                    arr.iter()
                                        .filter(|s| s["state"].as_str() == Some(want))
                                        .cloned()
                                        .collect(),
                                )
                            })
                            .unwrap_or(Value::Array(vec![]));
                    }
                    Ok(json!({
                        "stats": full["stats"],
                        "lan_ok": full["lan_ok"],
                        "servers": servers,
                    }))
                }
            },
        );
    // server_detail : le panneau de détail de l'UI.
    let b = b
        .annotate("server_detail", read_only("Détail d'un serveur"))
        .tool_typed(
            "server_detail",
            "Détail complet d'un serveur supervisé : état, erreur éventuelle, \
             latence, version, protocol, framework détecté, pid, commande de \
             lancement et liste de ses outils (avec l'indicateur disabled).",
            zod::object().field(
                "id",
                zod::describe(
                    "identifiant du serveur (voir list_servers)",
                    zod::string().min(1),
                ),
            ),
            {
                let st = Arc::clone(state);
                move |args| {
                    let id = args["id"].as_str().unwrap_or_default().to_string();
                    let detail = manage::detail_json(&st, &id);
                    if detail.get("error").is_some() {
                        return Err(ToolError::invalid_input(
                            detail["error"]
                                .as_str()
                                .unwrap_or("serveur inconnu")
                                .to_string(),
                        ));
                    }
                    Ok(detail)
                }
            },
        );
    // list_server_tools : le catalogue d'outils d'un serveur détecté.
    let b = b
        .annotate("list_server_tools", read_only("Outils d'un serveur"))
        .tool_typed(
            "list_server_tools",
            "Liste les outils (tools) annoncés par un serveur détecté, avec leur \
             description et l'indicateur disabled (interrupteur par outil de l'UI). \
             Le proxy d'appel est call_server_tool.",
            zod::object().field(
                "server",
                zod::describe(
                    "identifiant du serveur (voir list_servers)",
                    zod::string().min(1),
                ),
            ),
            {
                let st = Arc::clone(state);
                move |args| {
                    let id = args["server"].as_str().unwrap_or_default();
                    let servers = lockv(&st.servers);
                    let Some(s) = servers.iter().find(|s| s.id == id) else {
                        return Err(ToolError::invalid_input(format!("serveur inconnu : {id}")));
                    };
                    Ok(json!({
                        "server": s.id,
                        "label": s.label,
                        "state": s.state.as_str(),
                        "framework": s.framework,
                        "tools": s.tools.iter().map(|t| json!({
                            "name": t.name,
                            "description": t.desc,
                            "disabled": s.tools_disabled.contains(&t.name),
                        })).collect::<Vec<_>>(),
                    }))
                }
            },
        );
    // journal : la barre « Journal » de l'UI.
    let b = b
        .annotate("journal", read_only("Journal de la console"))
        .tool_typed(
            "journal",
            "Dernières entrées du journal de la console (détections, actions, \
             crashs, relances, activité gateway). Ordre : plus récent d'abord.",
            zod::object().opt(
                "limit",
                zod::describe(
                    "nombre d'entrées (1..500, défaut 40)",
                    zod::integer().min(1).max(500),
                ),
            ),
            {
                let st = Arc::clone(state);
                move |args| {
                    let limit = args
                        .get("limit")
                        .and_then(Value::as_i64)
                        .unwrap_or(40)
                        .clamp(1, 500) as usize;
                    let log = lockv(&st.log);
                    let entries: Vec<Value> = log
                        .iter()
                        .rev()
                        .take(limit)
                        .map(|e| json!({"ts": e.ts, "level": e.level, "msg": e.msg}))
                        .collect();
                    Ok(json!({ "entries": entries }))
                }
            },
        );
    // runtime_history : l'historique crashs/relances/backoff.
    b.annotate("runtime_history", read_only("Historique runtime"))
        .tool(
            "runtime_history",
            "Historique runtime de supervision : crashs, redémarrages, échecs de \
             démarrage et backoff d'autoboot par serveur.",
            {
                let st = Arc::clone(state);
                move |_args| Ok(manage::runtime_json(&st))
            },
        )
        .annotate("list_peers", read_only("Pairs du LAN"))
        .tool(
            "list_peers",
            "Autres instances MCP Control détectées sur le réseau local et les \
             serveurs qu'elles annoncent.",
            {
                let st = Arc::clone(state);
                move |_args| {
                    let full = manage::state_json(&st);
                    Ok(json!({ "peers": full["peers"], "lan_ok": full["lan_ok"] }))
                }
            },
        )
        .annotate("store_catalog", read_only("Catalogue boutique"))
        .tool_typed(
            "store_catalog",
            "Catalogue de la boutique : serveurs MCP prêts à installer en un clic \
             (install_store_server), filtrables par mot-clé (id, nom, description, \
             catégorie).",
            zod::object().opt(
                "query",
                zod::describe(
                    "filtre plein texte sur id/nom/description/catégorie",
                    zod::string().min(1),
                ),
            ),
            {
                move |args| {
                    let all = manage::store_json();
                    let query = args
                        .get("query")
                        .and_then(Value::as_str)
                        .map(str::to_ascii_lowercase);
                    let items = match (&query, all.as_array()) {
                        (Some(q), Some(arr)) => Value::Array(
                            arr.iter()
                                .filter(|it| {
                                    let hay = format!(
                                        "{} {} {} {}",
                                        it["id"], it["name"], it["desc"], it["cat"]
                                    )
                                    .to_ascii_lowercase();
                                    hay.contains(q.as_str())
                                })
                                .cloned()
                                .collect(),
                        ),
                        _ => all,
                    };
                    Ok(json!({ "count": items.as_array().map(Vec::len).unwrap_or(0), "items": items }))
                }
            },
        )
        .annotate("list_frameworks", read_only("Frameworks reconnus"))
        .tool(
            "list_frameworks",
            "Les frameworks de serveurs MCP reconnus par la détection (badge de \
             chaque ligne) : id, libellé, langage.",
            move |_args| {
                let arr: Vec<Value> = crate::frameworks::FRAMEWORKS
                    .iter()
                    .map(|f| json!({"id": f.id, "label": f.label, "lang": f.lang}))
                    .collect();
                Ok(json!({ "count": arr.len(), "frameworks": arr }))
            },
        )
        .annotate("get_settings", read_only("Réglages (lecture)"))
        .tool(
            "get_settings",
            "Réglages non sensibles de la console : ports, bind, plage de scan, \
             refresh, réseau. Le code d'accès API n'est JAMAIS exposé — les \
             réglages d'accès se modifient uniquement dans l'UI.",
            {
                let st = Arc::clone(state);
                move |_args| {
                    let s = lockv(&st.settings);
                    Ok(json!({
                        "refresh_secs": s.refresh_secs,
                        "ui_port": s.ui_port,
                        "ui_bind": s.ui_bind,
                        "scan_from": s.scan_from,
                        "scan_to": s.scan_to,
                        "network_enabled": s.network_enabled,
                        "autostart": s.autostart,
                        "ssh_hosts": s.ssh_hosts.iter().map(|h| json!({"ip": h.ip, "user": h.user})).collect::<Vec<_>>(),
                        "server_order": s.server_order,
                    }))
                }
            },
        )
}

// ---------- outils d'action (chaque bouton de l'UI a son outil) ----------
//
// Tous adossés au dispatcher UI (manage::handle_action) : la validation, le
// journal et les effets sont EXACTEMENT ceux des boutons — une seule
// sémantique à tester (la suite E2E couvre déjà le contrat de chaque action).

fn action_tools(b: ServerBuilder, state: &Arc<AppState>) -> ServerBuilder {
    let act = |st: &Arc<AppState>, action: &'static str| {
        let st = Arc::clone(st);
        move |_args: &Value| do_action(&st, json!({ "action": action }))
    };
    let act_id = |st: &Arc<AppState>, action: &'static str| {
        let st = Arc::clone(st);
        move |args: &Value| {
            let mut req = action_req(action, args);
            req["id"] = args["id"].clone();
            do_action(&st, req)
        }
    };

    let b = b
        .annotate("refresh", modifies("Rafraîchir"))
        .tool(
            "refresh",
            "Re-sonde tous les serveurs et relance la recherche des nouveaux \
             (équivalent du bouton ⟳) : rafraîchit les états, latences et \
             catalogue d'outils de chaque serveur.",
            act(state, "refresh"),
        )
        .annotate("scan_ports", modifies("Scan de ports"))
        .tool_typed(
            "scan_ports",
            "Cherche des serveurs MCP sur une plage de ports locaux (défaut : la \
             plage réglée dans l'app). Synchronique : rend quand le scan est fini.",
            zod::object()
                .opt("from", zod::integer().min(1).max(65535))
                .opt("to", zod::integer().min(1).max(65535)),
            {
                let st = Arc::clone(state);
                move |args| {
                    let mut req = action_req("scan", args);
                    req["from"] = args.get("from").cloned().unwrap_or(Value::Null);
                    req["to"] = args.get("to").cloned().unwrap_or(Value::Null);
                    do_action(&st, req)
                }
            },
        )
        .annotate("scan_network", modifies("Scan réseau"))
        .tool_typed(
            "scan_network",
            "Cherche des serveurs MCP sur les autres machines du réseau local \
             (plage d'IP, ports connus). Synchronique : les détections arrivent \
             dans le journal.",
            zod::object()
                .opt("from", zod::integer().min(1).max(65535))
                .opt("to", zod::integer().min(1).max(65535)),
            {
                let st = Arc::clone(state);
                move |args| {
                    let mut req = action_req("scan_network", args);
                    req["from"] = args.get("from").cloned().unwrap_or(Value::Null);
                    req["to"] = args.get("to").cloned().unwrap_or(Value::Null);
                    do_action(&st, req)
                }
            },
        )
        .annotate("discover_peers", modifies("Découvrir les pairs"))
        .tool(
            "discover_peers",
            "Redécouvre les autres instances MCP Control du LAN (preuve de pair \
             HMAC) et importe leurs annonces.",
            act(state, "discover_peers"),
        )
        .annotate("check_update", read_only("Vérifier les mises à jour"))
        .tool(
            "check_update",
            "Vérifie maintenant la dernière release publiée sur GitHub (bouton \
             « Vérifier » des réglages).",
            {
                let st = Arc::clone(state);
                move |_args| {
                    // pas un bouton du dispatcher : lecture + badge, sans ok
                    manage::check_update(&st);
                    let latest = lockv(&st.update_latest).clone();
                    Ok(json!({
                        "installed": env!("CARGO_PKG_VERSION"),
                        "latest": latest,
                        "update_available": latest.is_some(),
                    }))
                }
            },
        )
        .annotate("add_server", modifies("Ajouter un serveur"))
        .tool_typed(
            "add_server",
            "Ajoute un serveur à la supervision (bouton « + Ajouter ») : HTTP \
             (port + chemin, machine locale ou LAN) ou stdio (programme + args). \
             Rend l'identifiant du nouveau serveur.",
            zod::object()
                .field(
                    "transport",
                    zod::describe("type de transport", zod::enumeration(&["http", "stdio"])),
                )
                .opt("label", zod::string())
                .opt(
                    "port",
                    zod::describe(
                        "port HTTP (requis pour http)",
                        zod::integer().min(1).max(65535),
                    ),
                )
                .opt(
                    "host",
                    zod::describe("hôte HTTP (défaut : cette machine)", zod::string()),
                )
                .opt(
                    "path",
                    zod::describe("chemin MCP (défaut /mcp)", zod::string()),
                )
                .opt(
                    "program",
                    zod::describe(
                        "programme stdio (requis pour stdio, ex. node)",
                        zod::string(),
                    ),
                )
                .opt(
                    "args",
                    zod::describe("arguments du programme stdio", zod::array(zod::string())),
                )
                .opt("workdir", zod::string()),
            {
                let st = Arc::clone(state);
                move |args| do_action(&st, action_req("add", args))
            },
        )
        .annotate("remove_server", destructive("Supprimer un serveur"))
        .tool_typed(
            "remove_server",
            "Retire un serveur de la liste. `durable=true` l'empêche aussi d'être \
             réinjecté au boot (équivalent « Retirer durablement »).",
            zod::object()
                .field(
                    "id",
                    zod::describe("identifiant du serveur", zod::string().min(1)),
                )
                .opt("durable", zod::boolean()),
            act_id(state, "remove"),
        )
        .annotate("toggle_server", modifies("Allumer / éteindre"))
        .tool_typed(
            "toggle_server",
            "Interrupteur ON/OFF d'un serveur : ON = supervisé et démarrable \
             (relance le process stdio s'il en a un), OFF = désactivé (sonde \
             arrêtée, process tué).",
            zod::object().field("id", zod::string().min(1)),
            act_id(state, "toggle"),
        )
        .annotate("start_server", modifies("Démarrer"))
        .tool_typed(
            "start_server",
            "Démarre le programme d'un serveur (bouton ▶, stdio ou commande de \
             lancement connue).",
            zod::object().field("id", zod::string().min(1)),
            act_id(state, "start"),
        )
        .annotate("stop_server", destructive("Arrêter"))
        .tool_typed(
            "stop_server",
            "Arrête le programme d'un serveur (bouton ■ : tue le process stdio / \
             le process du port).",
            zod::object().field("id", zod::string().min(1)),
            act_id(state, "kill"),
        )
        .annotate("set_start_command", modifies("Commande de lancement"))
        .tool_typed(
            "set_start_command",
            "Définit (ou retire avec program=null) la commande de lancement d'un \
             serveur : ce qui est exécuté par start_server et à l'autoboot.",
            zod::object()
                .field("id", zod::string().min(1))
                .opt("program", zod::string())
                .opt("args", zod::array(zod::string()))
                .opt("workdir", zod::string()),
            act_id(state, "set_start_cmd"),
        )
        .annotate("set_host", modifies("Re-host"))
        .tool_typed(
            "set_host",
            "Change la machine (IP ou hostname) d'un serveur HTTP — re-host sans \
             réimporter. Refusé sur un serveur stdio (toujours local).",
            zod::object()
                .field("id", zod::string().min(1))
                .field("host", zod::string().min(1)),
            act_id(state, "set_host"),
        )
        .annotate("toggle_tool", modifies("Couper / rétablir un outil"))
        .tool_typed(
            "toggle_tool",
            "Interrupteur par outil d'un serveur (panneau détail de l'UI) : un \
             outil coupé est exclu du proxy call_server_tool.",
            zod::object()
                .field("id", zod::string().min(1))
                .field("tool", zod::string().min(1)),
            act_id(state, "toggle_tool"),
        )
        .annotate("set_auto_restart", modifies("Autoboot du serveur"))
        .tool_typed(
            "set_auto_restart",
            "Active/désactive la relance automatique d'un serveur au boot de \
             l'app et après un crash (bouton ⚡ de la ligne).",
            zod::object()
                .field("id", zod::string().min(1))
                .field("enabled", zod::boolean()),
            act_id(state, "set_auto_restart"),
        )
        .annotate("reorder_servers", modifies("Réordonner"))
        .tool_typed(
            "reorder_servers",
            "Réordonne la liste (glisser-déposer) : ids dans le nouvel ordre \
             (ordre d'affichage ET priorité de sondage).",
            zod::object().field(
                "ids",
                zod::describe(
                    "ids des serveurs dans le nouvel ordre",
                    zod::array(zod::string().min(1)),
                ),
            ),
            act_id(state, "reorder"),
        )
        .annotate("import_config", modifies("Importer une config MCP"))
        .tool_typed(
            "import_config",
            "Importe des serveurs depuis une configuration MCP existante (texte \
             JSON : config Claude/Cursor, mcpServers…) — même parseur que le \
             bouton « Importer » de l'UI.",
            zod::object().field(
                "text",
                zod::describe(
                    "contenu JSON de la configuration à importer",
                    zod::string().min(2),
                ),
            ),
            {
                let st = Arc::clone(state);
                move |args| {
                    let mut req = action_req("import_mcp", args);
                    req["text"] = args["text"].clone();
                    do_action(&st, req)
                }
            },
        )
        .annotate(
            "install_store_server",
            modifies("Installer depuis la boutique"),
        )
        .tool_typed(
            "install_store_server",
            "Installe un serveur du catalogue boutique en un clic (id du \
             catalogue, voir store_catalog).",
            zod::object()
                .field(
                    "id",
                    zod::describe("id du catalogue boutique", zod::string().min(1)),
                )
                .opt(
                    "args",
                    zod::describe(
                        "surcharges d'arguments (ex. dossier du serveur Filesystem)",
                        zod::array(zod::string()),
                    ),
                ),
            {
                let st = Arc::clone(state);
                move |args| do_action(&st, action_req("install_store", args))
            },
        )
        .annotate("ssh_save", modifies("Enregistrer une machine"))
        .tool_typed(
            "ssh_save",
            "Enregistre une machine distante (IP + utilisateur SSH, auth par clé) \
             pour la découverte de ses serveurs.",
            zod::object()
                .field("ip", zod::string().min(1))
                .field("user", zod::string().min(1)),
            {
                let st = Arc::clone(state);
                move |args| do_action(&st, action_req("ssh_save", args))
            },
        )
        .annotate("ssh_forget", modifies("Oublier une machine"))
        .tool_typed(
            "ssh_forget",
            "Retire une machine distante enregistrée.",
            zod::object().field("ip", zod::string().min(1)),
            {
                let st = Arc::clone(state);
                move |args| do_action(&st, action_req("ssh_forget", args))
            },
        )
        .annotate(
            "ssh_discover",
            modifies("Découvrir les serveurs d'une machine"),
        )
        .tool_typed(
            "ssh_discover",
            "Va chercher les serveurs MCP d'une machine distante enregistrée (via \
             SSH). Sans ip : interroge toutes les machines enregistrées.",
            zod::object().opt("ip", zod::string()),
            {
                let st = Arc::clone(state);
                move |args| do_action(&st, action_req("ssh_discover", args))
            },
        );

    b
}

// ---------- gateway : proxy vers les serveurs détectés ----------

/// Résout la cible proxy d'un serveur — pur et testé. `Err` = motif clair
/// pour le client MCP.
fn proxy_target(s: &crate::types::Server) -> Result<(String, u16, String), ToolError> {
    if !s.enabled {
        return Err(ToolError::invalid_input(format!(
            "serveur '{}' en pause (DISABLED) — toggle_server pour le réactiver",
            s.id
        )));
    }
    if s.state != SState::Up {
        return Err(ToolError::invalid_input(format!(
            "serveur '{}' à l'état {} : seul un serveur UP répond au proxy \
             (start_server pour le démarrer, refresh pour re-sonder)",
            s.id,
            s.state.as_str()
        )));
    }
    if s.via_ssh {
        return Err(ToolError::invalid_input(
            "serveur sondé via SSH : le proxy ne délivre que les serveurs \
             directement joignables (HTTP)",
        ));
    }
    match &s.transport {
        Transport::Stdio { .. } => Err(ToolError::invalid_input(
            "serveur stdio : le proxy tools/call ne couvre que les serveurs HTTP",
        )),
        Transport::Http { host, port, path } => {
            // bound_host = le host qui a réellement répondu à la dernière sonde
            // (::1 vs 127.0.0.1, IP LAN) : on appelle ce qui marche, pas ce
            // qui est écrit dans la config
            let h = s
                .bound_host
                .clone()
                .or_else(|| host.clone())
                .unwrap_or_else(|| "127.0.0.1".to_string());
            Ok((h, *port, path.clone()))
        }
    }
}

fn proxy_tools(b: ServerBuilder, state: &Arc<AppState>) -> ServerBuilder {
    let b = b
        .annotate("call_server_tool", modifies("Appeler un outil détecté"))
        .tool_typed(
            "call_server_tool",
            "Gateway : appelle un outil (tools/call) sur un serveur MCP détecté \
             par la console et renvoie sa réponse complète (content + \
             structuredContent de l'upstream). HTTP uniquement ; un outil coupé \
             dans l'UI (toggle_tool) est refusé.",
            zod::object()
                .field(
                    "server",
                    zod::describe(
                        "identifiant du serveur détecté (voir list_servers)",
                        zod::string().min(1),
                    ),
                )
                .field(
                    "tool",
                    zod::describe("nom de l'outil distant", zod::string().min(1)),
                )
                .opt(
                    "arguments",
                    zod::describe("arguments de l'outil distant", zod::object()),
                ),
            {
                let st = Arc::clone(state);
                move |args| {
                    let id = args["server"].as_str().unwrap_or_default();
                    let tool = args["tool"].as_str().unwrap_or_default();
                    let arguments = args.get("arguments").cloned().unwrap_or(json!({}));
                    let (host, port, path) = {
                        let servers = lockv(&st.servers);
                        let Some(s) = servers.iter().find(|s| s.id == id) else {
                            return Err(ToolError::invalid_input(format!(
                                "serveur inconnu : {id}"
                            )));
                        };
                        if s.tools_disabled.iter().any(|t| t == tool) {
                            return Err(ToolError::invalid_input(format!(
                                "outil '{tool}' coupé pour ce serveur dans l'UI \
                                 (toggle_tool pour le rétablir)"
                            )));
                        }
                        proxy_target(s)?
                    };
                    let rpc_id = PROXY_RPC_ID.fetch_add(1, Ordering::Relaxed);
                    let params = json!({ "name": tool, "arguments": arguments });
                    match probe::rpc_call_host(
                        &host,
                        port,
                        &path,
                        "tools/call",
                        params,
                        rpc_id,
                        PROXY_TIMEOUT_MS,
                    ) {
                        Ok((result, _head)) => {
                            // transparence : le résultat du serveur distant
                            // (CallToolResult upstream) est rendu tel quel —
                            // un isError distant reste visible par le client
                            Ok(result)
                        }
                        Err(e) => Err(ToolError::upstream(format!(
                            "appel de '{tool}' sur '{id}' ({host}:{port}{path}) : {e}"
                        ))),
                    }
                }
            },
        );
    b
}

// ---------- ressources dynamiques (une par serveur détecté) ----------

fn dynamic_resources(b: ServerBuilder, state: &Arc<AppState>) -> ServerBuilder {
    let mut b = b
        .resource(
            "mcp-control://servers",
            "Serveurs supervisés",
            "Aperçu complet : chaque serveur, son état, ses compteurs — le même JSON que l'écran.",
            {
                let st = Arc::clone(state);
                move || {
                    let full = manage::state_json(&st);
                    let doc = json!({
                        "stats": full["stats"],
                        "lan_ok": full["lan_ok"],
                        "servers": full["servers"],
                    });
                    Ok(ResourceContents {
                        uri: "mcp-control://servers".to_string(),
                        mime_type: Some("application/json".to_string()),
                        text: serde_json::to_string(&doc).unwrap_or_default(),
                    })
                }
            },
        )
        .resource(
            "mcp-control://journal",
            "Journal",
            "Dernières entrées du journal de la console.",
            {
                let st = Arc::clone(state);
                move || {
                    let log = lockv(&st.log);
                    let mut text = String::new();
                    for e in log.iter().rev().take(80) {
                        text.push_str(&format!(
                            "[{}] {} {}\n",
                            crate::types::iso_utc(e.ts),
                            e.level,
                            e.msg
                        ));
                    }
                    Ok(ResourceContents {
                        uri: "mcp-control://journal".to_string(),
                        mime_type: Some("text/plain; charset=utf-8".to_string()),
                        text,
                    })
                }
            },
        )
        .resource(
            "mcp-control://settings",
            "Réglages",
            "Réglages non sensibles de la console (jamais le code d'accès).",
            {
                let st = Arc::clone(state);
                move || {
                    let s = lockv(&st.settings);
                    let doc = json!({
                        "refresh_secs": s.refresh_secs,
                        "ui_port": s.ui_port,
                        "ui_bind": s.ui_bind,
                        "scan_from": s.scan_from,
                        "scan_to": s.scan_to,
                        "network_enabled": s.network_enabled,
                        "autostart": s.autostart,
                    });
                    Ok(ResourceContents {
                        uri: "mcp-control://settings".to_string(),
                        mime_type: Some("application/json".to_string()),
                        text: serde_json::to_string(&doc).unwrap_or_default(),
                    })
                }
            },
        );
    // une ressource par serveur DÉTECTÉ : le snapshot est reconstruit quand
    // l'empreinte de détection change (cf. fingerprint/refresh)
    let ids: Vec<(String, String)> = {
        let servers = lockv(&state.servers);
        servers
            .iter()
            .map(|s| (s.id.clone(), s.label.clone()))
            .collect()
    };
    for (id, label) in ids {
        let st = Arc::clone(state);
        let uri = format!("mcp-control://server/{id}");
        let name = format!("Serveur {label}");
        let uri_in = uri.clone();
        let id_in = id.clone();
        b = b.resource(
            &uri,
            &name,
            "Détail du serveur supervisé (état, outils, métriques).",
            move || {
                let detail = manage::detail_json(&st, &id_in);
                Ok(ResourceContents {
                    uri: uri_in.clone(),
                    mime_type: Some("application/json".to_string()),
                    text: serde_json::to_string(&detail).unwrap_or_default(),
                })
            },
        );
    }
    b
}

// ---------- transport /mcp (les règles de LiteMCP reflétées sur NOTRE socket) ----------

/// `Content-Type: application/json` strict (paramètres autorisés — charset
/// UTF-8 seulement ; jamais un suffixe `-patch+json` ni un utf-16).
fn is_json_content_type(v: &str) -> bool {
    let mut parts = v.split(';');
    let mime = parts.next().unwrap_or("").trim();
    if !mime.eq_ignore_ascii_case("application/json") {
        return false;
    }
    for param in parts {
        let param = param.trim();
        if let Some(charset) = param.strip_prefix("charset=").map(str::trim) {
            let charset = charset.trim_matches('"');
            if !charset.eq_ignore_ascii_case("utf-8") && !charset.eq_ignore_ascii_case("us-ascii") {
                return false;
            }
        }
    }
    !v.trim_end().ends_with(';') // `application/json;` = paramètre vide → refus
}

/// Poids nul dans un paramètre `q=` (RFC 9110 §12.5.1) : `0`, `0.0`,
/// `0.0000…` — toute suite de zéros.
fn is_zero_weight(q: &str) -> bool {
    let q = q.trim();
    let Some(rest) = q.strip_prefix('0') else {
        return false;
    };
    match rest.strip_prefix('.') {
        None => rest.is_empty(),
        Some(frac) => !frac.is_empty() && frac.chars().all(|c| c == '0'),
    }
}

/// Négociation `Accept` : jetons exacts (jamais par sous-chaîne —
/// `text/event-stream-x` ne compte pas), `q=0` disqualifie le type.
fn accept_media(v: &str) -> (bool, bool) {
    let mut json_ok = false;
    let mut sse_ok = false;
    for token in v.split(',') {
        let mut parts = token.split(';');
        let media = parts.next().unwrap_or("").trim().to_ascii_lowercase();
        let mut weight_zero = false;
        for p in parts {
            let p = p.trim();
            if let Some(q) = p.strip_prefix("q=") {
                if is_zero_weight(q) {
                    weight_zero = true;
                }
            }
        }
        match media.as_str() {
            "application/json" if !weight_zero => json_ok = true,
            "text/event-stream" if !weight_zero => sse_ok = true,
            _ => {}
        }
    }
    (json_ok, sse_ok)
}

/// Formate une charge JSON-RPC en événement SSE mono-ligne — tout `\r`/`\n`
/// est neutralisé (jamais d'injection de champs `event:`).
fn sse_event(reply: &[u8]) -> Vec<u8> {
    let mut event = Vec::with_capacity(reply.len() + 16);
    event.extend_from_slice(b"event: message\n");
    for line in reply.split(|&b| b == b'\n') {
        event.extend_from_slice(b"data: ");
        for &byte in line {
            if byte != b'\r' {
                event.push(byte);
            }
        }
        event.push(b'\n');
    }
    event.push(b'\n');
    event
}

fn err_json(status: u16, body: Value) -> (u16, &'static str, Vec<u8>) {
    (status, "application/json", body.to_string().into_bytes())
}

/// Extrait l'`id` JSON-RPC d'un corps — *best-effort*, pour la réponse -32603
/// de repli (JSON-RPC 2.0 §5 : l'id lisible est toujours repris).
fn extract_id(body: &[u8]) -> Value {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|v| v.get("id").cloned())
        .filter(|id| id.is_null() || id.is_string() || id.is_number())
        .unwrap_or(Value::Null)
}

/// Traite une requête HTTP sur `/mcp` — appelée par `handle_conn` APRÈS le
/// contrôle d'accès (Host/Origin/code hors loopback, cf. `check_access`).
/// Règles de transport identiques à LiteMCP : POST seul, `Content-Type`
/// JSON exigé, `Accept` négocié (SSE mono-événement), en-têtes SEP-2243
/// vérifiés, notification → 202 sans corps.
pub fn serve(
    method: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> (u16, &'static str, Vec<u8>) {
    let Some(gw) = current() else {
        return (
            503,
            "application/json",
            br#"{"ok":false,"error":"gateway indisponible"}"#.to_vec(),
        );
    };
    respond(gw.snapshot().as_ref(), method, headers, body)
}

/// Les règles de transport `/mcp`, appliquées à un noyau explicite — pure et
/// testée sans le slot global.
fn respond(
    core: &ServerCore,
    method: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> (u16, &'static str, Vec<u8>) {
    if method != "POST" {
        return (
            405,
            "application/json",
            br#"{"ok":false,"error":"method not allowed: POST /mcp only (stateless transport)"}"#
                .to_vec(),
        );
    }
    // Content-Type : un seul, application/json strict (415) ; dupliqué → 400
    let mut ct_json = false;
    let mut ct_seen = false;
    for (name, value) in headers {
        if name == "content-type" {
            if ct_seen {
                return err_json(
                    400,
                    json!({"ok": false, "error": "duplicate Content-Type header"}),
                );
            }
            ct_seen = true;
            ct_json = is_json_content_type(value);
        }
    }
    if !ct_json {
        return err_json(
            415,
            json!({"ok": false, "error": "expected Content-Type: application/json"}),
        );
    }
    // Accept : si l'en-tête existe, au moins un type acceptable est requis
    let mut accept_json = false;
    let mut accept_sse = false;
    let mut accept_seen = false;
    for (name, value) in headers {
        if name == "accept" {
            accept_seen = true;
            let (j, s) = accept_media(value);
            accept_json |= j;
            accept_sse |= s;
        }
    }
    if accept_seen && !accept_json && !accept_sse {
        return err_json(
            406,
            json!({"ok": false, "error": "not acceptable: serve application/json or text/event-stream"}),
        );
    }
    // En-têtes de routage MCP (SEP-2243) : transmis au noyau, qui vérifie
    // leur cohérence exacte avec le corps (duplicata → 400 ici).
    let mut mcp_method: Option<String> = None;
    let mut mcp_name: Option<String> = None;
    for (name, value) in headers {
        match name.as_str() {
            "mcp-method" => {
                if mcp_method.is_some() {
                    return err_json(
                        400,
                        json!({"ok": false, "error": "duplicate Mcp-Method header"}),
                    );
                }
                mcp_method = Some(value.clone());
            }
            "mcp-name" => {
                if mcp_name.is_some() {
                    return err_json(
                        400,
                        json!({"ok": false, "error": "duplicate Mcp-Name header"}),
                    );
                }
                mcp_name = Some(value.clone());
            }
            _ => {}
        }
    }
    let headers_mcp = if mcp_method.is_none() && mcp_name.is_none() {
        None
    } else {
        Some(McpHeaders {
            method: mcp_method,
            name: mcp_name,
        })
    };
    // dispatch sur le noyau — panique de repli → -32603 avec id repris
    let dispatched = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        core.dispatch_with_headers(body, headers_mcp.as_ref())
    }));
    let reply = match dispatched {
        Ok(Some(reply)) => reply,
        Ok(None) => {
            // notification : aucune réponse JSON-RPC attendue
            return (202, "application/json", Vec::new());
        }
        Err(_) => {
            let payload = json!({
                "jsonrpc": "2.0",
                "id": extract_id(body),
                "error": { "code": ErrorObject::INTERNAL_ERROR, "message": "internal error" }
            });
            return (200, "application/json", payload.to_string().into_bytes());
        }
    };
    if accept_sse && !accept_json {
        return (200, "text/event-stream", sse_event(&reply));
    }
    (200, "application/json", reply)
}

// ---------- tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Origin, Server};
    use std::collections::{HashMap, HashSet, VecDeque};

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            servers: std::sync::Mutex::new(vec![]),
            log: std::sync::Mutex::new(VecDeque::new()),
            settings: std::sync::Mutex::new(crate::types::Settings::default()),
            children: std::sync::Mutex::new(HashMap::new()),
            peers: std::sync::Mutex::new(HashMap::new()),
            allowed_origins: std::sync::Mutex::new(Vec::new()),
            allowed_hosts: std::sync::Mutex::new(vec!["127.0.0.1".to_string()]),
            spawn_fails: std::sync::Mutex::new(HashMap::new()),
            runtime: std::sync::Mutex::new(crate::runtime::RuntimeLog::new()),
            spawning: std::sync::Mutex::new(HashSet::new()),
            update_latest: std::sync::Mutex::new(None),
            lan_ok: std::sync::Mutex::new(None),
            config_path: String::new(),
        })
    }

    fn http_server(id: &str, port: u16) -> Server {
        Server {
            id: id.to_string(),
            label: id.to_string(),
            transport: Transport::Http {
                host: None,
                port,
                path: "/mcp".to_string(),
            },
            origin: Origin::Manual,
            enabled: true,
            state: SState::Up,
            pid: None,
            bound_host: Some("127.0.0.1".to_string()),
            server_name: Some("test".to_string()),
            version: Some("1.0".to_string()),
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
        }
    }

    fn hs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect()
    }

    fn rpc(method: &str, id: u64) -> String {
        json!({ "jsonrpc": "2.0", "id": id, "method": method }).to_string()
    }

    /// Noms publiés par un noyau via son `tools/list`.
    fn tools_list_names(core: &ServerCore) -> Vec<String> {
        let reply = core.dispatch(rpc("tools/list", 0).as_bytes()).unwrap();
        let v: Value = serde_json::from_slice(&reply).unwrap();
        v["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap_or("").to_string())
            .collect()
    }

    // ---------- transport ----------

    #[test]
    fn content_type_json_strict() {
        for ok in [
            "application/json",
            "application/json; charset=utf-8",
            "application/json;charset=UTF-8",
            "application/json; boundary=something",
        ] {
            assert!(is_json_content_type(ok), "{ok} doit passer");
        }
        for ko in [
            "application/json-patch+json",
            "application/json, text/plain",
            "application/json; charset=utf-16",
            "application/json; charset=iso-8859-1",
            "application/json;",
            "text/plain",
            "",
        ] {
            assert!(!is_json_content_type(ko), "{ko} doit être refusé");
        }
    }

    #[test]
    fn poids_nul_q_zero_sous_toutes_les_formes() {
        for zero in ["0", "0.0", "0.00", "0.0000", "0.000000"] {
            assert!(is_zero_weight(zero), "q={zero} = poids nul");
        }
        for non_zero in ["1", "0.1", "0.01", "0.5", "1.0", "x", "", ".5"] {
            assert!(!is_zero_weight(non_zero), "q={non_zero} n'est PAS nul");
        }
    }

    #[test]
    fn accept_jeton_exact_et_q_zero() {
        assert_eq!(accept_media("application/json"), (true, false));
        assert_eq!(accept_media("text/event-stream"), (false, true));
        assert_eq!(
            accept_media("application/json, text/event-stream"),
            (true, true)
        );
        // q=0 disqualifie le type, l'autre token reste
        assert_eq!(
            accept_media("application/json;q=0, text/event-stream"),
            (false, true)
        );
        assert_eq!(accept_media("application/json;q=0.0000"), (false, false));
        // jamais par sous-chaîne
        assert_eq!(accept_media("text/event-stream-x"), (false, false));
        // casse et espaces
        assert_eq!(accept_media(" Application/JSON "), (true, false));
    }

    #[test]
    fn sse_event_neutralise_les_sauts_de_ligne() {
        let out = sse_event(br#"{"jsonrpc":"2.0","id":1,"result":{}}"#);
        let text = String::from_utf8_lossy(&out).to_string();
        assert!(text.starts_with("event: message\ndata: "), "{text}");
        assert!(text.ends_with("\n\n"));
        // injection de champs SSE via un corps hostile → neutralisée : le
        // seul champ `event:` est le nôtre, tout le reste est du `data:`
        let hostile = sse_event(b"line1\r\nevent: pwned\ndata: fake");
        let text = String::from_utf8_lossy(&hostile).to_string();
        let event_lines = text.lines().filter(|l| l.starts_with("event:")).count();
        assert_eq!(event_lines, 1, "{text}");
        assert!(text.contains("data: event: pwned"), "{text}");
    }

    #[test]
    fn respond_refuse_get_et_content_type_non_json() {
        let core = build_builder(&test_state()).core();
        assert_eq!(respond(&core, "GET", &hs(&[]), b"").0, 405);
        assert_eq!(respond(&core, "DELETE", &hs(&[]), b"").0, 405);
        let r = respond(&core, "POST", &hs(&[("content-type", "text/plain")]), b"{}");
        assert_eq!(r.0, 415);
        let r = respond(
            &core,
            "POST",
            &hs(&[("content-type", "application/json; charset=utf-16")]),
            b"{}",
        );
        assert_eq!(r.0, 415);
        // en-tête dupliqué → 400
        let r = respond(
            &core,
            "POST",
            &hs(&[
                ("content-type", "application/json"),
                ("content-type", "application/json"),
            ]),
            b"{}",
        );
        assert_eq!(r.0, 400);
        // accept sans type acceptable → 406
        let r = respond(
            &core,
            "POST",
            &hs(&[
                ("content-type", "application/json"),
                ("accept", "text/html"),
            ]),
            b"{}",
        );
        assert_eq!(r.0, 406);
    }

    #[test]
    fn serve_sans_gateway_rend_503() {
        // aucun test unitaire n'installe le gateway global (E2E, process
        // séparé, couvre le chemin installé) — le slot est donc vide ici
        assert!(
            current().is_none(),
            "le slot global doit rester vide en tests unitaires"
        );
        let r = serve("POST", &hs(&[("content-type", "application/json")]), b"{}");
        assert_eq!(r.0, 503);
    }

    #[test]
    fn notification_rend_202_sans_corps() {
        // test isolé du format : un dispatch litemcp sur une notification → None
        let core = ServerBuilder::new("t", "1.0")
            .tool("a", "x", |_a| Ok(json!({})))
            .core();
        assert!(core
            .dispatch(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none());
    }

    #[test]
    fn extract_id_reprend_l_id_lisible_et_rejete_le_reste() {
        assert_eq!(extract_id(br#"{"id":42}"#), json!(42));
        assert_eq!(extract_id(br#"{"id":"abc"}"#), json!("abc"));
        assert_eq!(extract_id(br#"{"id":null}"#), Value::Null);
        assert_eq!(extract_id(br#"{"id":{"x":1}}"#), Value::Null);
        assert_eq!(extract_id(br#"pas du json"#), Value::Null);
    }

    // ---------- snapshot : la surface complète de l'UI ----------

    #[test]
    fn snapshot_contient_toute_la_surface_supervision() {
        let state = test_state();
        let core = build_builder(&state).core();
        let names = tools_list_names(&core);
        for attendu in [
            "list_servers",
            "server_detail",
            "list_server_tools",
            "journal",
            "runtime_history",
            "list_peers",
            "store_catalog",
            "list_frameworks",
            "get_settings",
            "refresh",
            "scan_ports",
            "scan_network",
            "discover_peers",
            "check_update",
            "add_server",
            "remove_server",
            "toggle_server",
            "start_server",
            "stop_server",
            "set_start_command",
            "set_host",
            "toggle_tool",
            "set_auto_restart",
            "reorder_servers",
            "import_config",
            "install_store_server",
            "ssh_save",
            "ssh_forget",
            "ssh_discover",
            "call_server_tool",
        ] {
            assert!(
                names.contains(&attendu.to_string()),
                "outil manquant : {attendu}"
            );
        }
        // ressources : aperçu + journal + réglages + une par serveur détecté
        state
            .servers
            .lock()
            .unwrap()
            .push(http_server("fixture-a", 30_001));
        let core = build_builder(&state).core();
        let reply = core.dispatch(rpc("resources/list", 1).as_bytes()).unwrap();
        let v: Value = serde_json::from_slice(&reply).unwrap();
        let uris: Vec<String> = v["result"]["resources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["uri"].as_str().unwrap_or("").to_string())
            .collect();
        assert!(uris.contains(&"mcp-control://servers".to_string()));
        assert!(uris.contains(&"mcp-control://journal".to_string()));
        assert!(uris.contains(&"mcp-control://settings".to_string()));
        assert!(
            uris.contains(&"mcp-control://server/fixture-a".to_string()),
            "ressource par serveur manquante : {uris:?}"
        );
        // la lecture de la ressource serveur renvoie le détail JSON
        let reply = core
            .dispatch(
                json!({"jsonrpc":"2.0","id":2,"method":"resources/read",
                       "params":{"uri":"mcp-control://server/fixture-a"}})
                .to_string()
                .as_bytes(),
            )
            .unwrap();
        let v: Value = serde_json::from_slice(&reply).unwrap();
        let text = v["result"]["contents"][0]["text"].as_str().unwrap_or("");
        let detail: Value = serde_json::from_str(text).unwrap_or(Value::Null);
        assert_eq!(detail["id"], "fixture-a");
    }

    #[test]
    fn list_servers_refleche_l_etat_en_direct_et_le_filtre() {
        let state = test_state();
        {
            let mut servers = state.servers.lock().unwrap();
            servers.push(http_server("up-srv", 30_101));
            let mut down = http_server("down-srv", 30_102);
            down.state = SState::Down;
            servers.push(down);
        }
        let core = build_builder(&state).core();
        let call = |args: Value| {
            let body = json!({
                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": { "name": "list_servers", "arguments": args }
            })
            .to_string();
            let reply = core.dispatch(body.as_bytes()).unwrap();
            serde_json::from_slice::<Value>(&reply).unwrap()["result"]["structuredContent"].clone()
        };
        let full = call(json!({}));
        assert_eq!(full["stats"]["total"], 2);
        assert_eq!(full["servers"].as_array().unwrap().len(), 2);
        let ups = call(json!({ "state": "up" }));
        assert_eq!(ups["servers"].as_array().unwrap().len(), 1);
        assert_eq!(ups["servers"][0]["id"], "up-srv");
        let downs = call(json!({ "state": "down" }));
        assert_eq!(downs["servers"][0]["id"], "down-srv");
    }

    #[test]
    fn tools_validation_schema_avant_execution() {
        let state = test_state();
        let core = build_builder(&state).core();
        // server_detail sans id → -32602 avec issues, jamais le handler
        let body = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "server_detail", "arguments": {} }
        })
        .to_string();
        let reply = core.dispatch(body.as_bytes()).unwrap();
        let v: Value = serde_json::from_slice(&reply).unwrap();
        assert_eq!(v["error"]["code"], -32602, "{v}");
        assert!(!v["error"]["data"]["issues"].as_array().unwrap().is_empty());
    }

    #[test]
    fn journal_tool_renvoie_les_dernieres_entrees() {
        let state = test_state();
        manage::add_log(&state, "info", "entrée-1");
        manage::add_log(&state, "warn", "entrée-2");
        let core = build_builder(&state).core();
        let body = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "journal", "arguments": { "limit": 2 } }
        })
        .to_string();
        let reply = core.dispatch(body.as_bytes()).unwrap();
        let v: Value = serde_json::from_slice(&reply).unwrap();
        let entries = v["result"]["structuredContent"]["entries"]
            .as_array()
            .unwrap();
        assert_eq!(entries.len(), 2);
        // plus récent d'abord
        assert_eq!(entries[0]["msg"], "entrée-2");
        assert_eq!(entries[1]["msg"], "entrée-1");
    }

    #[test]
    fn toggle_par_mcp_change_letat_et_le_journal_gateway_se_remplit() {
        let state = test_state();
        state
            .servers
            .lock()
            .unwrap()
            .push(http_server("srv-t", 30_201));
        let core = build_builder(&state).core();
        let body = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "toggle_server", "arguments": { "id": "srv-t" } }
        })
        .to_string();
        let reply = core.dispatch(body.as_bytes()).unwrap();
        let v: Value = serde_json::from_slice(&reply).unwrap();
        assert_eq!(v["result"]["structuredContent"]["ok"], true, "{v}");
        // l'interrupteur a réellement bougé dans l'état (contrat dispatcher)
        let now_enabled = state.servers.lock().unwrap()[0].enabled;
        assert!(!now_enabled, "toggle via MCP doit couper le serveur");
        // le hook on_call a écrit dans le journal
        let log = lockv(&state.log);
        assert!(
            log.iter().any(|e| e.msg.contains("[gateway] tools/call")),
            "le journal doit raconter l'activité gateway"
        );
    }

    #[test]
    fn respond_dispatch_tools_list_avec_les_meta_litemcp() {
        let core = build_builder(&test_state()).core();
        let headers = hs(&[("content-type", "application/json")]);
        let (status, ctype, payload) =
            respond(&core, "POST", &headers, rpc("tools/list", 7).as_bytes());
        assert_eq!(status, 200);
        assert_eq!(ctype, "application/json");
        let v: Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(v["id"], 7);
        assert!(v["result"]["tools"].as_array().unwrap().len() >= 28);
        assert_eq!(v["result"]["resultType"], "complete");
    }

    #[test]
    fn respond_sse_mono_evenement_quand_le_client_ne_veut_que_ca() {
        let core = build_builder(&test_state()).core();
        let headers = hs(&[
            ("content-type", "application/json"),
            ("accept", "text/event-stream"),
        ]);
        let (status, ctype, payload) = respond(&core, "POST", &headers, rpc("ping", 1).as_bytes());
        assert_eq!(status, 200);
        assert_eq!(ctype, "text/event-stream");
        let text = String::from_utf8_lossy(&payload).to_string();
        assert!(text.starts_with("event: message\ndata: "), "{text}");
        // le client JSON (défaut) reçoit du JSON brut
        let headers = hs(&[
            ("content-type", "application/json"),
            ("accept", "application/json, text/event-stream"),
        ]);
        let (_, ctype, _) = respond(&core, "POST", &headers, rpc("ping", 1).as_bytes());
        assert_eq!(ctype, "application/json");
    }

    #[test]
    fn respond_verification_en_tetes_sep_2243() {
        let core = build_builder(&test_state()).core();
        let body = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "list_servers", "arguments": {} }
        })
        .to_string();
        // en-têtes cohérents → réponse normale
        let headers = hs(&[
            ("content-type", "application/json"),
            ("mcp-method", "tools/call"),
            ("mcp-name", "list_servers"),
        ]);
        let (status, _, payload) = respond(&core, "POST", &headers, body.as_bytes());
        assert_eq!(status, 200);
        let v: Value = serde_json::from_slice(&payload).unwrap();
        assert!(v.get("result").is_some(), "{v}");
        // Mcp-Method incohérent avec le corps → -32020
        let headers = hs(&[("content-type", "application/json"), ("mcp-method", "ping")]);
        let (_, _, payload) = respond(&core, "POST", &headers, body.as_bytes());
        let v: Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(v["error"]["code"], -32020);
        // duplicata d'en-tête → 400
        let headers = hs(&[
            ("content-type", "application/json"),
            ("mcp-method", "tools/call"),
            ("mcp-method", "ping"),
        ]);
        let (status, _, _) = respond(&core, "POST", &headers, body.as_bytes());
        assert_eq!(status, 400);
    }

    #[test]
    fn notification_202_via_respond() {
        let core = build_builder(&test_state()).core();
        let headers = hs(&[("content-type", "application/json")]);
        let (status, _, payload) = respond(
            &core,
            "POST",
            &headers,
            br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        );
        assert_eq!(status, 202);
        assert!(payload.is_empty());
    }

    // ---------- empreinte + proxy ----------

    #[test]
    fn fingerprint_change_avec_la_detection() {
        let state = test_state();
        let f0 = fingerprint(&state);
        state
            .servers
            .lock()
            .unwrap()
            .push(http_server("fp-a", 30_301));
        let f1 = fingerprint(&state);
        assert_ne!(f0, f1, "un serveur de plus doit changer l'empreinte");
        // toggle d'un outil : même serveur, empreinte différente
        state.servers.lock().unwrap()[0]
            .tools_disabled
            .push("ping".to_string());
        let f2 = fingerprint(&state);
        assert_ne!(f1, f2);
        // état identique reconstruit → même empreinte
        let f2_bis = fingerprint(&state);
        assert_eq!(f2, f2_bis);
    }

    #[test]
    fn proxy_target_refuse_les_etats_non_up_et_stdio() {
        let mut s = http_server("p", 30_401);
        // UP + HTTP → cible
        let (host, port, path) = proxy_target(&s).unwrap();
        assert_eq!(
            (host.as_str(), port, path.as_str()),
            ("127.0.0.1", 30_401, "/mcp")
        );
        // DISABLED → refus
        s.enabled = false;
        assert!(proxy_target(&s).is_err());
        s.enabled = true;
        // DOWN → refus
        s.state = SState::Down;
        assert!(proxy_target(&s).is_err());
        s.state = SState::Up;
        // via_ssh → refus
        s.via_ssh = true;
        assert!(proxy_target(&s).is_err());
        s.via_ssh = false;
        // stdio → refus
        s.transport = Transport::Stdio {
            program: "node".to_string(),
            args: vec![],
            workdir: None,
        };
        assert!(proxy_target(&s).is_err());
        // bound_host prioritaire sur le host de config
        s.transport = Transport::Http {
            host: Some("192.0.2.99".to_string()),
            port: 30_402,
            path: "/mcp".to_string(),
        };
        let (host, _, _) = proxy_target(&s).unwrap();
        assert_eq!(host, "127.0.0.1", "bound_host gagne");
        s.bound_host = None;
        let (host, _, _) = proxy_target(&s).unwrap();
        assert_eq!(host, "192.0.2.99", "retour au host de config");
    }

    #[test]
    fn call_server_tool_refuse_serveur_inconnu_et_outil_coupe() {
        let state = test_state();
        let mut s = http_server("srv-cut", 30_501);
        s.tools_disabled.push("ping".to_string());
        state.servers.lock().unwrap().push(s);
        let core = build_builder(&state).core();
        let call = |server: &str, tool: &str| {
            let body = json!({
                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": { "name": "call_server_tool",
                            "arguments": { "server": server, "tool": tool } }
            })
            .to_string();
            let reply = core.dispatch(body.as_bytes()).unwrap();
            serde_json::from_slice::<Value>(&reply).unwrap()
        };
        // serveur inconnu → isError avec message clair
        let v = call("inexistant", "ping");
        assert_eq!(v["result"]["isError"], true, "{v}");
        let err = v["result"]["structuredContent"]["error"].clone();
        assert!(
            err["message"]
                .as_str()
                .unwrap_or("")
                .contains("serveur inconnu"),
            "{err}"
        );
        // outil coupé dans l'UI → refus explicite AVANT tout appel réseau
        let v = call("srv-cut", "ping");
        let err = v["result"]["structuredContent"]["error"].clone();
        assert!(
            err["message"]
                .as_str()
                .unwrap_or("")
                .contains("coupé pour ce serveur"),
            "{err}"
        );
    }
}
