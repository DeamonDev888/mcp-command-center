//! Scénarios e2e « sécurité » : fuzz HTTP brut sur le VRAI moteur (sockets TCP
//! brutes via Engine::raw), et isolation d'un label hostile ajouté via l'API.
//!
//! Contrats vérifiés contre src/lib.rs au 2026-10-26 :
//! - MAX_BODY = 2 MiB → 413 ; MAX_CONNS = 64 → refus à sec (aucune réponse) ;
//! - Transfer-Encoding présent → 400 ; Content-Length dupliqué → 400 ;
//! - OPTIONS → 403 ; Host/Origin étrangers → 403 ; /api hors loopback → 401 ;
//! - JSON malformé / action inconnue → 200 avec {"ok":false,...} ;
//! - bloc d'en-têtes > 1 MiB → connexion fermée sans réponse (le moteur survit).
//!
//! À exécuter SEUL (jamais en parallèle des autres scénarios) :
//! `cargo test --test e2e scenarios_security -- --test-threads=1`

use crate::harness::{spawn_loopback, wait_until, Fixture};
use serde_json::json;
use std::net::TcpStream;
use std::time::Duration;

/// Taille max d'un corps acceptée par le moteur (src/lib.rs MAX_BODY).
const MAX_BODY: usize = 2 * 1024 * 1024;

/// Fuzz léger mais réel : JSON malformé, action inconnue, request smuggling,
/// corps surdimensionné, Host absent, bloc d'en-têtes géant, tempête de
/// connexions. Chaque étape vérifie que le moteur répond CE qui est attendu
/// aujourd'hui (et surtout qu'il SURVIT).
#[test]
fn fuzz_lite() {
    let _garde = crate::harness::serial_guard();
    let e = spawn_loopback();

    // ---- 1. corps JSON malformé → 200 + {"ok":false,"error":"body invalide: …"}
    let bad_json = e.request(
        "127.0.0.1",
        e.port,
        "POST",
        "/api/action",
        &[],
        Some(b"{not json"),
    );
    assert_eq!(
        bad_json.status, 200,
        "un JSON malformé ne doit pas casser la réponse HTTP"
    );
    let v = bad_json.json();
    assert_eq!(v["ok"], json!(false), "ok:false attendu");
    assert!(
        v["error"]
            .as_str()
            .unwrap_or_default()
            .starts_with("body invalide"),
        "erreur « body invalide » attendue, obtenu : {}",
        v["error"]
    );

    // ---- 2. action inconnue → 200 + {"ok":false,"error":"action inconnue: …"}
    let unknown = e.post_json("/api/action", &json!({"action": "danse-des-canards"}));
    assert_eq!(
        unknown.status, 200,
        "action inconnue : HTTP 200 (erreur métier)"
    );
    let v = unknown.json();
    assert_eq!(v["ok"], json!(false), "ok:false attendu");
    assert!(
        v["error"]
            .as_str()
            .unwrap_or_default()
            .contains("action inconnue"),
        "erreur « action inconnue » attendue, obtenu : {}",
        v["error"]
    );

    // ---- 3. Transfer-Encoding: chunked → 400 (anti request-smuggling)
    let te = e.raw(
        b"POST /api/action HTTP/1.1\r\nHost: 127.0.0.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
        0,
    );
    assert_eq!(te.status, 400, "Transfer-Encoding: chunked → 400");

    // ---- 4. Content-Length dupliqué → 400 (cadrage ambigu)
    let dup = e.raw(
        b"POST /api/action HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2\r\ncontent-length: 2\r\n\r\n{}",
        0,
    );
    assert_eq!(dup.status, 400, "Content-Length dupliqué → 400");

    // ---- 5. corps > MAX_BODY (2 MiB) → 413 Payload Too Large
    let big = vec![b'x'; MAX_BODY + 16];
    let mut flood = format!(
        "POST /api/action HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        big.len()
    )
    .into_bytes();
    flood.extend_from_slice(&big);
    let too_big = e.raw(&flood, 0);
    assert_eq!(too_big.status, 413, "corps > 2 MiB → 413");
    assert!(
        too_big.text().contains("body trop grand"),
        "message « body trop grand » attendu, obtenu : {}",
        too_big.text().chars().take(200).collect::<String>()
    );

    // ---- 6. requête SANS en-tête Host.
    // COMPORTEMENT ACTUEL (P3, audit 2026-10-26) : ACCEPTÉ — le contrôle anti
    // DNS-rebinding ne s'applique qu'à un Host PRÉSENT (headers.iter().find) ;
    // une requête sans Host contourne donc la liste blanche. Ce test fige le
    // comportement : le jour où le moteur rejette (400/403), il faudra le
    // mettre à jour — volontairement visible.
    let no_host = e.raw(b"GET /api/hello HTTP/1.1\r\n\r\n", 0);
    assert_eq!(
        no_host.status, 200,
        "sans Host : comportement actuel = accepté (P3 documenté, à durcir côté moteur)"
    );

    // ---- 7. bloc d'en-têtes surdimensionné (> cap 1 MiB de la lecture d'en-tête)
    // → le moteur ferme SANS répondre (head_end None → return). On accepte un
    // échec d'écriture côté client (RST pendant le flood) : l'essentiel est que
    // le moteur SURVIVE et reponde ensuite.
    let mut huge = String::from("GET /api/hello HTTP/1.1\r\nHost: 127.0.0.1\r\n");
    for i in 0..15_000u32 {
        huge.push_str(&format!("X-Flood-{}: {}\r\n", i, "A".repeat(64)));
    }
    huge.push_str("\r\n");
    let _ = e.raw(huge.as_bytes(), 0); // aucune réponse attendue (status 0)
    let alive = e.get("/api/hello");
    assert_eq!(
        alive.status, 200,
        "le moteur doit survivre à un bloc d'en-têtes surdimensionné"
    );

    // ---- 8. tempête de connexions : au-delà de MAX_CONNS (64), refus À SEC
    // (le handler retourne sans écrire). On tient 90 sockets ouvertes (lecture
    // bloquée côté moteur), on attend la saturation, puis on vérifie que le
    // moteur récupère une fois les sockets relâchées.
    let mut held: Vec<TcpStream> = Vec::new();
    for _ in 0..90 {
        if let Ok(s) = TcpStream::connect(("127.0.0.1", e.port)) {
            held.push(s);
        }
    }
    thread_sleep(Duration::from_millis(300));
    // adaptatif : tant qu'une sonde obtient une réponse, la saturation n'est
    // pas atteinte (des threads sont encore sous le plafond) → on continue.
    let probe = b"GET /api/hello HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
    let saturated = wait_until(Duration::from_secs(6), || {
        let r = e.raw(probe, 0);
        if r.status == 0 {
            Some(true) // aucune réponse = refus à sec (MAX_CONNS atteint)
        } else {
            None
        }
    });
    assert!(
        saturated,
        "saturation MAX_CONNS (64) non observée avec 90 connexions tenues"
    );
    drop(held);
    // ConnGuard relâchées → le moteur répond de nouveau (il a SURVÉCU)
    let recovered = wait_until(Duration::from_secs(6), || {
        if e.get("/api/hello").status == 200 {
            Some(true)
        } else {
            None
        }
    });
    assert!(recovered, "le moteur doit récupérer après la tempête");
    // et une action métier passe toujours après l'incident
    let after = e.post_json("/api/action", &json!({"action": "scan"}));
    assert_eq!(after.status, 200, "POST /api/action après récupération");
    assert_eq!(
        after.json()["ok"],
        json!(true),
        "action scan ok après l'incident"
    );
}

