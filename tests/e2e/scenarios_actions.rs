//! Scénarios e2e « actions » : chaque bouton, chaque action du dispatcher.
//!
//! INVENTAIRE DE LA SURFACE (union ui.rs × manage.rs::handle_action) :
//!
//! Émis par l'UI (src/ui.rs, POST /api/action) — 25 noms, TOUS connus du
//! dispatcher :
//!   toggle, kill, start, refresh (act()) ; set_autostart ; set_host ;
//!   reorder ; set_start_cmd ; remove (durable) ; open_path ;
//!   set_auto_restart ; scan ; scan_network ; discover ; discover_peers ;
//!   import_peer ; check_update ; reset_state ; set_settings ; ssh_discover ;
//!   ssh_save ; ssh_forget ; toggle_tool ; add ; install_store.
//!
//! Acceptés par le dispatcher (src/manage.rs, arms du match) mais JAMAIS émis
//! par l'UI actuelle — 5 noms « dispatcher-only » :
//!   kill_stdio, start_stdio (alias de kill/start), set_notify,
//!   reset_runtime, import_mcp.
//!   → écart UI↔dispatcher constaté, NON corrigé (rapport seulement).
//!
//! Faits source vérifiés (src/manage.rs au 2026-10-26) :
//! - TOUTE action répond HTTP 200 + JSON avec champ "ok" (booléen) — jamais de
//!   panic, jamais de réponse vide, y compris pour un id inconnu ou un payload
//!   minimal vide.
//! - Erreurs canoniques : "id manquant", "serveur inconnu", "port invalide",
//!   "ip et user requis", "chemin invalide", "chemin introuvable", "ids
//!   requis", "what inconnu (placement | detection | oublis | tout)",
//!   "utilisateur SSH requis (ou enregistre la machine)", "ip manquante",
//!   "tool manquant", "item inconnu", "texte vide", "pair inconnu",
//!   "action inconnue: <nom>".
//! - id inconnu : toggle/start/start_stdio/set_*/toggle_tool → ok:false
//!   "serveur inconnu" ; refresh → ok:TRUE (poll_one silencieux) ; kill →
//!   ok:TRUE (arrêt d'un id inconnu = no-op) ; remove → ok:false sans error.
//! - reorder : payload {"ids":[…]} — ids inconnus filtrés, manquants ajoutés
//!   en fin ; persisté dans settings.server_order.
//! - set_settings : clamp refresh_secs 5..3600, scan_from/scan_to permutés si
//!   inversés, ports réseau filtrés (1..=65535, vide = ignoré), ui_bind
//!   whitelisté ("0.0.0.0" sinon "127.0.0.1").
//! - set_start_cmd : refuse les serveurs distants (host Some) ; poll_one
//!   AUTO-LANCE un serveur HTTP local éteint porteur d'une start_cmd
//!   (auto_restart) → les scénarios qui posent une start_cmd gardent l'entrée
//!   DÉSACTIVÉE (toggle OFF : poll_one sort sur !enabled).
//! - check_update : curl -m 10 puis repli PowerShell -TimeoutSec 10 → ~20 s au
//!   pire hors ligne, réponse {"ok":true,"latest":null|tag} propre.
//! - reset_state : what ∈ placement | detection | oublis | tout ; "tout"
//!   RÉINJECTE les seeds retirés durablement (config e2e : les 8) → le test
//!   relève refresh_secs à 3600 AVANT (le poller relit la valeur à chaque
//!   cycle, src/lib.rs) et attend le cycle déjà armé : plus aucun sondage
//!   pendant le test, les seeds réinjectés ne seront JAMAIS lancés.
//! - install_store : installer pour de vrai lance npx/uvx (téléchargements +
//!   process) — INTERDIT en e2e ; seuls les chemins de validation sont
//!   verrouillés (item inconnu, filesystem sans dossier).
//! - ssh_discover avec user lancerait un ssh réel — jamais testé ; on ne
//!   vérifie que le refus sans machine enregistrée.

use crate::harness::{free_port, spawn_loopback, wait_until, Engine, Fixture, SEED_IDS};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

/// Attente maximale pour qu'une entrée passe UP après un poll.
const DELAI_UP: Duration = Duration::from_secs(30);
/// Attente bornée générique (constat d'absence, purge…).
const DELAI_LIMITE: Duration = Duration::from_secs(15);
/// Timeout de lecture du client HTTP local (post_json_long) : couvre le pire
/// cas check_update hors ligne (~20 s : curl -m 10 + powershell 10 s).
const DELAI_REPONSE: Duration = Duration::from_secs(35);

// ---------- helpers locaux ----------

/// Moteur isolé prêt (harnais : port UI libre, config générée dans un tmp dir
/// unique — autostart false, réseau désactivé, seeds retirés).
fn moteur() -> Engine {
    spawn_loopback()
}

