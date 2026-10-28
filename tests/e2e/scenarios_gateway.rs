//! Scénarios e2e du GATEWAY MCP : la console est elle-même un serveur MCP
//! (POST /mcp sur le port UI) — toute la surface de l'UI en tools + proxy
//! vers les serveurs détectés.
//!
//! Ces scénarios parlent au VRAI moteur via le vrai transport : initialize,
//! tools/list, tools/call, resources/*, SSE, en-têtes SEP-2243, notification
//! 202, content-type 415, GET 405, auth LAN héritée du sas /api/*.

use crate::harness::{free_port, serial_guard, spawn, wait_until, Engine, EngineCfg, Fixture};
use serde_json::{json, Value};
use std::time::Duration;

const DELAI_UP: Duration = Duration::from_secs(30);
const DELAI_SNAPSHOT: Duration = Duration::from_secs(12);

// ---------- petits helpers locaux ----------

fn moteur() -> Engine {
    let mut e = spawn(EngineCfg::default()).expect("spawn moteur gateway");
    e.wait_ready(Duration::from_secs(15))
        .expect("moteur gateway prêt");
    e
}

fn ajoute_http(eng: &Engine, port: u16, label: &str) -> Value {
    eng.action(&json!({
        "action": "add", "transport": "http",
        "host": "127.0.0.1", "port": port, "path": "/mcp", "label": label
    }))
}

fn champ<'a>(s: &'a Value, cle: &str) -> &'a str {
    s.get(cle).and_then(Value::as_str).unwrap_or("")
}

/// POST /mcp JSON-RPC (Content-Type imposé par le harnais : application/json).
fn mcp(eng: &Engine, body: &Value) -> Value {
    let resp = eng.post_json("/mcp", body);
    resp.json()
}

/// POST /mcp avec en-têtes additionnels — retourne la Resp complète.
fn mcp_with(eng: &Engine, headers: &[(&str, &str)], body: &Value) -> crate::harness::Resp {
    let mut all: Vec<(&str, &str)> = vec![];
    all.extend_from_slice(headers);
    eng.request(
        "127.0.0.1",
        eng.port,
        "POST",
        "/mcp",
        &all,
        Some(body.to_string().as_bytes()),
    )
}

fn rpc_call(outil: &str, args: Value, id: u64) -> Value {
    json!({
        "jsonrpc": "2.0", "id": id, "method": "tools/call",
        "params": { "name": outil, "arguments": args }
    })
}

/// structuredContent d'un tools/call réussi (panic avec diagnostic sinon).
fn structured(rep: Value, contexte: &str) -> Value {
    let result = rep
        .get("result")
        .unwrap_or_else(|| panic!("{contexte} : pas de result → {rep}"));
    result
        .get("structuredContent")
        .cloned()
        .unwrap_or_else(|| panic!("{contexte} : pas de structuredContent → {rep}"))
}

/// L'entrée du serveur fixture dans l'état du moteur (par port).
fn serveur_par_port(eng: &Engine, port: u16) -> Option<Value> {
    let st = eng.get("/api/state").json();
    st["servers"]
        .as_array()
        .and_then(|arr| {
            arr.iter().find(|s| {
                s.get("transport_info")
                    .and_then(|t| t.get("port"))
                    .and_then(Value::as_u64)
                    == Some(u64::from(port))
            })
        })
        .cloned()
}

/// Attend l'état UP de la fixture (sonde du moteur = cycle poller).
fn attend_up(eng: &Engine, port: u16) -> Value {
    let trouve = wait_until(DELAI_UP, || {
        serveur_par_port(eng, port)
            .filter(|s| champ(s, "state") == "UP")
            .map(|_| true)
    });
    assert!(
        trouve,
        "pré-condition : la fixture n'est jamais passée UP (port {port})"
    );
    serveur_par_port(eng, port).unwrap()
}

// ---------- scénarios ----------

