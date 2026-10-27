//! Scénarios e2e « démarrage » : démarrage réel du binaire mcp-control, hello
//! et nonce, page UI (CSP, absence du code d'accès), matrice d'authentification
//! loopback/LAN, contrôles anti Host/Origin forgés, preuve de pair
//! (X-MCC-Nonce / X-MCC-Proof) et endpoints lecture-seule.
//!
//! Chaque scénario crée SON moteur (config isolée, port libre) — cf. harness.rs.
//! À exécuter en single-thread : `cargo test --test e2e scenarios_boot -- --test-threads=1`.

use crate::harness::{
    free_port, spawn_lan_with_code, spawn_loopback, spawn_loopback_with_code, wait_until, EngineCfg,
};
use mcp_control::crypto::hmac_sha256_hex;
use serde_json::json;
use std::time::Duration;

/// Code d'accès réaliste mais propre aux tests.
const CODE: &str = "e2e-code-42cafe";

/// Preuve de pair attendue par verify_proof : HMAC-SHA256(code, "mcc-proof-v1:{nonce}")
/// en hex — format vérifié dans src/lib.rs (verify_proof) au 2026-10-26.
fn proof_for(key: &str, nonce: &str) -> String {
    hmac_sha256_hex(key.as_bytes(), format!("mcc-proof-v1:{}", nonce).as_bytes())
}

/// 1. Au démarrage : /api/hello délivre un nonce 64-hex sans jamais fuiter le
///    code d'accès ; la page / est du HTML avec loginDlg, un CSP, X-Content-Type-
///    Options, et NE contient PAS le code (régression audit 2026-10-24).
#[test]
fn boot_hello_and_root_page() {
    let _garde = crate::harness::serial_guard();
    let e = spawn_loopback_with_code(CODE);
    let hello = e.get("/api/hello");
    assert_eq!(hello.status, 200, "GET /api/hello doit répondre 200");
    let v = hello.json();
    let nonce = v["nonce"].as_str().expect("nonce présent dans hello");
    assert_eq!(nonce.len(), 64, "nonce = 64 caractères hex");
    assert!(
        nonce.chars().all(|c| c.is_ascii_hexdigit()),
        "nonce hexadécimal, obtenu : {}",
        nonce
    );
    assert!(
        !hello.text().contains(CODE),
        "hello ne doit contenir NI le code d'accès ni aucun secret"
    );
    assert_eq!(v["app"], json!("mcp-control"), "identité hello");

    let root = e.get("/");
    assert_eq!(root.status, 200, "GET / doit répondre 200");
    assert!(
        root.header("content-type")
            .unwrap_or_default()
            .starts_with("text/html"),
        "la page / doit être du HTML, obtenu : {:?}",
        root.header("content-type")
    );
    let html = root.text();
    assert!(
        html.contains("loginDlg"),
        "la page doit proposer le déverrouillage (loginDlg)"
    );
    assert!(
        !html.contains(CODE),
        "RÉGRESSION P0 : la page / ne doit JAMAIS contenir le code d'accès"
    );
    assert!(
        root.header("content-security-policy").is_some(),
        "CSP attendu sur la page UI"
    );
    assert_eq!(
        root.header("x-content-type-options").as_deref(),
        Some("nosniff"),
        "anti-sniff attendu sur toutes les réponses"
    );
}

/// 2. Matrice d'auth côté LOOPBACK : un client local n'a JAMAIS à présenter le
///    code — ni sans code, ni avec un mauvais code, ni avec le bon (le gating
///    n'est pas appliqué du tout aux connexions locales, cf.
///    effective_require_access : !bind_loopback && !peer_loopback).
#[test]
fn auth_matrix_loopback() {
    let _garde = crate::harness::serial_guard();
    let e = spawn_loopback_with_code(CODE);
    let no_code = e.get("/api/state");
    assert_eq!(no_code.status, 200, "loopback sans code : 200 attendu");
    let bad = e.get_with("/api/state", &[("X-MCC-Access", "mauvais-code")]);
    assert_eq!(
        bad.status, 200,
        "loopback avec MAUVAIS code : toujours 200 (jamais de gating local)"
    );
    let good = e.get_with("/api/state", &[("X-MCC-Access", CODE)]);
    assert_eq!(good.status, 200, "loopback avec bon code : 200");
    // le JSON d'état est bien un état (clé servers présente)
    assert!(
        good.json().get("servers").is_some(),
        "/api/state doit exposer la liste des serveurs"
    );
}