/// Liste des serveurs de /api/state (vide si la réponse est inattendue).
fn serveurs_etat(eng: &Engine) -> Vec<Value> {
    eng.get("/api/state")
        .json()
        .get("servers")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Bloc "settings" de /api/state.
fn reglages_etat(eng: &Engine) -> Value {
    eng.get("/api/state")
        .json()
        .get("settings")
        .cloned()
        .unwrap_or(Value::Null)
}

/// Contenu de config.json du moteur (Null si le fichier n'existe pas encore).
fn config_du_moteur(eng: &Engine) -> Value {
    let chemin = eng.tmp_dir().join("config.json");
    match std::fs::read(&chemin) {
        Ok(octets) => serde_json::from_slice(&octets).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

/// Serveurs du tableau "servers" de config.json.
fn serveurs_config(eng: &Engine) -> Vec<Value> {
    config_du_moteur(eng)
        .get("servers")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Petit accès string sûr sur un champ JSON.
fn champ<'a>(s: &'a Value, cle: &str) -> &'a str {
    s.get(cle).and_then(|v| v.as_str()).unwrap_or("")
}

/// ok booléen d'une réponse du dispatcher (false si absente/inattendue).
fn ok_de(rep: &Value) -> bool {
    rep.get("ok").and_then(|o| o.as_bool()).unwrap_or(false)
}

/// L'array "changed"/"cleared" contient-il cette entrée ?
fn liste_contient(rep: &Value, cle: &str, val: &str) -> bool {
    rep.get(cle)
        .and_then(|c| c.as_array())
        .map(|a| a.iter().any(|x| x.as_str() == Some(val)))
        .unwrap_or(false)
}

/// Ajoute un serveur HTTP local (host absent = local) sur un port mort.
fn ajoute_http_mort(eng: &Engine, label: &str) -> Value {
    eng.action(&json!({
        "action": "add", "transport": "http",
        "port": free_port(), "path": "/mcp", "label": label
    }))
}

/// POST /api/action avec un timeout de lecture LIBRE (le harnais est figé à
/// 10 s — insuffisant pour check_update hors ligne : curl -m 10 + PowerShell
/// 10 s, et pour le scan local synchrone). Retourne (status, JSON parsé) ;
/// status 0 = pas de réponse (connexion perdue / timeout).
fn post_json_long(eng: &Engine, v: &Value, lecture: Duration) -> (u16, Value) {
    let addr = SocketAddr::from(([127, 0, 0, 1], eng.port));
    let Ok(mut flux) = TcpStream::connect_timeout(&addr, Duration::from_secs(5)) else {
        return (0, Value::Null);
    };
    flux.set_read_timeout(Some(lecture)).ok();
    flux.set_write_timeout(Some(Duration::from_secs(5))).ok();
    let corps = v.to_string();
    let requete = format!(
        "POST /api/action HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        eng.port,
        corps.len(),
        corps
    );
    if flux.write_all(requete.as_bytes()).is_err() {
        return (0, Value::Null);
    }
    let mut data = Vec::new();
    let mut tmp = [0u8; 16384];
    loop {
        match flux.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => data.extend_from_slice(&tmp[..n]),
            Err(_) => break,
        }
        if data.len() > 8 * 1024 * 1024 {
            break;
        }
    }
    let Some(sep) = data.windows(4).position(|w| w == b"\r\n\r\n") else {
        return (0, Value::Null);
    };
    let entete = String::from_utf8_lossy(&data[..sep]).to_string();
    let status = entete
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    let corps_json: Value = serde_json::from_slice(&data[sep + 4..]).unwrap_or(Value::Null);
    (status, corps_json)
}

// ---------- contrat du dispatcher ----------

/// Attente contractuelle sur une réponse du dispatcher.
enum Attendu {
    /// ok:true requis
    Vrai,
    /// ok:false requis (pas de champ error garanti — ex. remove introuvable)
    Faux,
    /// ok:false requis avec CE message exact (src/manage.rs au 2026-10-26)
    Erreur(&'static str),
    /// Réponse éventuellement PERDUE : discover (scan_listening) est synchrone
    /// côté moteur et peut dépasser le client — cf. scenarios_scan (DELAI_SCAN).
    PerduTolere,
}

/// Payload minimal SÛR par action ({} sauf exception documentée).
fn charge_minimal(action: &str) -> Value {
    match action {
        // set_autostart : {} voudrait dire enabled:true (défaut) → écrirait une
        // VRAIE clé Run HKCU. enabled:false est le seul minimal sûr.
        "set_autostart" => json!({"action": "set_autostart", "enabled": false}),
        _ => json!({"action": action}),
    }
}

/// Table du contrat : (action, attente du payload minimal).
/// Les attentes verrouillent les messages d'erreur canoniques du dispatcher.
fn contrat() -> Vec<(&'static str, Attendu)> {
    vec![
        ("toggle", Attendu::Erreur("id manquant")),
        ("refresh", Attendu::Erreur("id manquant")),
        ("start", Attendu::Erreur("id manquant")),
        ("kill", Attendu::Erreur("id manquant")),
        ("start_stdio", Attendu::Erreur("id manquant")),
        ("kill_stdio", Attendu::Erreur("id manquant")),
        ("remove", Attendu::Erreur("id manquant")),
        ("add", Attendu::Erreur("port invalide")),
        ("scan", Attendu::Vrai),           // plage config 3000..3000 = 1 port
        ("scan_network", Attendu::Vrai),   // asynchrone : démarrage seulement
        ("discover_peers", Attendu::Vrai), // réseau désactivé → no-op immédiat
        ("discover", Attendu::PerduTolere), // scan local synchrone (long)
        ("ssh_save", Attendu::Erreur("ip et user requis")),
        ("set_start_cmd", Attendu::Erreur("id manquant")),
        ("set_host", Attendu::Erreur("id manquant")),
        ("ssh_forget", Attendu::Faux), // {"ok":false} sans error (ip "")
        ("open_path", Attendu::Erreur("chemin invalide")),
        ("reorder", Attendu::Erreur("ids requis")),
        ("set_settings", Attendu::Vrai),  // changed: []
        ("set_notify", Attendu::Vrai),    // dispatcher-only
        ("reset_runtime", Attendu::Vrai), // dispatcher-only, cleared: 0
        ("check_update", Attendu::Vrai),  // borné ~20 s hors ligne
        (
            "reset_state",
            Attendu::Erreur("what inconnu (placement | detection | oublis | tout)"),
        ),
        ("set_autostart", Attendu::Vrai),
        ("set_auto_restart", Attendu::Erreur("id manquant")),
        (
            "ssh_discover",
            Attendu::Erreur("utilisateur SSH requis (ou enregistre la machine)"),
        ),
        ("import_peer", Attendu::Erreur("ip manquante")),
        ("toggle_tool", Attendu::Erreur("id manquant")),
        ("install_store", Attendu::Erreur("item inconnu")),
        ("import_mcp", Attendu::Erreur("texte vide")), // dispatcher-only
        (
            "action_inexistante_e2e",
            Attendu::Erreur("action inconnue: action_inexistante_e2e"),
        ),
    ]
}

/// Table « id inconnu » : (action, payload avec id bogus, attente).
/// Documente le comportement RÉEL de chaque bouton face à un id disparu.
fn contrat_id_inconnu() -> Vec<(&'static str, Value, Attendu)> {
    let bogus = "inexistant-e2e";
    vec![
        (
            "toggle",
            json!({"action": "toggle", "id": bogus}),
            Attendu::Erreur("serveur inconnu"),
        ),
        // refresh : poll_one silencieux sur id inconnu → ok:true (contrat réel)
        (
            "refresh",
            json!({"action": "refresh", "id": bogus}),
            Attendu::Vrai,
        ),
        (
            "start",
            json!({"action": "start", "id": bogus}),
            Attendu::Erreur("serveur inconnu"),
        ),
        // kill : arrêt d'un id inconnu = no-op → ok:true (contrat réel)
        (
            "kill",
            json!({"action": "kill", "id": bogus}),
            Attendu::Vrai,
        ),
        (
            "start_stdio",
            json!({"action": "start_stdio", "id": bogus}),
            Attendu::Erreur("serveur inconnu"),
        ),
        (
            "kill_stdio",
            json!({"action": "kill_stdio", "id": bogus}),
            Attendu::Vrai,
        ),
        // remove introuvable : {"ok":false} SANS champ error
        (
            "remove",
            json!({"action": "remove", "id": bogus}),
            Attendu::Faux,
        ),
        (
            "set_start_cmd",
            json!({"action": "set_start_cmd", "id": bogus, "program": "node"}),
            Attendu::Erreur("serveur inconnu"),
        ),
        (
            "set_host",
            json!({"action": "set_host", "id": bogus, "host": "127.0.0.1"}),
            Attendu::Erreur("serveur inconnu"),
        ),
        (
            "set_auto_restart",
            json!({"action": "set_auto_restart", "id": bogus, "enabled": false}),
            Attendu::Erreur("serveur inconnu"),
        ),
        (
            "toggle_tool",
            json!({"action": "toggle_tool", "id": bogus, "tool": "ping"}),
            Attendu::Erreur("serveur inconnu"),
        ),
        // reorder : ids inconnus FILTRÉS → ok:true, count 0
        (
            "reorder",
            json!({"action": "reorder", "ids": [bogus]}),
            Attendu::Vrai,
        ),
    ]
}

/// Vérifie une réponse contre son attente (200 + champ ok + message éventuel).
fn verifie_contrat(action: &str, status: u16, rep: &Value, attendu: &Attendu) {
    match attendu {
        Attendu::PerduTolere => {
            // réponse perdue acceptable (scan synchrone plus long que le
            // client) ; si réponse il y a : 200 + champ ok présent.
            if status != 0 {
                assert_eq!(status, 200, "{action} : status inattendu");
                assert!(rep.get("ok").is_some(), "{action} sans champ ok : {rep}");
            }
        }
        _ => {
            assert_eq!(
                status, 200,
                "{action} : le dispatcher doit répondre 200 (obtenu : {status})"
            );
            assert!(
                rep.get("ok").is_some(),
                "{action} : pas de champ \"ok\" — contrat du dispatcher rompu : {rep}"
            );
            assert!(
                rep.is_object(),
                "{action} : réponse non-objet — jamais un panic ni un corps vide : {rep}"
            );
        }
    }
    match attendu {
        Attendu::Vrai => assert!(
            ok_de(rep),
            "{action} : ok:true attendu au payload minimal : {rep}"
        ),
        Attendu::Faux => assert!(!ok_de(rep), "{action} : ok:false attendu : {rep}"),
        Attendu::Erreur(msg) => {
            assert!(!ok_de(rep), "{action} : ok:false attendu : {rep}");
            assert_eq!(
                rep.get("error").and_then(|e| e.as_str()),
                Some(*msg),
                "{action} : message d'erreur canonique attendu : {rep}"
            );
        }
        Attendu::PerduTolere => {}
    }
}

// ---------- scénarios ----------

/// Scénario A1 — CONTRAT DU DISPATCHER : pour CHAQUE action de l'union
/// (25 UI + 5 dispatcher-only + 1 action inconnue), le payload minimal et le
/// payload à id bogus doivent répondre HTTP 200 avec un JSON objet portant un
/// champ "ok" — jamais de panic, jamais de réponse vide — et les erreurs
/// canoniques attendues. Verrouille la surface de tous les boutons.
#[test]
fn contrat_dispatcher_toutes_actions() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();

    // Passe 1 : payload minimal ({} ; set_autostart → enabled:false).
    for (action, attendu) in contrat() {
        let (status, rep) = post_json_long(&eng, &charge_minimal(action), DELAI_REPONSE);
        verifie_contrat(action, status, &rep, &attendu);
    }

    // Passe 2 : id bogus "inexistant-e2e" — comportement réel par bouton.
    for (action, payload, attendu) in contrat_id_inconnu() {
        let (status, rep) = post_json_long(&eng, &payload, DELAI_REPONSE);
        verifie_contrat(action, status, &rep, &attendu);
    }

    // Le moteur a survécu à toute la salve : /api/hello répond toujours.
    let hello = eng.get("/api/hello");
    assert_eq!(
        hello.status, 200,
        "le moteur doit survivre à la salve d'actions"
    );
}

/// Scénario A2 — set_settings (dialog ⚙) : refresh/scan/ports appliqués et
/// reflétés dans /api/state, ui_bind inchangé, clamps (plancher 5, plafond
/// 3600), permutation scan_from/scan_to, ports invalides filtrés, et
/// persistance dans config.json.
#[test]
fn set_settings_applique_et_clamp() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();

    // Application principale (network_ports en NUMÉRIQUE : les strings sont
    // filtrées par as_u64 et un résultat vide est ignoré — contrat réel).
    let rep = eng.action(&json!({
        "action": "set_settings",
        "refresh_secs": 10,
        "scan_from": 5000,
        "scan_to": 5010,
        "network_ports": [5090],
        "ui_bind": "127.0.0.1"
    }));
    assert!(ok_de(&rep), "set_settings refusé : {rep}");
    assert!(
        liste_contient(&rep, "changed", "refresh")
            && liste_contient(&rep, "changed", "scan")
            && liste_contient(&rep, "changed", "network_ports"),
        "changed doit lister refresh+scan+network_ports : {rep}"
    );
    assert!(
        !liste_contient(&rep, "changed", "ui_bind"),
        "ui_bind identique au réglage courant : aucun changement attendu : {rep}"
    );

    let reg = reglages_etat(&eng);
    assert_eq!(
        reg.get("refresh_secs").and_then(|v| v.as_u64()),
        Some(10),
        "refresh_secs appliqué : {reg}"
    );
    assert_eq!(
        reg.get("scan_from").and_then(|v| v.as_u64()),
        Some(5000),
        "{reg}"
    );
    assert_eq!(
        reg.get("scan_to").and_then(|v| v.as_u64()),
        Some(5010),
        "{reg}"
    );
    assert_eq!(
        reg.get("network_ports").and_then(|v| v.as_array()),
        Some(&vec![json!(5090)]),
        "network_ports appliqué : {reg}"
    );
    assert_eq!(
        champ(&reg, "ui_bind"),
        "127.0.0.1",
        "ui_bind inchangé : {reg}"
    );

    // Clamp plancher : 1 → 5.
    let rep = eng.action(&json!({"action": "set_settings", "refresh_secs": 1}));
    assert!(ok_de(&rep), "{rep}");
    assert_eq!(
        reglages_etat(&eng)
            .get("refresh_secs")
            .and_then(|v| v.as_u64()),
        Some(5),
        "refresh_secs 1 doit être clampé au plancher 5"
    );

    // Clamp plafond : 99999 → 3600.
    let rep = eng.action(&json!({"action": "set_settings", "refresh_secs": 99999}));
    assert!(ok_de(&rep), "{rep}");
    assert_eq!(
        reglages_etat(&eng)
            .get("refresh_secs")
            .and_then(|v| v.as_u64()),
        Some(3600),
        "refresh_secs 99999 doit être clampé au plafond 3600"
    );

    // scan_from > scan_to → permutation silencieuse.
    let rep = eng.action(&json!({
        "action": "set_settings", "scan_from": 5020, "scan_to": 5010
    }));
    assert!(ok_de(&rep), "{rep}");
    let reg = reglages_etat(&eng);
    assert_eq!(
        reg.get("scan_from").and_then(|v| v.as_u64()),
        Some(5010),
        "{reg}"
    );
    assert_eq!(
        reg.get("scan_to").and_then(|v| v.as_u64()),
        Some(5020),
        "{reg}"
    );

    // Ports invalides (0, 70000) → filtrés → vide → IGNORÉ (toujours [5090]).
    let rep = eng.action(&json!({"action": "set_settings", "network_ports": [0, 70000]}));
    assert!(ok_de(&rep), "{rep}");
    assert_eq!(
        reglages_etat(&eng)
            .get("network_ports")
            .and_then(|v| v.as_array()),
        Some(&vec![json!(5090)]),
        "une liste de ports entièrement invalide doit être ignorée"
    );

    // Persistance : config.json porte les réglages appliqués.
    let cfg = config_du_moteur(&eng);
    let reg_cfg = cfg.get("settings").cloned().unwrap_or(Value::Null);
    assert_eq!(
        reg_cfg.get("refresh_secs").and_then(|v| v.as_u64()),
        Some(3600),
        "config.json : refresh_secs clampé persisté : {cfg}"
    );
    assert_eq!(
        reg_cfg.get("network_ports").and_then(|v| v.as_array()),
        Some(&vec![json!(5090)]),
        "config.json : network_ports persistés : {cfg}"
    );
}