/// Petit sleep nommé pour la lisibilité (l'attente RÉELLE est wait_until).
fn thread_sleep(d: Duration) {
    std::thread::sleep(d);
}

/// Un label d'UI hostile (XSS) ajouté via l'API ne doit JAMAIS se retrouver
/// dans le HTML servi sur / : le label vit dans le JSON de /api/state (donnée
/// brute pour le front), la page reste le shell statique avec son CSP.
#[test]
fn hostile_label_never_in_html() {
    let _garde = crate::harness::serial_guard();
    // faux serveur MCP complet : initialize valide → le moteur doit le voir UP
    let fx = Fixture::spawn_mcp();
    let e = spawn_loopback();

    let label = "<img src=x onerror=alert(1)>";
    let add = e.post_json(
        "/api/action",
        &json!({
            "action": "add",
            "transport": "http",
            "port": fx.port(),
            "path": "/mcp",
            "label": label
        }),
    );
    let av = add.json();
    assert_eq!(
        av["ok"],
        json!(true),
        "l'ajout doit réussir, obtenu : {}",
        add.text().chars().take(200).collect::<String>()
    );
    let id = av["id"].as_str().expect("id du serveur ajouté").to_string();
    assert!(
        id.starts_with("manual-http-"),
        "id attendu manual-http-*, obtenu : {}",
        id
    );

    // le moteur sonde ::1 puis 127.0.0.1, chemin /mcp (puis / et /api/mcp) :
    // la fixture répond à TOUT → UP au prochain cycle (refresh_secs = 5).
    // Deadline large (45 s) : sous la suite complète, 45 scénarios lancent
    // leurs moteurs en parallèle et le premier cycle de sonde peut dépasser
    // largement refresh_secs — ce test a flotté à 20 s en exécution chargée.
    let up = wait_until(Duration::from_secs(45), || {
        let v = e.get("/api/state").json();
        let found = v["servers"]
            .as_array()?
            .iter()
            .find(|s| s["id"] == json!(id))?;
        if found["state"] == json!("UP") {
            Some(true)
        } else {
            None
        }
    });
    assert!(
        up,
        "le serveur fictif doit être UP (initialize + tools/list répondu)"
    );

    // /api/state : le label EST présent, en clair, comme DONNÉE JSON
    let state = e.get("/api/state");
    assert_eq!(state.status, 200);
    assert!(
        state.text().contains("onerror=alert(1)"),
        "le label hostile doit être conservé tel quel dans /api/state"
    );

    // la page HTML : NI le label, NI onerror, NI un quelconque échappement
    let root = e.get("/");
    assert_eq!(root.status, 200);
    assert!(
        root.header("content-type")
            .unwrap_or_default()
            .starts_with("text/html"),
        "la page / est bien du HTML"
    );
    let html = root.text();
    assert!(
        !html.contains("onerror"),
        "aucun onerror ne doit apparaître dans la page / (XSS neutralisée)"
    );
    assert!(
        !html.contains("<img src=x"),
        "le label hostile ne doit pas fuiter dans la page /"
    );
    assert!(
        root.header("content-security-policy").is_some(),
        "le CSP doit être présent sur la page /"
    );
}