/// 3. Matrice d'auth côté LAN (bind 0.0.0.0, client via l'IP LAN) : le code est
///    EXIGÉ — absent → 401, mauvais (header ou ?access=) → 401, bon → 200 ;
///    et le MÊME moteur reste sans code pour un client loopback.
///    Skip propre si la machine n'a pas d'IP LAN joignable.
#[test]
fn auth_matrix_lan() {
    let _garde = crate::harness::serial_guard();
    let e = spawn_lan_with_code(CODE);
    let Some(ip) = e.lan_ip() else {
        eprintln!("SKIP auth_matrix_lan : aucune IP LAN non-loopback détectée");
        return;
    };
    // sanity : /api/hello est pré-authentifié ET l'Host <ip> est dans la liste
    // autorisée (local_ips inclut la même IP que l'astuce UDP) — si ce 200
    // manque, l'environnement est atypique et le reste du test serait du bruit.
    eprintln!("diagnostic : IP LAN retenue = {}", ip);
    let hello = e.request(&ip, e.port, "GET", "/api/hello", &[], None);
    assert_eq!(
        hello.status,
        200,
        "hello via LAN doit passer (pré-auth + Host autorisé) (détail : {})",
        hello.text().chars().take(300).collect::<String>()
    );

    let no_code = e.request(&ip, e.port, "GET", "/api/state", &[], None);
    assert_eq!(
        no_code.status,
        401,
        "LAN sans code → 401 (détail : {})",
        no_code.text().chars().take(300).collect::<String>()
    );

    let bad_hdr = e.request(
        &ip,
        e.port,
        "GET",
        "/api/state",
        &[("X-MCC-Access", "mauvais-code")],
        None,
    );
    assert_eq!(bad_hdr.status, 401, "LAN avec mauvais code (header) → 401");

    let bad_q = e.request(
        &ip,
        e.port,
        "GET",
        "/api/state?access=mauvais-code",
        &[],
        None,
    );
    assert_eq!(bad_q.status, 401, "LAN avec mauvais code (?access=) → 401");

    let good_hdr = e.request(
        &ip,
        e.port,
        "GET",
        "/api/state",
        &[("X-MCC-Access", CODE)],
        None,
    );
    assert_eq!(good_hdr.status, 200, "LAN avec bon code (header) → 200");

    let good_q = e.request(
        &ip,
        e.port,
        "GET",
        &format!("/api/state?access={}", CODE),
        &[],
        None,
    );
    assert_eq!(good_q.status, 200, "LAN avec bon code (?access=) → 200");

    // client LOCAL du même moteur (fenêtre desktop, navigateur de la machine) :
    // jamais de code, même quand le bind est 0.0.0.0
    let local = e.get("/api/state");
    assert_eq!(
        local.status, 200,
        "le même moteur bind 0.0.0.0 reste libre pour un client loopback"
    );
}