/// Scénario A3 — reorder (glisser-déposer) : le nouvel ordre est reflété par
/// /api/state et persisté dans settings.server_order (config.json) ; les ids
/// inconnus sont filtrés, les manquants réajoutés en fin.
#[test]
fn reorder_change_ordre_et_persiste() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();

    let rep_a = ajoute_http_mort(&eng, "ord-a");
    // now_compact() est en millisecondes : deux adds rapprochés peuvent créer
    // le MÊME id — on respire et on exige la différence.
    std::thread::sleep(Duration::from_millis(20));
    let rep_b = ajoute_http_mort(&eng, "ord-b");
    assert!(
        ok_de(&rep_a) && ok_de(&rep_b),
        "add refusé : {rep_a} / {rep_b}"
    );
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
        !id_a.is_empty() && !id_b.is_empty() && id_a != id_b,
        "{rep_a} / {rep_b}"
    );

    // Ordre initial (insertion) : A avant B dans /api/state.
    let positions = |ids: &[&str], serveurs: &[Value]| -> Vec<usize> {
        ids.iter()
            .map(|id| {
                serveurs
                    .iter()
                    .position(|s| champ(s, "id") == *id)
                    .unwrap_or(9999)
            })
            .collect()
    };
    let serveurs = serveurs_etat(&eng);
    let pos = positions(&[&id_a, &id_b], &serveurs);
    assert!(
        pos[0] < pos[1],
        "ordre initial attendu A puis B : {pos:?} sur {serveurs:?}"
    );

    // reorder B,A → /api/state inverse l'ordre, config.json persiste.
    // NB : l'auto-scan du boot (scan_listening à +1.2 s) peut avoir ajouté des
    // entrées disc-* pour des ports réellement en écoute sur la machine — le
    // dispatcher réajoute ces ids inconnus du payload EN FIN ; on asserte donc
    // le PRÉFIXE [B,A] et l'ordre relatif, pas une égalité stricte.
    let rep = eng.action(&json!({"action": "reorder", "ids": [&id_b, &id_a]}));
    assert!(ok_de(&rep), "reorder refusé : {rep}");
    let serveurs = serveurs_etat(&eng);
    let pos = positions(&[&id_a, &id_b], &serveurs);
    assert!(
        pos[1] < pos[0],
        "après reorder B,A : {pos:?} sur {serveurs:?}"
    );
    let ordre_cfg = config_du_moteur(&eng)
        .get("settings")
        .and_then(|s| s.get("server_order"))
        .and_then(|o| o.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        ordre_cfg.get(..2),
        Some(&vec![json!(id_b), json!(id_a)][..]),
        "settings.server_order persisté (préfixe B,A) : {ordre_cfg:?}"
    );

    // Ids inconnus filtrés, manquants réajoutés en fin : ["inconnu", A] →
    // A en tête, B réajouté après (les entrées étrangères de l'auto-scan /
    // annonces litemcp de la machine peuvent s'intercaler entre les deux).
    let rep = eng.action(&json!({"action": "reorder", "ids": ["inexistant-e2e", &id_a]}));
    assert!(ok_de(&rep), "reorder avec id inconnu refusé : {rep}");
    assert!(
        rep.get("count").and_then(|c| c.as_u64()).unwrap_or(0) >= 2,
        "{rep}"
    );
    let ordre_cfg = config_du_moteur(&eng)
        .get("settings")
        .and_then(|s| s.get("server_order"))
        .and_then(|o| o.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        ordre_cfg.first(),
        Some(&json!(id_a)),
        "id inconnu filtré, A réordonné en tête : {ordre_cfg:?}"
    );
    assert!(
        !ordre_cfg.iter().any(|x| x == &json!("inexistant-e2e")),
        "l'id inconnu ne doit pas être persisté : {ordre_cfg:?}"
    );
    let pos_a = ordre_cfg
        .iter()
        .position(|x| x == &json!(id_a))
        .unwrap_or(9999);
    let pos_b = ordre_cfg
        .iter()
        .position(|x| x == &json!(id_b))
        .unwrap_or(9998);
    assert!(
        pos_a < pos_b,
        "B manquant du payload doit être réajouté APRÈS A : {ordre_cfg:?}"
    );
}

