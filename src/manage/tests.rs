
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
                favorite: false,
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
            favorite: false,
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
            favorite: false,
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
            favorite: false,
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
                favorite: false,
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
                favorite: false,
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
                favorite: false,
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
        favorite: false,
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
            favorite: false,
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
            favorite: false,
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
            favorite: false,
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
            favorite: false,
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
            favorite: false,
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
            favorite: false,
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
            favorite: false,
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
            favorite: false,
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
            favorite: false,
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
fn set_notify_valide_et_persiste() {
    // Panneau Réglages → Notifications : toast desktop + webhook. URL
    // invalide refusée AVANT tout, méthode normalisée, min_interval
    // clampé, config persistée, et état exposé à l'UI (/api/state).
    let path = tmp_config("cfg_set_notify.json", Some("{\"servers\":[]}"));
    let mut base = match Arc::try_unwrap(test_state()) {
        Ok(s) => s,
        Err(_) => panic!("test_state doit être unique"),
    };
    base.config_path = path.clone();
    let state = Arc::new(base);

    // 1. URL invalide → refus net, rien de persisté
    let j = handle_action(
        &state,
        "{\"action\":\"set_notify\",\"webhook_url\":\"ftp://non\"}",
    );
    assert_eq!(j["ok"], serde_json::json!(false));
    assert!(j["error"].as_str().unwrap_or("").contains("invalide"));
    assert!(lockv(&state.settings).notify.webhook_url.is_empty());

    // 2. config complète → ok + état rendu + persisté dans config.json
    let j2 = handle_action(
            &state,
            "{\"action\":\"set_notify\",\"desktop\":true,\"webhook_url\":\"https://example.com/hook\",\"method\":\"put\",\"headers\":\"Authorization: Bearer x\",\"states\":[\"DOWN\"],\"min_interval_secs\":60}",
        );
    assert_eq!(j2["ok"], serde_json::json!(true));
    assert_eq!(j2["notify"]["desktop"], serde_json::json!(true));
    assert_eq!(
        j2["notify"]["webhook_url"],
        serde_json::json!("https://example.com/hook")
    );
    assert_eq!(
        j2["notify"]["method"],
        serde_json::json!("PUT"),
        "méthode normalisée"
    );
    assert_eq!(j2["notify"]["states"], serde_json::json!(["DOWN"]));
    {
        let st = lockv(&state.settings);
        assert!(st.notify.desktop);
        assert_eq!(st.notify.webhook_url, "https://example.com/hook");
        assert_eq!(st.notify.min_interval_secs, 60);
    }
    let cfg = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(cfg.contains("example.com/hook"), "persisté : {}", cfg);

    // 3. min_interval hors bornes → clamp 0..86400
    let j3 = handle_action(
        &state,
        "{\"action\":\"set_notify\",\"min_interval_secs\":999999}",
    );
    assert_eq!(j3["ok"], serde_json::json!(true));
    assert_eq!(lockv(&state.settings).notify.min_interval_secs, 86400);

    // 4. /api/state expose la config notifications (panneau UI) ; le
    //    journal raconte l'enregistrement
    let full = state_json(&state);
    assert_eq!(
        full["settings"]["notify"]["webhook_url"],
        serde_json::json!("https://example.com/hook")
    );
    let log = lockv(&state.log);
    assert!(
        log.iter()
            .any(|e| e.msg.contains("[notif] configuration enregistrée")),
        "le journal doit raconter l'enregistrement"
    );
}

#[test]
fn test_notify_refuse_sans_canal_et_rapporte_les_echecs() {
    // Bouton « Tester » : sans canal actif → refus explicite (l'UI affiche
    // pourquoi) ; webhook invalide → échec rapporté PAR CANAL, sans panic
    // et sans requête réseau (la validation URL précède tout envoi).
    let state = test_state();
    let j = handle_action(&state, "{\"action\":\"test_notify\"}");
    assert_eq!(j["ok"], serde_json::json!(false));
    assert!(j["error"]
        .as_str()
        .unwrap_or("")
        .contains("aucun canal actif"));
    {
        let mut st = lockv(&state.settings);
        st.notify.webhook_url = "notaurl".into();
    }
    let j2 = handle_action(&state, "{\"action\":\"test_notify\"}");
    assert_eq!(j2["ok"], serde_json::json!(true));
    let wh = j2["webhook"].as_object().expect("canal webhook rapporté");
    assert_eq!(wh["ok"], serde_json::json!(false));
    assert!(wh["error"].as_str().unwrap_or("").contains("invalide"));
    // canal desktop jamais activé → absent de la réponse (pas d'échec inventé)
    assert!(j2["desktop"].is_null());
}

#[test]
fn download_update_refuse_les_plateformes_hors_table() {
    // os fourni mais invalide → refus AVANT tout lancement de navigateur
    // (aucun repli silencieux sur la plateforme du moteur, aucun effet de
    // bord visible) ; l'UI affiche l'erreur telle quelle.
    let state = test_state();
    let j = handle_action(&state, "{\"action\":\"download_update\",\"os\":\"beos\"}");
    assert_eq!(j["ok"], serde_json::json!(false));
    assert!(j["error"].as_str().unwrap_or("").contains("os inconnu"));
}