/// 4. Forgery Host / Origin / preflight : Host étranger → 403 (anti
///    DNS-rebinding), Origin cross-site sur POST /api/action → 403 (anti CSRF),
///    OPTIONS → 403 (aucun préflight CORS ne passe). Ces contrôles s'appliquent
///    À TOUTES les connexions, locales incluses (cf. check_access).
///    Moteur bind 0.0.0.0 : la requête Host forgée part via l'IP LAN quand elle
///    existe (cas réel du DNS-rebinding), sinon via loopback (contrôle
///    identique côté moteur) ; sans IP LAN, tout reste testable en local.
#[test]
fn origin_host_forgery() {
    let _garde = crate::harness::serial_guard();
    let e = spawn_lan_with_code(CODE);
    let host = e.lan_ip().unwrap_or_else(|| "127.0.0.1".to_string());
    // Host forgé : le header Host remplace celui dérivé de la cible
    let forged = e.request(
        &host,
        e.port,
        "GET",
        "/api/state",
        &[("Host", "evil.example.com")],
        None,
    );
    assert_eq!(
        forged.status,
        403,
        "Host étranger → 403 (anti DNS-rebinding) (détail : {})",
        forged.text().chars().take(300).collect::<String>()
    );

    // Origin forgée sur POST /api/action (corps JSON valide) → 403 — le contrôle
    // Origin s'applique aussi aux connexions LOCALES (anti CSRF).
    let body = json!({"action": "refresh", "id": "seed-overmind"});
    let csrf = e.request(
        "127.0.0.1",
        e.port,
        "POST",
        "/api/action",
        &[("Origin", "https://malveille.example")],
        Some(body.to_string().as_bytes()),
    );
    assert_eq!(csrf.status, 403, "Origin cross-site → 403 (anti CSRF)");

    // préflight OPTIONS → 403, quelle que soit la cible
    let pre = e.request("127.0.0.1", e.port, "OPTIONS", "/api/state", &[], None);
    assert_eq!(pre.status, 403, "OPTIONS → 403 (préflight refusé)");
    let pre2 = e.request("127.0.0.1", e.port, "OPTIONS", "/api/action", &[], None);
    assert_eq!(pre2.status, 403, "OPTIONS /api/action → 403");
}

/// 5. RCE sans authentification interdit : un POST /api/action « add stdio »
///    (qui lancerait un process node éternel) envoyé via le LAN SANS code doit
///    répondre 401 et NE JAMAIS créer d'entrée manual-stdio — même après deux
///    cycles complets de rafraîchissement (refresh_secs = 5 dans la config e2e).
#[test]
fn unauth_rce_never_spawns() {
    let _garde = crate::harness::serial_guard();
    let e = spawn_lan_with_code(CODE);
    let Some(ip) = e.lan_ip() else {
        eprintln!("SKIP unauth_rce_never_spawns : aucune IP LAN non-loopback détectée");
        return;
    };
    let payload = json!({
        "action": "add",
        "transport": "stdio",
        "program": "node",
        "args": ["-e", "setInterval(()=>{},1e9)"]
    });
    let r = e.request(
        &ip,
        e.port,
        "POST",
        "/api/action",
        &[],
        Some(payload.to_string().as_bytes()),
    );
    assert_eq!(
        r.status,
        401,
        "add stdio sans code via LAN → 401 obligatoire (détail : {})",
        r.text().chars().take(300).collect::<String>()
    );
    assert!(
        !r.text().contains("\"ok\":true"),
        "aucun ok:true ne doit fuiter dans un 401"
    );

    // deux cycles de refresh (2 × 5 s) : on SURVEILLE l'apparition et on
    // échoue immédiatement si l'entrée venait à exister.
    let leaked = wait_until(Duration::from_secs(13), || {
        let text = e.get("/api/state").text();
        if text.contains("manual-stdio") {
            Some(true)
        } else {
            None
        }
    });
    assert!(
        !leaked,
        "RCE : un serveur stdio a été ajouté sans authentification !"
    );
    let final_state = e.get("/api/state").text();
    assert!(
        !final_state.contains("manual-stdio"),
        "état final : aucune entrée manual-stdio attendue"
    );
}