/// Scénario A4 — set_host (glisser-déposer sur un groupe machine) : le host
/// du transport passe de None (local) à la valeur posée, reflétée en
/// /api/state et persistée ; un serveur devenu DISTANT refuse set_start_cmd.
#[test]
fn set_host_passe_local_en_distant() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();

    let rep = ajoute_http_mort(&eng, "e2e-host");
    assert!(ok_de(&rep), "add refusé : {rep}");
    let id = rep
        .get("id")
        .and_then(|i| i.as_str())
        .unwrap_or("")
        .to_string();

    let rep = eng.action(&json!({"action": "set_host", "id": id, "host": "127.0.0.1"}));
    assert!(ok_de(&rep), "set_host refusé : {rep}");
    assert_eq!(
        rep.get("old_host").and_then(|h| h.as_str()),
        None,
        "l'ancien host d'un serveur local est null : {rep}"
    );

    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| champ(s, "id") == id)
        .expect("entrée disparue");
    assert_eq!(
        s.get("transport_info")
            .and_then(|t| t.get("host"))
            .and_then(|h| h.as_str()),
        Some("127.0.0.1"),
        "transport_info.host reflété : {s}"
    );
    let cfg = serveurs_config(&eng)
        .into_iter()
        .find(|c| champ(c, "label") == "e2e-host")
        .expect("entrée absente de config.json");
    assert_eq!(
        cfg.get("host").and_then(|h| h.as_str()),
        Some("127.0.0.1"),
        "host persisté dans config.json : {cfg}"
    );

    // Devenu distant (host Some) : la commande de lancement appartient à sa
    // machine hôte — refus canonique.
    let rep = eng.action(&json!({"action": "set_start_cmd", "id": id, "program": "node"}));
    assert!(
        !ok_de(&rep),
        "set_start_cmd sur distant doit refuser : {rep}"
    );
    assert_eq!(
        rep.get("error").and_then(|e| e.as_str()),
        Some("serveur distant : la commande de lancement appartient à sa machine hôte"),
        "{rep}"
    );
}

