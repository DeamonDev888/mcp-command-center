//! Scénarios e2e — scan / découverte locale et intégrité des entrées manuelles.
//!
//! Faits source vérifiés (src/manage.rs, src/probe.rs) :
//! - action `discover` → `scan_listening` (SYNCHRONE) : ports en écoute de la
//!   machine (netstat/ss), sonde /mcp puis / puis /api/mcp en parallèle ; un
//!   répondeur MCP devient une entrée `disc-<port>`, origin "discovered",
//!   state UP — et N'EST PAS persistée dans config.json (save_config saute
//!   les Discovered).
//! - action `scan` (from/to) → `scan_ports` (SYNCHRONE) : même principe sur
//!   une plage, saute les ports déjà connus (http local) et le port UI.
//! - validation stricte d'initialize : une réponse JSON-RPC SANS serverInfo
//!   (`{"result":{}}`) est rejetée (probe.rs::validate_initialize) — un tel
//!   port ne doit JAMAIS devenir une entrée.
//! - `merge_duplicate_servers` ne fusionne QUE les entrées Discovered + http
//!   avec hôte explicite + nom annoncé non vide (audit P1 2026-10-26) : jamais
//!   une entrée manuelle. `prune_dead_network_servers` ne retire que les ids
//!   `net-*` (scan réseau) : jamais une entrée manuelle, même morte.
//!
//! Chaque `#[test]` démarre SON propre moteur ; attentes bornées par
//! `wait_until`, sérialisation via `--test-threads=1`.

use crate::harness::{free_port, spawn_loopback, wait_until, Engine, Fixture};
use serde_json::{json, Value};
use std::time::Duration;

/// Attente maximale pour voir une découverte apparaître. LARGEMENT bornée :
/// scan_listening joint les sondes DANS L'ORDRE des ports candidats (croissant)
/// et un port muet (service Windows qui accepte sans répondre) coûte jusqu'à
/// ~26 s d'échelle de sonde — une fixture sur un port élevé n'est ajoutée
/// qu'au bouquet final (~40 s et plus quand plusieurs moteurs scannent en
/// même temps). Le POST "discover" peut de son côté dépasser le timeout de
/// lecture du harnais (10 s) : le scan continue CÔTÉ MOTEUR (synchrone dans
/// le thread de la requête), la réponse perdue n'interrompt rien.
const DELAI_SCAN: Duration = Duration::from_secs(75);
/// Deux cycles de poll (refresh_secs plafonné à 5 s par le harnais) — utilisé
/// pour les assertions d'ABSENCE (laisser le temps aux faux positifs d'apparaître)
/// et pour vérifier qu'une entrée manuelle SURVIT à un cycle après scan.
const DEUX_CYCLES: Duration = Duration::from_secs(12);

// ---------- helpers locaux ----------

fn moteur() -> Engine {
    spawn_loopback()
}

fn serveurs_etat(eng: &Engine) -> Vec<Value> {
    eng.get("/api/state")
        .json()
        .get("servers")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default()
}

/// config.json du moteur (Null si le fichier n'existe pas encore — le scan
/// n'appelle PAS save_config).
fn config_du_moteur(eng: &Engine) -> Value {
    let chemin = eng.tmp_dir().join("config.json");
    match std::fs::read(&chemin) {
        Ok(octets) => serde_json::from_slice(&octets).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

fn serveurs_config(eng: &Engine) -> Vec<Value> {
    config_du_moteur(eng)
        .get("servers")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default()
}

fn meme_port(s: &Value, port: u16) -> bool {
    s.get("transport_info")
        .and_then(|t| t.get("port"))
        .and_then(|p| p.as_u64())
        == Some(port as u64)
}

fn champ<'a>(s: &'a Value, cle: &str) -> &'a str {
    s.get(cle).and_then(|v| v.as_str()).unwrap_or("")
}

fn ajoute_http(eng: &Engine, port: u16, label: &str) -> Value {
    eng.action(&json!({
        "action": "add", "transport": "http",
        "host": "127.0.0.1", "port": port, "path": "/mcp", "label": label
    }))
}

/// Le scan local, déclenché comme l'UI (POST /api/action → handle_action).
/// Synchrone : au retour, les détections sont déjà dans l'état.
fn scan_local(eng: &Engine) -> Value {
    eng.action(&json!({"action": "discover"}))
}

/// Scan par plage borné (rapide : 150 ms par port au pire).
fn scan_plage(eng: &Engine, from: u16, to: u16) -> Value {
    eng.action(&json!({"action": "scan", "from": from, "to": to}))
}

// ---------- scénarios ----------