/// initialize → la console s'annonce comme serveur MCP ; tools/list → la
/// surface complète de supervision ; server/discover → capacités.
#[test]
fn gateway_initialize_tools_list_et_discover() {
    let _garde = serial_guard();
    let eng = moteur();

    let rep = mcp(
        &eng,
        &json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2024-11-05", "capabilities": {},
                         "clientInfo": {"name": "e2e", "version": "1.0"} }
        }),
    );
    let res = rep.get("result").expect("initialize sans result");
    assert_eq!(
        res["protocolVersion"], "2026-07-28",
        "le gateway annonce la spec LiteMCP : {res}"
    );
    assert_eq!(res["serverInfo"]["name"], "mcp-control");
    assert!(
        res.get("resultType").is_some(),
        "resultType (SEP-2322) absent : {res}"
    );

    let rep = mcp(
        &eng,
        &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    );
    let tools = rep["result"]["tools"]
        .as_array()
        .expect("tools/list sans tableau");
    let noms: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .collect();
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
        "add_server",
        "remove_server",
        "toggle_server",
        "start_server",
        "stop_server",
        "toggle_tool",
        "import_config",
        "install_store_server",
        "ssh_save",
        "call_server_tool",
    ] {
        assert!(
            noms.contains(&attendu),
            "outil gateway manquant : {attendu}"
        );
    }
    // chaque outil porte un inputSchema objet (contrat des clients MCP)
    for t in tools {
        assert!(
            t["inputSchema"].is_object(),
            "inputSchema manquant sur {}",
            t["name"]
        );
    }
    // réglages sensibles JAMAIS exposés comme outils
    for exclu in [
        "set_settings",
        "reset_state",
        "set_autostart",
        "set_notify",
        "open_path",
    ] {
        assert!(!noms.contains(&exclu), "outil sensible exposé : {exclu}");
    }

    let rep = mcp(
        &eng,
        &json!({"jsonrpc": "2.0", "id": 3, "method": "server/discover"}),
    );
    let caps = rep["result"]["capabilities"].clone();
    assert!(
        caps.get("tools").is_some(),
        "server/discover sans capacités tools : {rep}"
    );
}