/// Scénario A5 — set_start_cmd (panneau Démarrage) : la commande posée est
/// reflétée en /api/state (start_cmd + can_start) et persistée. L'entrée est
/// gardée DÉSACTIVÉE pendant tout le test : poll_one auto-lance un serveur
/// HTTP local éteint porteur d'une start_cmd — désactivé, il ne spawn JAMAIS
/// (aucun process réel pendant l'e2e).
#[test]
fn set_start_cmd_reflechi_et_persiste() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();

    let rep = ajoute_http_mort(&eng, "e2e-startcmd");
    assert!(ok_de(&rep), "add refusé : {rep}");
    let id = rep
        .get("id")
        .and_then(|i| i.as_str())
        .unwrap_or("")
        .to_string();

    // OFF d'abord : verrou anti-spawn documenté en tête de scénario.
    let rep = eng.action(&json!({"action": "toggle", "id": id}));
    assert_eq!(
        rep.get("enabled").and_then(|e| e.as_bool()),
        Some(false),
        "{rep}"
    );

    let rep = eng.action(&json!({
        "action": "set_start_cmd", "id": id,
        "program": "node", "args": ["--version"], "workdir": null
    }));
    assert!(ok_de(&rep), "set_start_cmd refusé : {rep}");

    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| champ(s, "id") == id)
        .expect("entrée disparue");
    let start_cmd = s.get("start_cmd").cloned().unwrap_or(Value::Null);
    assert_eq!(
        start_cmd.get("program").and_then(|p| p.as_str()),
        Some("node"),
        "start_cmd.program reflété : {s}"
    );
    assert_eq!(
        start_cmd.get("args").and_then(|a| a.as_array()),
        Some(&vec![json!("--version")]),
        "start_cmd.args reflété : {s}"
    );
    assert_eq!(
        s.get("can_start").and_then(|c| c.as_bool()),
        Some(true),
        "can_start dérivé de start_cmd : {s}"
    );
    let cfg = serveurs_config(&eng)
        .into_iter()
        .find(|c| champ(c, "label") == "e2e-startcmd")
        .expect("entrée absente de config.json");
    assert_eq!(
        cfg.get("program").and_then(|p| p.as_str()),
        Some("node"),
        "program persisté dans config.json : {cfg}"
    );
    assert_eq!(
        cfg.get("args").and_then(|a| a.as_array()),
        Some(&vec![json!("--version")]),
        "args persistés dans config.json : {cfg}"
    );
}

/// Scénario A6 — set_auto_restart (bouton ⚡ autoboot par serveur) : le
/// drapeau est reflété en /api/state et persisté (config.json n'écrit que la
/// DÉSACTIVATION — le défaut est true).
#[test]
fn set_auto_restart_persiste() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();

    let rep = ajoute_http_mort(&eng, "e2e-autorestart");
    assert!(ok_de(&rep), "add refusé : {rep}");
    let id = rep
        .get("id")
        .and_then(|i| i.as_str())
        .unwrap_or("")
        .to_string();

    // Défaut : auto_restart true (add pose le drapeau).
    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| champ(s, "id") == id)
        .expect("entrée disparue");
    assert_eq!(
        s.get("auto_restart").and_then(|a| a.as_bool()),
        Some(true),
        "{s}"
    );

    // OFF : réponse, /api/state ET config.json (clé écrite car non-défaut).
    let rep = eng.action(&json!({"action": "set_auto_restart", "id": id, "enabled": false}));
    assert!(ok_de(&rep), "set_auto_restart refusé : {rep}");
    assert_eq!(
        rep.get("auto_restart").and_then(|a| a.as_bool()),
        Some(false),
        "{rep}"
    );
    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| champ(s, "id") == id)
        .expect("entrée disparue");
    assert_eq!(
        s.get("auto_restart").and_then(|a| a.as_bool()),
        Some(false),
        "{s}"
    );
    let cfg = serveurs_config(&eng)
        .into_iter()
        .find(|c| champ(c, "label") == "e2e-autorestart")
        .expect("entrée absente de config.json");
    assert_eq!(
        cfg.get("auto_restart").and_then(|a| a.as_bool()),
        Some(false),
        "auto_restart:false persisté dans config.json : {cfg}"
    );

    // ON : retour au défaut — config.json ne réécrit PAS la clé.
    let rep = eng.action(&json!({"action": "set_auto_restart", "id": id, "enabled": true}));
    assert!(ok_de(&rep), "{rep}");
    assert_eq!(
        rep.get("auto_restart").and_then(|a| a.as_bool()),
        Some(true),
        "{rep}"
    );
    let cfg = serveurs_config(&eng)
        .into_iter()
        .find(|c| champ(c, "label") == "e2e-autorestart")
        .expect("entrée absente de config.json");
    assert_ne!(
        cfg.get("auto_restart").and_then(|a| a.as_bool()),
        Some(false),
        "auto_restart:true est le défaut : la clé ne doit pas être écrite : {cfg}"
    );
}

/// Scénario A7 — toggle_tool (bouton d'un outil du panneau détail) : sur un
/// serveur UP à 2 outils (fixture), désactiver "ping" fait tomber tools_count
/// à 1 avec tools_disabled=["ping"] (persisté), réactiver restaure 2.
#[test]
fn toggle_tool_desactive_et_reactive() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();
    let fx = Fixture::spawn_mcp();

    let rep = eng.action(&json!({
        "action": "add", "transport": "http",
        "host": "127.0.0.1", "port": fx.port(), "path": "/mcp", "label": "e2e-tools"
    }));
    assert!(ok_de(&rep), "add refusé : {rep}");
    let id = rep
        .get("id")
        .and_then(|i| i.as_str())
        .unwrap_or("")
        .to_string();
    eng.action(&json!({"action": "refresh", "id": id}));

    let up = wait_until(DELAI_UP, || {
        let serveurs = serveurs_etat(&eng);
        match serveurs.iter().find(|s| champ(s, "id") == id) {
            Some(s) if s.get("tools_count").and_then(|t| t.as_u64()) == Some(2) => Some(true),
            _ => None,
        }
    });
    assert!(
        up,
        "pré-condition : la fixture n'a jamais annoncé ses 2 outils"
    );

    // Désactive "ping" → tools_count 1, tools_disabled ["ping"], total 2.
    let rep = eng.action(&json!({"action": "toggle_tool", "id": id, "tool": "ping"}));
    assert!(ok_de(&rep), "toggle_tool refusé : {rep}");
    assert_eq!(
        rep.get("disabled").and_then(|d| d.as_bool()),
        Some(true),
        "{rep}"
    );
    assert_eq!(
        rep.get("tools_actifs").and_then(|t| t.as_u64()),
        Some(1),
        "{rep}"
    );
    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| champ(s, "id") == id)
        .expect("entrée disparue");
    assert_eq!(
        s.get("tools_count").and_then(|t| t.as_u64()),
        Some(1),
        "{s}"
    );
    assert_eq!(
        s.get("tools_total").and_then(|t| t.as_u64()),
        Some(2),
        "{s}"
    );
    assert_eq!(
        s.get("tools_disabled").and_then(|t| t.as_array()),
        Some(&vec![json!("ping")]),
        "{s}"
    );
    let cfg = serveurs_config(&eng)
        .into_iter()
        .find(|c| champ(c, "label") == "e2e-tools")
        .expect("entrée absente de config.json");
    assert_eq!(
        cfg.get("tools_disabled").and_then(|t| t.as_array()),
        Some(&vec![json!("ping")]),
        "tools_disabled persisté dans config.json : {cfg}"
    );

    // Réactive → retour à 2 outils actifs, liste vide.
    let rep = eng.action(&json!({"action": "toggle_tool", "id": id, "tool": "ping"}));
    assert!(ok_de(&rep), "{rep}");
    assert_eq!(
        rep.get("disabled").and_then(|d| d.as_bool()),
        Some(false),
        "{rep}"
    );
    assert_eq!(
        rep.get("tools_actifs").and_then(|t| t.as_u64()),
        Some(2),
        "{rep}"
    );
    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| champ(s, "id") == id)
        .expect("entrée disparue");
    assert_eq!(
        s.get("tools_count").and_then(|t| t.as_u64()),
        Some(2),
        "{s}"
    );
    assert_eq!(
        s.get("tools_disabled").and_then(|t| t.as_array()),
        Some(&Vec::new()),
        "tools_disabled doit être vide après réactivation : {s}"
    );
}