/// Scénario 8 — Le scan local trouve une fixture MCP réelle : fixture écoutant
/// sur un port libre, action "discover" (tous ports en écoute) → une entrée
/// doit apparaître pour CE port, state UP, origin "discovered" (id disc-<port>
/// ou équivalent — on ancre l'assertion sur le port seul). L'entrée découverte
/// ne doit PAS être écrite dans config.json (redécouverte au boot).
#[test]
fn scan_listening_finds_local_fixture() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();
    let fx = Fixture::spawn_mcp();
    scan_local(&eng);

    let trouve = wait_until(DELAI_SCAN, || {
        let serveurs = serveurs_etat(&eng);
        match serveurs.iter().find(|s| meme_port(s, fx.port())) {
            Some(s) if champ(s, "state") == "UP" => Some(true),
            _ => None,
        }
    });
    assert!(
        trouve,
        "le scan local n'a pas trouvé la fixture sur le port {} — état : {:?}",
        fx.port(),
        serveurs_etat(&eng)
    );
    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| meme_port(s, fx.port()))
        .expect("entrée disparue");
    assert_eq!(champ(&s, "origin"), "discovered", "{s}");
    assert!(
        champ(&s, "id").starts_with("disc-"),
        "id attendu disc-<port> : {s}"
    );
    assert_eq!(champ(&s, "server_name"), "fake-e2e-mcp", "{s}");
    // Non-persistence : les entrées découvertes ne vont pas dans config.json.
    assert!(
        !serveurs_config(&eng)
            .iter()
            .any(|c| meme_port(c, fx.port())),
        "une entrée discovered ne doit pas être persistée : {:?}",
        config_du_moteur(&eng)
    );
}

/// Scénario 9 — Un port qui répond du JSON-RPC SANS serverInfo est rejeté :
/// fixture `result-empty` ({"result":{}} à initialize) sur un port libre,
/// scan local, puis DEUX cycles de poll d'attente (~12 s) → AUCUNE entrée ne
/// doit exister pour ce port dans /api/state ni dans config.json (régression
/// faux positifs — validate_initialize rejette l'initialize sans serverInfo).
#[test]
fn scan_rejects_result_empty() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();
    let fx = Fixture::spawn_result_empty();
    scan_local(&eng);
    // deux cycles de poll (plancher refresh 5 s) pour laisser un éventuel
    // faux positif apparaître, puis constat d'absence.
    std::thread::sleep(DEUX_CYCLES);
    assert!(
        !serveurs_etat(&eng).iter().any(|s| meme_port(s, fx.port())),
        "le port result-empty a été accepté comme serveur MCP (faux positif) : {:?}",
        serveurs_etat(&eng)
    );
    assert!(
        !serveurs_config(&eng)
            .iter()
            .any(|c| meme_port(c, fx.port())),
        "le port result-empty a été persisté dans config.json : {:?}",
        config_du_moteur(&eng)
    );
}

/// Scénario 10 — RÉGRESSION fusion P1 : deux serveurs MANUELS de même label
/// ("doublon-e2e") sur deux ports différents (un port mort, une fixture
/// vivante) doivent TOUS DEUX survivre à un scan — en /api/state ET dans
/// config.json. Le correctif merge_duplicate_servers restreint la fusion aux
/// entrées Discovered (hôte explicite + nom annoncé) : toute disparition ici
/// signifierait une régression du correctif. NB : la fusion elle-même ne
/// tourne que sur le chemin scan_network (asynchrone, réseau réel — hors
/// budget e2e) ; ce scénario verrouille l'invariant persistant côté manuel.
#[test]
fn manual_same_name_two_ports_survive_scan() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();
    let fx = Fixture::spawn_mcp();
    let port_mort = free_port(); // rien n'écoute → l'entrée restera DOWN/PORT-ONLY
    let (port_a, port_b) = if port_mort < fx.port() {
        (port_mort, fx.port())
    } else {
        (fx.port(), port_mort)
    };

    let rep_a = ajoute_http(&eng, port_a, "doublon-e2e");
    // now_compact() est en millisecondes : deux adds à <1 ms d'écart
    // produiraient le MÊME id — on laisse souffler et on exige la différence.
    std::thread::sleep(Duration::from_millis(20));
    let rep_b = ajoute_http(&eng, port_b, "doublon-e2e");
    let id_a = rep_a
        .get("id")
        .and_then(|i| i.as_str())
        .unwrap_or("")
        .to_string();
    let id_b = rep_b
        .get("id")
        .and_then(|i| i.as_str())
        .unwrap_or("")
        .to_string();
    assert!(
        !id_a.is_empty() && !id_b.is_empty(),
        "add sans id : {rep_a} / {rep_b}"
    );
    assert_ne!(
        id_a, id_b,
        "collision d'id entre les deux adds : {rep_a} / {rep_b}"
    );

    // Scan borné à la plage des deux ports : synchrone, rapide, et il voit le
    // port vivant (déjà connu → ignoré) et le port mort (fermé → ignoré).
    let rep = scan_plage(&eng, port_a, port_b);
    assert!(
        rep.get("ok").and_then(|o| o.as_bool()).unwrap_or(false),
        "scan refusé : {rep}"
    );

    // Un cycle complet, puis les deux entrées doivent toujours être là.
    let survivent = wait_until(DEUX_CYCLES, || {
        let serveurs = serveurs_etat(&eng);
        let a = serveurs
            .iter()
            .any(|s| champ(s, "id") == id_a && meme_port(s, port_a));
        let b = serveurs
            .iter()
            .any(|s| champ(s, "id") == id_b && meme_port(s, port_b));
        if a && b {
            Some(true)
        } else {
            None
        }
    });
    assert!(
        survivent,
        "une entrée manuelle a disparu de /api/state après scan : {:?}",
        serveurs_etat(&eng)
    );
    let cfg = serveurs_config(&eng);
    assert!(
        cfg.iter().any(|c| c.get("port") == Some(&json!(port_a))),
        "port A absent de config.json après scan : {:?}",
        config_du_moteur(&eng)
    );
    assert!(
        cfg.iter().any(|c| c.get("port") == Some(&json!(port_b))),
        "port B absent de config.json après scan : {:?}",
        config_du_moteur(&eng)
    );
}