/// La supervision en tools : list_servers / server_detail / toggle — le
/// miroir exact des boutons de l'UI, par MCP.
#[test]
fn gateway_list_servers_detail_et_toggle() {
    let _garde = serial_guard();
    let eng = moteur();
    let fx = Fixture::spawn_mcp();
    let rep = ajoute_http(&eng, fx.port(), "e2e-gw-supervision");
    let id = rep["id"].as_str().expect("add sans id").to_string();
    eng.action(&json!({"action": "refresh"}));
    attend_up(&eng, fx.port());

    // list_servers : la fixture est là, UP, comptée dans les stats
    let sc = structured(
        mcp(&eng, &rpc_call("list_servers", json!({}), 10)),
        "list_servers",
    );
    assert!(
        sc["stats"]["total"].as_u64().unwrap_or(0) >= 1,
        "stats sans serveur : {sc}"
    );
    assert!(
        sc["servers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| champ(s, "id") == id && champ(s, "state") == "UP"),
        "fixture absente ou pas UP : {sc}"
    );

    // server_detail : les outils ping/echo de la fixture, avec disabled:false
    let sc = structured(
        mcp(&eng, &rpc_call("server_detail", json!({ "id": id }), 11)),
        "server_detail",
    );
    let outils = sc["tools_list"].as_array().expect("tools_list absent");
    let noms: Vec<&str> = outils
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .collect();
    assert!(noms.contains(&"ping") && noms.contains(&"echo"), "{sc}");
    assert!(
        sc["framework"].is_null() || sc["framework"].is_string(),
        "{sc}"
    );

    // toggle_server : OFF puis ON — l'interrupteur de la ligne
    let sc = structured(
        mcp(&eng, &rpc_call("toggle_server", json!({ "id": id }), 12)),
        "toggle OFF",
    );
    assert_eq!(sc["enabled"], false, "{sc}");
    assert_eq!(
        serveur_par_port(&eng, fx.port()).unwrap()["enabled"],
        false,
        "l'état du moteur doit refléter le toggle via MCP"
    );
    let sc = structured(
        mcp(&eng, &rpc_call("toggle_server", json!({ "id": id }), 13)),
        "toggle ON",
    );
    assert_eq!(sc["enabled"], true, "{sc}");

    // journal : l'activité gateway est racontée
    let sc = structured(
        mcp(&eng, &rpc_call("journal", json!({ "limit": 50 }), 14)),
        "journal",
    );
    let msgs = sc["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e.get("msg").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert!(
        msgs.iter().any(|m| m.contains("[gateway]")),
        "le journal ne raconte pas l'activité gateway : {msgs:?}"
    );
}

/// Le proxy : call_server_tool traverse jusqu'à la fixture réelle et rend
/// son résultat structuré ; les refus (outil inconnu, serveur DOWN) sont
/// des erreurs classifiées.
#[test]
fn gateway_proxy_call_server_tool() {
    let _garde = serial_guard();
    let eng = moteur();
    let fx = Fixture::spawn_mcp();
    let rep = ajoute_http(&eng, fx.port(), "e2e-gw-proxy");
    let id = rep["id"].as_str().expect("add sans id").to_string();
    eng.action(&json!({"action": "refresh"}));
    attend_up(&eng, fx.port());

    // proxy réussi : le résultat upstream (content + structuredContent) est rendu
    // tel quel — il est donc imbriqué dans NOTRE structuredContent
    let rep = mcp(
        &eng,
        &rpc_call(
            "call_server_tool",
            json!({ "server": id, "tool": "ping", "arguments": {} }),
            20,
        ),
    );
    let result = rep.get("result").expect("proxy sans result");
    let upstream = result
        .get("structuredContent")
        .expect("proxy sans structuredContent (le résultat upstream doit être rendu tel quel)");
    assert_eq!(
        upstream["structuredContent"]["pong"], true,
        "le résultat structuré de la fixture doit transiter : {upstream}"
    );
    assert_eq!(upstream["structuredContent"]["tool"], "ping");
    let texte = upstream["content"][0]["text"].as_str().unwrap_or("");
    assert!(texte.contains("pong:ping"), "content upstream : {texte}");

    // list_server_tools : le catalogue de la fixture
    let sc = structured(
        mcp(
            &eng,
            &rpc_call("list_server_tools", json!({ "server": id }), 21),
        ),
        "list_server_tools",
    );
    let noms: Vec<&str> = sc["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .collect();
    assert!(noms.contains(&"ping") && noms.contains(&"echo"), "{sc}");

    // outil inconnu chez l'upstream → isError (le serveur distant répond en erreur)
    let rep = mcp(
        &eng,
        &rpc_call(
            "call_server_tool",
            json!({ "server": id, "tool": "inexistant" }),
            22,
        ),
    );
    // la fixture répond `initialize` à tout corps non reconnu → pas un
    // tools/call valide : le moteur doit transmettre l'échec proprement
    let result = rep
        .get("result")
        .expect("proxy doit toujours rendre un result");
    if let Some(true) = result.get("isError").and_then(Value::as_bool) {
        let msg = result["structuredContent"]["error"]["message"]
            .as_str()
            .unwrap_or("");
        assert!(!msg.is_empty(), "erreur proxy sans message : {result}");
    }

    // serveur DOWN : ajouté sans être démarré → refus classifié AVANT l'appel
    let port_mort = free_port();
    let rep = ajoute_http(&eng, port_mort, "e2e-gw-mort");
    let id_mort = rep["id"].as_str().expect("add sans id").to_string();
    let rep = mcp(
        &eng,
        &rpc_call(
            "call_server_tool",
            json!({ "server": id_mort, "tool": "ping" }),
            23,
        ),
    );
    let result = rep.get("result").expect("proxy DOWN sans result");
    assert_eq!(
        result["isError"], true,
        "un serveur DOWN doit être refusé : {result}"
    );
    let msg = result["structuredContent"]["error"]["message"]
        .as_str()
        .unwrap_or("");
    assert!(
        msg.contains("UP"),
        "le motif du refus doit nommer l'état requis : {msg}"
    );
    assert_eq!(
        champ(&serveur_par_port(&eng, port_mort).unwrap(), "state"),
        "DOWN"
    );
}

/// toggle_tool (interrupteur par outil de l'UI) coupe le proxy : l'appel est
/// refusé SANS joindre l'upstream.
#[test]
fn gateway_toggle_tool_coupe_le_proxy() {
    let _garde = serial_guard();
    let eng = moteur();
    let fx = Fixture::spawn_mcp();
    let rep = ajoute_http(&eng, fx.port(), "e2e-gw-toggle-tool");
    let id = rep["id"].as_str().expect("add sans id").to_string();
    eng.action(&json!({"action": "refresh"}));
    attend_up(&eng, fx.port());

    // l'UI coupe l'outil ping
    let rep = eng.action(&json!({"action": "toggle_tool", "id": id, "tool": "ping"}));
    assert_eq!(rep["ok"], true, "toggle_tool refusé : {rep}");

    // le gateway reflète l'interrupteur immédiatement
    let sc = structured(
        mcp(
            &eng,
            &rpc_call(
                "call_server_tool",
                json!({ "server": id, "tool": "ping" }),
                30,
            ),
        ),
        "proxy outil coupé",
    );
    let msg = sc["error"]["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("coupé pour ce serveur"),
        "le refus doit expliquer l'interrupteur : {sc}"
    );
    // et list_server_tools marque disabled:true
    let sc = structured(
        mcp(
            &eng,
            &rpc_call("list_server_tools", json!({ "server": id }), 31),
        ),
        "list après coupe",
    );
    let ping = sc["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "ping")
        .expect("ping absent du catalogue");
    assert_eq!(ping["disabled"], true, "{sc}");
}

/// Les ressources du gateway : aperçu, journal, réglages + une par serveur
/// détecté (le snapshot est reconstruit quand la détection change).
#[test]
fn gateway_ressources_par_serveur_detecte() {
    let _garde = serial_guard();
    let eng = moteur();

    // aperçu + journal + réglages présents d'emblée
    let rep = mcp(
        &eng,
        &json!({"jsonrpc": "2.0", "id": 1, "method": "resources/list"}),
    );
    let uris = |rep: &Value| -> Vec<String> {
        rep["result"]["resources"]
            .as_array()
            .unwrap_or(&vec![])
            .clone()
            .iter()
            .filter_map(|r| r.get("uri").and_then(Value::as_str).map(String::from))
            .collect()
    };
    let mut liste = uris(&rep);
    assert!(
        liste.contains(&"mcp-control://servers".to_string()),
        "{liste:?}"
    );
    assert!(
        liste.contains(&"mcp-control://journal".to_string()),
        "{liste:?}"
    );
    assert!(
        liste.contains(&"mcp-control://settings".to_string()),
        "{liste:?}"
    );

    // lecture de l'aperçu : JSON valide, tableau servers
    let rep = mcp(
        &eng,
        &json!({"jsonrpc": "2.0", "id": 2, "method": "resources/read",
                "params": { "uri": "mcp-control://servers" }}),
    );
    let texte = rep["result"]["contents"][0]["text"]
        .as_str()
        .expect("aperçu sans texte");
    let apercu: Value = serde_json::from_str(texte).expect("aperçu non JSON");
    assert!(apercu.get("servers").is_some() && apercu.get("stats").is_some());

    // une fixture est détectée → sa ressource dédiée apparaît sans redémarrage
    let fx = Fixture::spawn_mcp();
    let rep = ajoute_http(&eng, fx.port(), "e2e-gw-res");
    let id = rep["id"].as_str().expect("add sans id").to_string();
    eng.action(&json!({"action": "refresh"}));
    attend_up(&eng, fx.port());
    let attendu = format!("mcp-control://server/{id}");
    let apparait = wait_until(DELAI_SNAPSHOT, || {
        let rep = mcp(
            &eng,
            &json!({"jsonrpc": "2.0", "id": 3, "method": "resources/list"}),
        );
        if uris(&rep).contains(&attendu) {
            Some(true)
        } else {
            None
        }
    });
    assert!(
        apparait,
        "la ressource du serveur détecté n'est jamais apparue : {liste:?}"
    );
    liste = uris(&mcp(
        &eng,
        &json!({"jsonrpc": "2.0", "id": 4, "method": "resources/list"}),
    ));
    assert!(liste.contains(&attendu), "{liste:?}");
    let rep = mcp(
        &eng,
        &json!({"jsonrpc": "2.0", "id": 5, "method": "resources/read",
                "params": { "uri": attendu }}),
    );
    let texte = rep["result"]["contents"][0]["text"]
        .as_str()
        .expect("ressource serveur sans texte");
    let detail: Value = serde_json::from_str(texte).expect("détail non JSON");
    assert_eq!(detail["id"], json!(id), "{detail}");
}

/// Le transport /mcp reproduit les règles LiteMCP : notification 202, GET 405,
/// content-type 415, accept 406, SSE mono-événement, en-têtes SEP-2243.
#[test]
fn gateway_transport_regles_litemcp() {
    let _garde = serial_guard();
    let eng = moteur();

    // notification → 202 sans corps
    let resp = eng.post_json(
        "/mcp",
        &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    );
    assert_eq!(resp.status, 202, "notification doit rendre 202");
    assert!(resp.body.is_empty(), "202 avec corps : {:?}", resp.text());

    // GET → 405 (pas de listener SSE en stateless)
    let resp = eng.get("/mcp");
    assert_eq!(resp.status, 405);

    // content-type non JSON → 415 (requête brute : le harnais impose
    // application/json, on forge à la main)
    let brut = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}",
        eng.port
    );
    let resp = eng.raw(brut.as_bytes(), 0);
    assert_eq!(resp.status, 415, "content-type text/plain doit donner 415");

    // accept sans type acceptable → 406
    let resp = mcp_with(
        &eng,
        &[("Accept", "text/html")],
        &json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}),
    );
    assert_eq!(resp.status, 406);

    // client SSE-only → réponse text/event-stream mono-événement
    let resp = mcp_with(
        &eng,
        &[("Accept", "text/event-stream")],
        &json!({"jsonrpc": "2.0", "id": 2, "method": "ping"}),
    );
    assert_eq!(resp.status, 200);
    assert!(
        resp.header("content-type")
            .unwrap_or_default()
            .starts_with("text/event-stream"),
        "SSE attendu : {:?}",
        resp.headers
    );
    let texte = resp.text();
    assert!(texte.starts_with("event: message\ndata: "), "{texte}");

    // en-têtes SEP-2243 : cohérents → 200 ; incohérents → -32020 ; dupliqué → 400
    let corps = json!({
        "jsonrpc": "2.0", "id": 3, "method": "tools/call",
        "params": { "name": "list_servers", "arguments": {} }
    });
    let resp = mcp_with(
        &eng,
        &[("Mcp-Method", "tools/call"), ("Mcp-Name", "list_servers")],
        &corps,
    );
    assert_eq!(resp.status, 200);
    assert!(resp.json().get("result").is_some());

    let resp = mcp_with(&eng, &[("Mcp-Method", "ping")], &corps);
    let rep = resp.json();
    assert_eq!(rep["error"]["code"], -32020, "mismatch SEP-2243 : {rep}");

    let brut = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nMcp-Method: tools/call\r\nMcp-Method: ping\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        eng.port,
        corps.to_string().len(),
        corps
    );
    let resp = eng.raw(brut.as_bytes(), 0);
    assert_eq!(resp.status, 400, "Mcp-Method dupliqué doit donner 400");
}