/// Scénario A8 — import_mcp (« collez n'importe quoi », dispatcher-only) :
/// un bloc {"mcpServers":{…}} décrivant un serveur HTTP sur port mort crée
/// l'entrée store-<nom> (origin manual, reflétée en /api/state) ; le
/// ré-import est idempotent (added:0) ; les textes non reconnus et les URL
/// https sont refusés avec un message clair.
/// NB : seules les entrées "url" sont testées — une entrée "command" (npx/node)
/// serait un serveur stdio spawnable par le poller (effet de bord interdit).
#[test]
fn import_mcp_json_et_rejets() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();
    let port_mort = free_port();

    let texte = format!(
        r#"{{"mcpServers":{{"import-mort":{{"url":"http://127.0.0.1:{port_mort}/mcp"}}}}}}"#
    );
    let rep = eng.action(&json!({"action": "import_mcp", "text": texte}));
    assert!(ok_de(&rep), "import_mcp refusé : {rep}");
    assert_eq!(rep.get("added").and_then(|a| a.as_u64()), Some(1), "{rep}");
    assert_eq!(champ(&rep, "source"), "json", "{rep}");

    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| champ(s, "id") == "store-import-mort")
        .expect("l'entrée store-import-mort n'apparaît pas dans /api/state");
    assert_eq!(champ(&s, "transport"), "http", "{s}");
    assert_eq!(champ(&s, "origin"), "manual", "{s}");
    assert_eq!(
        s.get("transport_info")
            .and_then(|t| t.get("port"))
            .and_then(|p| p.as_u64()),
        Some(port_mort as u64),
        "{s}"
    );
    assert_eq!(
        s.get("transport_info")
            .and_then(|t| t.get("host"))
            .and_then(|h| h.as_str()),
        Some("127.0.0.1"),
        "{s}"
    );

    // Ré-import : déjà présent → added:0 (idempotent).
    let rep = eng.action(&json!({"action": "import_mcp", "text": texte}));
    assert!(ok_de(&rep), "{rep}");
    assert_eq!(rep.get("added").and_then(|a| a.as_u64()), Some(0), "{rep}");

    // Texte non reconnu → refus explicite.
    let rep = eng.action(&json!({"action": "import_mcp", "text": "bonjour e2e rien ici"}));
    assert!(!ok_de(&rep), "{rep}");
    assert!(
        rep.get("error")
            .and_then(|e| e.as_str())
            .map(|e| e.contains("rien de reconnu"))
            .unwrap_or(false),
        "{rep}"
    );

    // URL https → TLS non supporté (client std sans rustls) : refus clair.
    let rep = eng.action(&json!({
        "action": "import_mcp", "text": "https://exemple.invalid/mcp"
    }));
    assert!(!ok_de(&rep), "{rep}");
    assert!(
        rep.get("error")
            .and_then(|e| e.as_str())
            .map(|e| e.contains("TLS non supporté"))
            .unwrap_or(false),
        "{rep}"
    );
}

/// Scénario A9 — install_store (boutique) : CHEMINS DE VALIDATION SEULEMENT.
/// L'installation réelle lance npx/uvx (téléchargements réseau + process) —
/// effet de bord interdit en e2e ; on verrouille donc le refus d'un item
/// inconnu et le garde-fou « dossier filesystem explicite », en prouvant
/// qu'AUCUNE entrée n'a été créée.
#[test]
fn install_store_validation_sans_installation() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();

    // Item inconnu → refus canonique.
    let rep = eng.action(&json!({"action": "install_store", "id": "item-inexistant-e2e"}));
    assert!(!ok_de(&rep), "{rep}");
    assert_eq!(
        rep.get("error").and_then(|e| e.as_str()),
        Some("item inconnu"),
        "{rep}"
    );

    // filesystem sans dossier explicite → refus (jamais un chemin personnel
    // par défaut dans un binaire distribué).
    let rep = eng.action(&json!({"action": "install_store", "id": "filesystem"}));
    assert!(
        !ok_de(&rep),
        "install filesystem sans dossier doit refuser : {rep}"
    );
    assert!(
        rep.get("error")
            .and_then(|e| e.as_str())
            .map(|e| e.starts_with("précisez le dossier à exposer"))
            .unwrap_or(false),
        "{rep}"
    );

    // Aucun effet de bord : aucune entrée store-* créée.
    let serveurs = serveurs_etat(&eng);
    assert!(
        serveurs
            .iter()
            .all(|s| !champ(s, "id").starts_with("store-")),
        "une validation ne doit créer aucune entrée store-* : {serveurs:?}"
    );
}