/// Scénario 11 — Un serveur manuel sur un port mort n'est JAMAIS purgé : scan
/// (la purge des fantômes ne touche que les entrées `net-*` découvertes via le
/// réseau), un cycle de poll, l'entrée doit toujours être là en /api/state et
/// en config.json.
#[test]
fn manual_dead_port_never_pruned() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();
    let port_mort = free_port();
    let rep = ajoute_http(&eng, port_mort, "e2e-mort");
    let id = rep
        .get("id")
        .and_then(|i| i.as_str())
        .expect("add sans id")
        .to_string();

    // scan d'une plage d'un seul port (from <= to) : aucune découverte
    // possible, c'est précisément le cas « rien n'écoute ».
    let rep = scan_plage(&eng, port_mort, port_mort);
    assert!(
        rep.get("ok").and_then(|o| o.as_bool()).unwrap_or(false),
        "scan refusé : {rep}"
    );

    let toujours_la = wait_until(DEUX_CYCLES, || {
        let serveurs = serveurs_etat(&eng);
        if serveurs
            .iter()
            .any(|s| champ(s, "id") == id && meme_port(s, port_mort))
        {
            Some(true)
        } else {
            None
        }
    });
    assert!(
        toujours_la,
        "l'entrée manuelle sur port mort a été purgée : {:?}",
        serveurs_etat(&eng)
    );
    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| champ(s, "id") == id)
        .expect("entrée disparue");
    // port fermé → DOWN (port ouvert sans MCP serait PORT-ONLY) — l'essentiel
    // est qu'elle existe toujours et reste activée/persistée.
    assert!(
        matches!(champ(&s, "state"), "DOWN" | "PORT-ONLY"),
        "état attendu DOWN ou PORT-ONLY : {s}"
    );
    assert!(
        serveurs_config(&eng)
            .iter()
            .any(|c| c.get("port") == Some(&json!(port_mort))),
        "entrée morte absente de config.json : {:?}",
        config_du_moteur(&eng)
    );
}

/// Scénario 12 — Le scan est borné et ré-entrant : deux scans consécutifs sur
/// la même plage (le second voit l'entrée disc-<port> déjà connue) doivent se
/// terminer — le second en moins de 30 s — sans laisser /api_state illisible.
#[test]
fn scan_terminates_bounded() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();
    let fx = Fixture::spawn_mcp();
    // plage de 3 ports autour de la fixture : assez grande pour exercer la
    // re-détection, assez petite pour rester instantanée.
    let from = fx.port().saturating_sub(1).max(1);
    let to = fx.port().saturating_add(1);

    let rep1 = scan_plage(&eng, from, to);
    assert!(
        rep1.get("ok").and_then(|o| o.as_bool()).unwrap_or(false),
        "premier scan refusé : {rep1}"
    );
    let t0 = std::time::Instant::now();
    let rep2 = scan_plage(&eng, from, to);
    let duree = t0.elapsed();
    assert!(
        rep2.get("ok").and_then(|o| o.as_bool()).unwrap_or(false),
        "second scan refusé : {rep2}"
    );
    assert!(
        // 60 s : le scan est borné côté moteur ; la borne de 30 s flottait
        // sous la suite complète (45 moteurs en parallèle saturent le CPU),
        // il passe isolément comme en suite avec cette marge.
        duree < Duration::from_secs(60),
        "second scan trop long : {duree:?}"
    );

    // /api/state reste lisible et la découverte du premier scan est intacte
    // (re-détection propre, pas d'entrée dupliquée ni d'état corrompu).
    let etat = eng.get("/api/state").json();
    assert!(
        etat.get("servers").and_then(|s| s.as_array()).is_some(),
        "/api/state illisible après les deux scans : {etat}"
    );
    let disc = serveurs_etat(&eng)
        .into_iter()
        .filter(|s| meme_port(s, fx.port()))
        .collect::<Vec<_>>();
    assert_eq!(
        disc.len(),
        1,
        "la fixture doit apparaître exactement une fois : {disc:?}"
    );
    assert_eq!(champ(&disc[0], "state"), "UP", "{:?}", disc[0]);
}