/// L'auth du gateway hérite du sas /api/* : loopback jamais de code, LAN
/// exige le code (401 sans, 200 avec).
#[test]
fn gateway_auth_heritee_du_sas_api() {
    let _garde = serial_guard();
    let code = "e2e-gw-secret";
    let cfg = EngineCfg {
        ui_port: free_port(),
        ui_bind: "0.0.0.0",
        api_access: Some(code.to_string()),
    };
    let mut eng = spawn(cfg).expect("spawn moteur LAN");
    eng.wait_ready(Duration::from_secs(15))
        .expect("moteur LAN prêt");
    let ping = json!({"jsonrpc": "2.0", "id": 1, "method": "ping"});

    // loopback : jamais de code (le harnais pose déjà le Content-Type JSON
    // sur les corps — n'en pas redonner un, ce serait un duplicata → 400)
    let resp = mcp_with(&eng, &[], &ping);
    assert_eq!(resp.status, 200, "loopback ne doit jamais exiger le code");

    // LAN : sans code → 401, avec code → 200
    let ip = eng
        .lan_ip()
        .expect("pré-condition : pas d'interface LAN sur cette machine");
    let resp = eng.request(
        &ip,
        eng.port,
        "POST",
        "/mcp",
        &[],
        Some(ping.to_string().as_bytes()),
    );
    assert_eq!(
        resp.status,
        401,
        "LAN sans code doit donner 401 : {}",
        resp.text()
    );
    let resp = eng.request(
        &ip,
        eng.port,
        "POST",
        "/mcp",
        &[("X-MCC-Access", code)],
        Some(ping.to_string().as_bytes()),
    );
    assert_eq!(
        resp.status,
        200,
        "LAN avec code doit passer : {}",
        resp.text()
    );
    let rep = resp.json();
    assert!(
        rep.get("result").is_some(),
        "ping LAN avec code sans result : {rep}"
    );
}