/// Scénario A10 — ssh_save / ssh_forget / ssh_discover (groupes Machines),
/// SANS SSH réel : ssh_discover sans machine enregistrée refuse proprement
/// (aucun process ssh lancé), ssh_save enregistre/met à jour et persiste,
/// ssh_forget oublie (ok:false la seconde fois).
/// NB : ssh_discover AVEC user lancerait un vrai ssh + netstat distant —
/// jamais testé ici.
#[test]
fn ssh_machines_sans_ssh_reel() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();
    // 203.0.113.x = TEST-NET-3 (RFC 5737) : jamais routé, jamais contacté —
    // et de toute façon ssh_discover est refusé AVANT toute connexion.
    let ip = "203.0.113.77";

    // Refus canonique sans user ni machine enregistrée.
    let rep = eng.action(&json!({"action": "ssh_discover", "ip": ip}));
    assert!(!ok_de(&rep), "{rep}");
    assert_eq!(
        rep.get("error").and_then(|e| e.as_str()),
        Some("utilisateur SSH requis (ou enregistre la machine)"),
        "{rep}"
    );

    // ssh_save sans user → refus.
    let rep = eng.action(&json!({"action": "ssh_save", "ip": ip}));
    assert!(!ok_de(&rep), "{rep}");
    assert_eq!(
        rep.get("error").and_then(|e| e.as_str()),
        Some("ip et user requis"),
        "{rep}"
    );

    // Enregistrement → /api/state (settings.ssh_hosts) + config.json.
    let rep = eng.action(&json!({"action": "ssh_save", "ip": ip, "user": "e2e-user"}));
    assert!(ok_de(&rep), "ssh_save refusé : {rep}");
    let reg = reglages_etat(&eng);
    assert_eq!(
        reg.get("ssh_hosts").and_then(|h| h.as_array()),
        Some(&vec![json!({"ip": ip, "user": "e2e-user"})]),
        "machine enregistrée dans /api/state : {reg}"
    );
    let reg_cfg = config_du_moteur(&eng)
        .get("settings")
        .cloned()
        .unwrap_or(Value::Null);
    assert_eq!(
        reg_cfg.get("ssh_hosts").and_then(|h| h.as_array()),
        Some(&vec![json!({"ip": ip, "user": "e2e-user"})]),
        "machine persistée dans config.json : {reg_cfg}"
    );

    // Même ip, autre user → mise à jour (pas de doublon).
    let rep = eng.action(&json!({"action": "ssh_save", "ip": ip, "user": "e2e-user-2"}));
    assert!(ok_de(&rep), "{rep}");
    let reg = reglages_etat(&eng);
    let hosts = reg
        .get("ssh_hosts")
        .and_then(|h| h.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(hosts.len(), 1, "mise à jour sans doublon : {hosts:?}");
    assert_eq!(
        hosts[0].get("user").and_then(|u| u.as_str()),
        Some("e2e-user-2")
    );

    // Oubli → ok:true puis ok:false (déjà oubliée).
    let rep = eng.action(&json!({"action": "ssh_forget", "ip": ip}));
    assert!(ok_de(&rep), "ssh_forget refusé : {rep}");
    let rep = eng.action(&json!({"action": "ssh_forget", "ip": ip}));
    assert!(
        !ok_de(&rep),
        "second ssh_forget doit donner ok:false : {rep}"
    );
    assert!(
        reglages_etat(&eng)
            .get("ssh_hosts")
            .and_then(|h| h.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(false),
        "ssh_hosts doit être vide après oubli"
    );
}

/// Scénario A11 — discover_peers + import_peer (pairs MCP Control) : avec le
/// réseau DÉSACTIVÉ (config e2e), discover_peers est un no-op immédiat
/// ok:true (graceful path réel — pas une erreur) et les pairs restent vides ;
/// import_peer d'une ip inconnue refuse avec "pair inconnu". Aucun paquet
/// réseau n'est émis (garde network_enabled en tête de discover_peers).
#[test]
fn pairs_et_import_peer_graceful() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();

    let t0 = Instant::now();
    let rep = eng.action(&json!({"action": "discover_peers"}));
    assert!(ok_de(&rep), "discover_peers (réseau désactivé) : {rep}");
    assert!(
        t0.elapsed() < Duration::from_secs(10),
        "discover_peers doit être immédiat quand le réseau est désactivé : {:?}",
        t0.elapsed()
    );
    assert!(
        eng.get("/api/state")
            .json()
            .get("peers")
            .and_then(|p| p.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(false),
        "aucun pair ne doit apparaître (réseau désactivé)"
    );

    // Import depuis un pair jamais découvert → refus canonique.
    let rep = eng.action(&json!({"action": "import_peer", "ip": "203.0.113.99"}));
    assert!(!ok_de(&rep), "{rep}");
    assert_eq!(
        rep.get("error").and_then(|e| e.as_str()),
        Some("pair inconnu"),
        "{rep}"
    );
}

/// Scénario A12 — reset_state what="detection" : les entrées DÉCOUVERTES
/// (scan local) sont retirées de /api/state, les pairs oubliés — et rien
/// d'autre. La détection est fabriquée avec un scan d'UN port (fixture MCP
/// réelle) : synchrone, rapide, déterministe.
#[test]
fn reset_state_detection_purge_decouvertes() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();
    let fx = Fixture::spawn_mcp();

    let rep = eng.action(&json!({
        "action": "scan", "from": fx.port(), "to": fx.port()
    }));
    assert!(ok_de(&rep), "scan refusé : {rep}");
    let decouvert = wait_until(DELAI_UP, || {
        let serveurs = serveurs_etat(&eng);
        match serveurs.iter().find(|s| {
            champ(s, "origin") == "discovered"
                && s.get("transport_info")
                    .and_then(|t| t.get("port"))
                    .and_then(|p| p.as_u64())
                    == Some(fx.port() as u64)
        }) {
            Some(s) if champ(s, "state") == "UP" => Some(true),
            _ => None,
        }
    });
    assert!(
        decouvert,
        "pré-condition : la fixture n'a pas été découverte"
    );

    let rep = eng.action(&json!({"action": "reset_state", "what": "detection"}));
    assert!(ok_de(&rep), "reset_state detection refusé : {rep}");
    assert_eq!(
        rep.get("cleared").and_then(|c| c.as_array()),
        Some(&vec![json!("detection")]),
        "{rep}"
    );
    let retirés = rep.get("removed").and_then(|r| r.as_u64()).unwrap_or(0);
    assert!(
        retirés >= 1,
        "au moins une entrée découverte retirée : {rep}"
    );

    let purge = wait_until(DELAI_LIMITE, || {
        let reste = serveurs_etat(&eng)
            .iter()
            .any(|s| champ(s, "origin") == "discovered");
        if reste {
            None
        } else {
            Some(true)
        }
    });
    assert!(
        purge,
        "une entrée discovered a survécu à reset_state detection"
    );
    // Les découvertes ne sont jamais persistées : config.json n'en contient pas.
    assert!(
        !serveurs_config(&eng)
            .iter()
            .any(|c| c.get("port") == Some(&json!(fx.port()))),
        "une entrée discovered traîne dans config.json : {:?}",
        config_du_moteur(&eng)
    );
}