/// 6. Preuve de pair (X-MCC-Nonce / X-MCC-Proof) : nonce émis par /api/hello,
///    signé HMAC-SHA256(code, "mcc-proof-v1:{nonce}") → 200 ; REJEU du même
///    nonce → 401 (usage unique) ; preuve signée avec le mauvais code → 401 ;
///    nonce inventé (64-hex jamais émis) → 401.
#[test]
fn peer_proof_valid_replay_fake() {
    let _garde = crate::harness::serial_guard();
    let e = spawn_lan_with_code(CODE);
    let Some(ip) = e.lan_ip() else {
        eprintln!("SKIP peer_proof_valid_replay_fake : aucune IP LAN non-loopback détectée");
        return;
    };

    // 1. récupérer un nonce réel via /api/hello (pré-authentifié)
    let hello = e.request(&ip, e.port, "GET", "/api/hello", &[], None);
    assert_eq!(hello.status, 200, "hello via LAN doit passer");
    let nonce = hello.json()["nonce"]
        .as_str()
        .expect("nonce dans hello")
        .to_string();
    assert_eq!(nonce.len(), 64, "nonce 64-hex");

    // 2. preuve valide → /api/state 200 SANS jamais envoyer le code en clair
    let proof = proof_for(CODE, &nonce);
    let ok = e.request(
        &ip,
        e.port,
        "GET",
        "/api/state",
        &[("X-MCC-Nonce", &nonce), ("X-MCC-Proof", &proof)],
        None,
    );
    assert_eq!(ok.status, 200, "preuve de pair valide → 200");
    assert!(
        ok.json().get("servers").is_some(),
        "l'état complet doit être servi derrière la preuve"
    );

    // 3. REJEU du même couple nonce/preuve → 401 (nonce consommé)
    let replay = e.request(
        &ip,
        e.port,
        "GET",
        "/api/state",
        &[("X-MCC-Nonce", &nonce), ("X-MCC-Proof", &proof)],
        None,
    );
    assert_eq!(replay.status, 401, "nonce à usage unique : rejeu → 401");

    // 4. nouveau nonce signé avec le MAUVAIS code → 401
    let hello2 = e.request(&ip, e.port, "GET", "/api/hello", &[], None);
    let nonce2 = hello2.json()["nonce"].as_str().expect("nonce2").to_string();
    let bad_proof = proof_for("mauvais-code-partage", &nonce2);
    let wrong_key = e.request(
        &ip,
        e.port,
        "GET",
        "/api/state",
        &[("X-MCC-Nonce", &nonce2), ("X-MCC-Proof", &bad_proof)],
        None,
    );
    assert_eq!(
        wrong_key.status, 401,
        "preuve signée avec un autre code → 401"
    );

    // 5. nonce INVENTÉ (64-hex, jamais émis par ce moteur) → 401
    let fake_nonce = "a".repeat(64);
    let fake_proof = proof_for(CODE, &fake_nonce);
    let invented = e.request(
        &ip,
        e.port,
        "GET",
        "/api/state",
        &[("X-MCC-Nonce", &fake_nonce), ("X-MCC-Proof", &fake_proof)],
        None,
    );
    assert_eq!(invented.status, 401, "nonce jamais émis → 401");
}

/// 7. Endpoints lecture-seule : /api/store, /api/store-logos et /api/frameworks
///    répondent 200 avec du JSON parseable (catalogues embarqués, hors ligne).
#[test]
fn store_frameworks_endpoints() {
    let _garde = crate::harness::serial_guard();
    let e = spawn_loopback();
    for path in ["/api/store", "/api/store-logos", "/api/frameworks"] {
        let r = e.get(path);
        assert_eq!(r.status, 200, "{} doit répondre 200", path);
        let v = r.json();
        assert!(
            !v.is_null(),
            "{} doit répondre du JSON valide, obtenu : {}",
            path,
            r.text().chars().take(120).collect::<String>()
        );
    }
    // contenu attendu : la table des frameworks expose une liste non vide
    let fw = e.get("/api/frameworks").json();
    assert!(
        fw["frameworks"].as_array().is_some_and(|a| !a.is_empty()),
        "la table des frameworks ne doit pas être vide"
    );
}

/// Ancrage de surface du harnais (EngineCfg::default, free_port) : garantit
/// que l'API partagée avec les modules de scénarios compile et reste saine.
#[test]
fn harness_surface_compile() {
    let _garde = crate::harness::serial_guard();
    let cfg = EngineCfg::default();
    assert_eq!(cfg.ui_bind, "127.0.0.1", "défaut = loopback");
    assert_ne!(free_port(), 0, "un port libre est trouvé");
}