#[test]
fn release_asset_table_blanche_couvre_toutes_les_plateformes() {
    // 6 combos : l'installateur desktop par OS + la CLI par OS. La table
    // est la SEULE source d'URL — hors table, rien n'est fabriqué.
    for (kind, os, attendu) in [
        (
            "desktop",
            "windows",
            "mcp-control-desktop-windows-x86_64-setup.exe",
        ),
        ("desktop", "macos", "mcp-control-desktop-macos-universal"),
        ("desktop", "linux", "mcp-control-desktop-linux-x86_64"),
        ("cli", "windows", "mcp-control-cli-windows-x86_64.exe"),
        ("cli", "macos", "mcp-control-cli-macos-universal"),
        ("cli", "linux", "mcp-control-cli-linux-x86_64"),
    ] {
        assert_eq!(release_asset(kind, os), Some(attendu), "{kind}/{os}");
    }
    assert_eq!(release_asset("cli", "beos"), None);
    assert_eq!(release_asset("autre", "windows"), None);
    assert_eq!(release_asset("", ""), None);
}

#[test]
fn surface_actions_sans_doublon_et_dispatch_connu() {
    // La const partagée (UI / gateway / tests) ne doit jamais dériver du
    // dispatcher : pas de doublon, et un échantillon dispatché doit
    // répondre SANS « action inconnue ». Sont écartés de l'échantillon les
    // actions à effet de bord réel (scan, discover, ssh, open_path,
    // check_update, open_release_page) — couvertes par la suite E2E.
    let mut seen = std::collections::HashSet::new();
    for a in ACTIONS {
        assert!(seen.insert(*a), "action en double dans ACTIONS : {a}");
    }
    let state = test_state();
    for a in [
        "toggle",
        "refresh",
        "start",
        "kill",
        "kill_stdio",
        "start_stdio",
        "remove",
        "toggle_tool",
        "set_settings",
        "set_notify",
        "set_autostart",
        "reset_runtime",
        "reorder",
        "install_store",
    ] {
        let j = handle_action(&state, &format!("{{\"action\":\"{a}\"}}"));
        let err = j["error"].as_str().unwrap_or("");
        assert!(
            !err.contains("action inconnue"),
            "{a} listée dans ACTIONS mais rejetée par le dispatcher : {err}"
        );
    }
    // et l'inverse : une action hors surface est rejetée nommément
    let j = handle_action(&state, "{\"action\":\"cette_action_n_existe_pas\"}");
    assert!(j["error"]
        .as_str()
        .unwrap_or("")
        .contains("action inconnue"));
}

#[test]
fn set_favorite_epingle_persiste_et_se_relit() {
    // Bouton ⭐ de l'explorateur : épingler un serveur persiste le favori
    // dans config.json (seulement quand actif — défaut false), survit à la
    // relecture, est exposé à l'UI, et se retire proprement.
    let path = tmp_config("cfg_set_favorite.json", Some("{\"servers\":[]}"));
    let mut base = match Arc::try_unwrap(test_state()) {
        Ok(s) => s,
        Err(_) => panic!("test_state doit être unique"),
    };
    base.config_path = path.clone();
    let state = Arc::new(base);
    lockv(&state.servers).push(Server {
        id: "fav-srv".into(),
        label: "FAV".into(),
        transport: Transport::Http {
            host: None,
            port: 3_211,
            path: "/mcp".into(),
        },
        origin: Origin::Manual,
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
        start_cmd: None,
        tools_disabled: vec![],
    });

    // 1. épingle → ok + état interne + persisté dans config.json
    let j = handle_action(
        &state,
        "{\"action\":\"set_favorite\",\"id\":\"fav-srv\",\"enabled\":true}",
    );
    assert_eq!(j["ok"], serde_json::json!(true));
    assert_eq!(j["favorite"], serde_json::json!(true));
    assert!(lockv(&state.servers)[0].favorite);
    let cfg = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(cfg.contains("\"favorite\": true"), "persisté : {}", cfg);

    // 2. /api/state l'expose (l'UI épingle d'après ce champ)
    let full = state_json(&state);
    assert_eq!(full["servers"][0]["favorite"], serde_json::json!(true));

    // 3. relecture : le favori survit au redémarrage du moteur
    let (servers, _) = load_config(&path);
    let s = servers.iter().find(|s| s.id == "fav-srv").unwrap();
    assert!(s.favorite, "le favori doit être relu du disque");

    // 4. retrait → la clé disparaît (défaut false, config lisible)
    let j2 = handle_action(
        &state,
        "{\"action\":\"set_favorite\",\"id\":\"fav-srv\",\"enabled\":false}",
    );
    assert_eq!(j2["ok"], serde_json::json!(true));
    let cfg2 = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(!cfg2.contains("favorite"), "défaut non écrit : {}", cfg2);

    // 5. serveur inconnu → erreur nette
    let j3 = handle_action(
        &state,
        "{\"action\":\"set_favorite\",\"id\":\"nope\",\"enabled\":true}",
    );
    assert_eq!(j3["ok"], serde_json::json!(false));
    assert_eq!(j3["error"], serde_json::json!("serveur inconnu"));
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
            favorite: false,
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
            favorite: false,
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
            favorite: false,
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