/// Scénario A13 — reset_state what="tout" : placement + detection + oublis en
/// une fois — server_order vidé, machines SSH oubliées, seeds retirés
/// durablement RÉINJECTÉS (les 8 de la config e2e).
///
/// ANTI-SPAWN : les seeds réinjectés portent une start_cmd réelle (le harnais
/// les retire précisément pour ça). Le poller relit refresh_secs à CHAQUE
/// cycle (src/lib.rs) : on relève le réglage à 3600 s AVANT, on laisse passer
/// le cycle déjà armé (≤5 s, liste vide à ce moment), puis on réinitialise —
/// plus aucun sondage pendant le test, aucun seed n'est jamais lancé.
#[test]
fn reset_state_tout_reinjecte_seeds() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();

    // 1. Poller endormi pour 1 h (clamp max = 3600).
    let rep = eng.action(&json!({"action": "set_settings", "refresh_secs": 3600}));
    assert!(ok_de(&rep), "set_settings refusé : {rep}");

    // 2. Le cycle de poll DÉJÀ ARMÉ (sommeil de 5 s posé au boot, liste encore
    //    vide) doit s'écouler AVANT la réinjection : attente bornée à 8 s.
    std::thread::sleep(Duration::from_secs(8));

    // 3. Une machine SSH à oublier (jamais contactée : cf. A10).
    let rep = eng.action(&json!({"action": "ssh_save", "ip": "203.0.113.78", "user": "e2e-user"}));
    assert!(ok_de(&rep), "{rep}");

    // 4. reset_state "tout".
    let rep = eng.action(&json!({"action": "reset_state", "what": "tout"}));
    assert!(ok_de(&rep), "reset_state tout refusé : {rep}");
    assert_eq!(
        rep.get("cleared").and_then(|c| c.as_array()),
        Some(&vec![
            json!("placement"),
            json!("detection"),
            json!("oublis")
        ]),
        "{rep}"
    );
    assert_eq!(
        rep.get("reinjected").and_then(|r| r.as_u64()),
        Some(SEED_IDS.len() as u64),
        "les 8 seeds retirés durablement doivent être réinjectés : {rep}"
    );

    // 5. /api/state : les 8 seeds sont de retour, origin "seed", sans machine
    //    SSH.
    let serveurs = serveurs_etat(&eng);
    for id in SEED_IDS {
        let s = serveurs
            .iter()
            .find(|s| champ(s, "id") == *id)
            .unwrap_or_else(|| panic!("seed {id} non réinjecté : {serveurs:?}"));
        assert_eq!(champ(s, "origin"), "seed", "{s}");
    }
    assert!(
        reglages_etat(&eng)
            .get("ssh_hosts")
            .and_then(|h| h.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(false),
        "les machines SSH doivent être oubliées par what=tout"
    );

    // 6. config.json : removed_seeds vidé, ssh_hosts vidé, seeds persistés.
    let cfg = config_du_moteur(&eng);
    let reg_cfg = cfg.get("settings").cloned().unwrap_or(Value::Null);
    assert!(
        reg_cfg
            .get("removed_seeds")
            .and_then(|r| r.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(false),
        "removed_seeds doit être vidé : {cfg}"
    );
    assert!(
        reg_cfg
            .get("ssh_hosts")
            .and_then(|h| h.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(false),
        "ssh_hosts doit être vidé dans config.json : {cfg}"
    );
    assert_eq!(
        serveurs_config(&eng).len(),
        SEED_IDS.len(),
        "les 8 seeds doivent être persistés : {:?}",
        config_du_moteur(&eng)
    );
}

/// Scénario A14 — check_update (bouton « Vérifier » du dialog Réglages) :
/// réponse TOUJOURS propre — ok:true, current renseigné, latest null (hors
/// ligne) ou tag — bornée : curl -m 10 + repli PowerShell 10 s ⇒ < 30 s,
/// jamais de suspension indéfinie (client local à 35 s de timeout lecture).
#[test]
fn check_update_borne_et_propre() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();

    let t0 = Instant::now();
    let (status, rep) = post_json_long(&eng, &json!({"action": "check_update"}), DELAI_REPONSE);
    let duree = t0.elapsed();
    assert_eq!(
        status, 200,
        "check_update doit répondre 200 (obtenu {status})"
    );
    assert!(ok_de(&rep), "check_update : {rep}");
    assert!(
        !champ(&rep, "current").is_empty(),
        "current doit contenir la version installée : {rep}"
    );
    let latest = rep.get("latest").cloned().unwrap_or(Value::Null);
    assert!(
        latest.is_null() || latest.is_string(),
        "latest doit être null ou un tag : {rep}"
    );
    assert_eq!(
        rep.get("update_available").and_then(|u| u.as_bool()),
        Some(latest.is_string()),
        "update_available doit refléter latest : {rep}"
    );
    assert!(
        duree < Duration::from_secs(30),
        "check_update doit rester borné (~20 s pire cas hors ligne) : {duree:?}"
    );
}

/// Scénario A15 — open_path (« Ouvrir dans l'explorateur ») : validation des
/// chemins SANS lancer explorer.exe (une ouverture réelle ouvrirait une
/// fenêtre sur le bureau de l'opérateur — effet de bord visible interdit).
/// Chimin vide ou à caractère de contrôle → "chemin invalide" ; chemin
/// inexistant → "chemin introuvable".
#[test]
fn open_path_validation_sans_explorer() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();

    let rep = eng.action(&json!({"action": "open_path", "path": ""}));
    assert!(!ok_de(&rep), "{rep}");
    assert_eq!(
        rep.get("error").and_then(|e| e.as_str()),
        Some("chemin invalide"),
        "{rep}"
    );

    // Caractère de contrôle → invalide avant même l'existence.
    let rep = eng.action(&json!({"action": "open_path", "path": "a\u{0000}b"}));
    assert!(!ok_de(&rep), "{rep}");
    assert_eq!(
        rep.get("error").and_then(|e| e.as_str()),
        Some("chemin invalide"),
        "{rep}"
    );

    // Chemin inexistant (dans le tmp du moteur, même machine) → introuvable.
    let inexistant = eng.tmp_dir().join("e2e-nexiste-pas.txt");
    let rep = eng.action(&json!({"action": "open_path", "path": inexistant.display().to_string()}));
    assert!(!ok_de(&rep), "{rep}");
    assert_eq!(
        rep.get("error").and_then(|e| e.as_str()),
        Some("chemin introuvable"),
        "{rep}"
    );
}

/// Scénario A16 — boutons Start ▶ / Stop ■ : sur un serveur HTTP local éteint
/// porteur d'une commande de lancement (programme INEXISTANT : tout spawn
/// échoue à sec, aucun process réel), start répond ok:true et réactive
/// l'entrée, kill répond ok:true et force DOWN sans pid.
#[test]
fn start_kill_boutons_sur_serveur_eteint() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();

    let rep = eng.action(&json!({
        "action": "add", "transport": "http",
        "port": free_port(), "path": "/mcp", "label": "e2e-startkill",
        "program": "mcc-e2e-programme-inexistant.exe", "args": []
    }));
    assert!(ok_de(&rep), "add refusé : {rep}");
    let id = rep
        .get("id")
        .and_then(|i| i.as_str())
        .unwrap_or("")
        .to_string();

    // Start ▶ : ok:true, l'entrée reste/rp devient activée avec can_start.
    let rep = eng.action(&json!({"action": "start", "id": id}));
    assert!(ok_de(&rep), "start refusé : {rep}");
    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| champ(s, "id") == id)
        .expect("entrée disparue");
    assert_eq!(
        s.get("enabled").and_then(|e| e.as_bool()),
        Some(true),
        "{s}"
    );
    assert_eq!(
        s.get("can_start").and_then(|c| c.as_bool()),
        Some(true),
        "{s}"
    );

    // Stop ■ : ok:true, état forcé DOWN, pid retiré.
    let rep = eng.action(&json!({"action": "kill", "id": id}));
    assert!(ok_de(&rep), "kill refusé : {rep}");
    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| champ(s, "id") == id)
        .expect("entrée disparue");
    assert_eq!(champ(&s, "state"), "DOWN", "kill doit forcer DOWN : {s}");
    assert_eq!(
        s.get("pid").and_then(|p| p.as_u64()),
        None,
        "kill doit retirer le pid : {s}"
    );
}
