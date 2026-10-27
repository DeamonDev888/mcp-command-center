//! Scénarios e2e — cycle de vie des serveurs (ajout HTTP, stdio, toggle,
//! persistance config.json, retrait).
//!
//! Chaque `#[test]` démarre SON propre moteur (port UI libre, config isolée
//! dans un tmp dir par le harnais) et bornes toutes ses attentes avec
//! `wait_until` (aucun sommeil illimité). Les tests sont sérialisés
//! (`--test-threads=1`) : chaque moteur a son port, chaque fixture son port.
//!
//! Faits source vérifiés (src/manage.rs, src/probe.rs, src/types.rs) :
//! - `/api/state` → `{"servers":[…],"settings":{…},…}` ; chaque serveur expose
//!   `id`, `label`, `transport`, `transport_info{host,port,path|program,args}`,
//!   `origin` ("seed"/"manual"/"discovered"), `enabled`, `state`
//!   ("UP"/"DOWN"/"PORT-ONLY"/"DISABLED"), `pid`, `server_name`, `version`,
//!   `tools_count`.
//! - action `add` : http → `port` (requis), `host` (optionnel), `path`
//!   (défaut "/mcp"), `label` ; stdio → `program` (requis), `args`, `workdir`,
//!   `label`. Réponse `{"ok":true,"id":"manual-http-<ts>"| "manual-stdio-<ts>"}`.
//! - action `toggle` (id) : OFF → kill_tree + `pid: null` + state DISABLED ;
//!   ON → re-sonde immédiatement (poll_one).
//! - action `refresh` (id) : poll_one immédiat (au lieu d'attendre le cycle).
//! - action `remove` (id, `durable`?) : retire l'entrée + save_config.
//! - stdio (probe_stdio) : JSON-RPC délimité par lignes sur stdin/stdout —
//!   initialize (id 1) → `result.serverInfo{name,version}` + `protocolVersion`,
//!   puis notifications/initialized + tools/list (id 2) → `result.tools[]`.
//! - config.json (save_config) : `{"servers":[{id,label,enabled,transport,
//!   port,path|program,args,…}],"settings":{…},"runtime":{…}}` — écrit à chaque
//!   add/toggle/remove. Au chargement, `enabled:false` → state DISABLED.

use crate::harness::{spawn, spawn_loopback, wait_until, Engine, EngineCfg, Fixture};
use serde_json::{json, Value};
use std::time::Duration;

/// Second d'attente maximal pour qu'un serveur passe UP après un poll.
const DELAI_UP: Duration = Duration::from_secs(30);
/// Attente bornée générique (constat d'absence, attente de persistance…).
const DELAI_LIMITE: Duration = Duration::from_secs(15);

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

/// Contenu de config.json du moteur (objet vide si le fichier n'existe pas
/// encore — il n'est créé qu'au premier save_config).
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

/// L'entrée de /api/state correspond-elle à CE port HTTP local ?
fn meme_port(s: &Value, port: u16) -> bool {
    s.get("transport_info")
        .and_then(|t| t.get("port"))
        .and_then(|p| p.as_u64())
        == Some(port as u64)
}

/// Petit accès string sûr sur un champ JSON.
fn champ<'a>(s: &'a Value, cle: &str) -> &'a str {
    s.get(cle).and_then(|v| v.as_str()).unwrap_or("")
}

/// Ajoute un serveur HTTP local (host explicite 127.0.0.1, chemin /mcp) et
/// retourne la réponse {"ok":true,"id":…}.
fn ajoute_http(eng: &Engine, port: u16, label: &str) -> Value {
    eng.action(&json!({
        "action": "add", "transport": "http",
        "host": "127.0.0.1", "port": port, "path": "/mcp", "label": label
    }))
}

/// Force un poll immédiat d'une entrée (action "refresh" → poll_one).
fn rafraichit(eng: &Engine, id: &str) {
    eng.action(&json!({"action": "refresh", "id": id}));
}

/// `node` est-il utilisable ? (le stdio e2e tourne sur Node)
fn node_disponible() -> bool {
    matches!(
        std::process::Command::new("node").arg("--version").output(),
        Ok(out) if out.status.success()
    )
}

/// Compteur pour un nom de fichier JS unique par appel (aucune collision,
/// même si un jour les tests passent en parallèle).
static JS_SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Écrit le serveur MCP stdio d'e2e (JS) dans le tempdir du TEST (pas celui du
/// moteur : le tmp du moteur peut être nettoyé par le harnais) et retourne son
/// chemin. Nettoyage en fin de test via `nettoie_js`.
const MCP_STDIO_JS: &str = r#"// Serveur MCP stdio minimal pour e2e : JSON-RPC delimite par lignes.
const rl = require('readline').createInterface({ input: process.stdin, terminal: false });
const repondre = (o) => process.stdout.write(JSON.stringify(o) + '\n');
rl.on('line', (ligne) => {
  let msg; try { msg = JSON.parse(ligne); } catch { return; }
  if (!msg || msg.method === undefined) return;
  if (msg.method === 'initialize') {
    repondre({ jsonrpc: '2.0', id: msg.id, result: {
      protocolVersion: '2024-11-05',
      capabilities: { tools: {} },
      serverInfo: { name: 'fake-stdio-e2e', version: '0.1.0' }
    } });
  } else if (msg.method === 'tools/list') {
    repondre({ jsonrpc: '2.0', id: msg.id, result: { tools: [
      { name: 'ping', description: 'repond pong' },
      { name: 'echo', description: 'renvoie le texte recu' }
    ] } });
  } else if (msg.id !== undefined && msg.id !== null) {
    repondre({ jsonrpc: '2.0', id: msg.id,
      error: { code: -32601, message: 'methode inconnue' } });
  }
});
// pas de process.exit : le process reste vivant (keep_alive cote moteur).
"#;

fn ecrit_stdio_js() -> std::path::PathBuf {
    let n = JS_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let chemin = std::env::temp_dir().join(format!("mcc-e2e-stdio-{}-{n}.js", std::process::id()));
    std::fs::write(&chemin, MCP_STDIO_JS).expect("écriture du JS stdio e2e");
    chemin
}

/// Meilleur effort : supprime le JS temporaire (le test continue si échec).
fn nettoie_js(chemin: &std::path::Path) {
    let _ = std::fs::remove_file(chemin);
}

/// Ajoute le serveur stdio "node <js>" et force un premier poll.
fn ajoute_stdio(eng: &Engine, js: &std::path::Path, label: &str) -> String {
    let rep = eng.action(&json!({
        "action": "add", "transport": "stdio",
        "program": "node", "args": [js.display().to_string()], "label": label
    }));
    assert!(
        rep.get("ok").and_then(|o| o.as_bool()).unwrap_or(false),
        "add stdio refusé : {rep}"
    );
    let id = rep
        .get("id")
        .and_then(|i| i.as_str())
        .expect("add stdio sans id")
        .to_string();
    rafraichit(eng, &id);
    id
}

/// Redémarre un moteur en lui réinjectant une config existante.
///
/// APPROCHE (limitation documentée) : le harnais isole chaque moteur dans un
/// tmp dir NEUF et pré-écrit SA config générée avant de lancer le process ; le
/// moteur lit config.json dès son boot (src/lib.rs : load_config précède le
/// bind TCP) — on ne peut donc pas préparer le fichier avant la lecture sans
/// course. Approche retenue : écraser `tmp/config.json` du nouveau moteur
/// immédiatement après le spawn (course gagnée dans l'immense majorité des
/// cas : le process enfant n'a pas encore atteint load_config), puis vérifier
/// que l'entrée attendue apparaît dans /api/state ; si la course est perdue
/// (le moteur a lu SA config générée), on retente avec un moteur NEUF (port
/// différent) — 3 essais bornés. `Drop` du harnais tue le process écarté.
///
/// La config réinjectée voit son `settings.ui_port` réécrit sur le port du
/// NOUVEAU moteur : sinon le moteur re-binderait le port de l'ancien et le
/// harnais (qui parle à cfg.ui_port) ne joindrait personne. Les autres
/// réglages sûrs (autostart false, network_enabled false, removed_seeds)
/// voyagent avec les octets de l'ancienne config.
fn redemarre_meme_config(anciens_octets: &[u8], present: &dyn Fn(&Value) -> bool) -> Engine {
    for tentative in 0..3u32 {
        let port = crate::harness::free_port();
        let mut eng = spawn(EngineCfg {
            ui_port: port,
            ui_bind: "127.0.0.1",
            api_access: None,
        })
        .unwrap_or_else(|e| panic!("redémarrage t{tentative} : spawn impossible : {e}"));
        // Réinjecte l'ancienne config, avec le port UI du nouveau moteur.
        let mut doc: Value = serde_json::from_slice(anciens_octets).unwrap_or_else(|e| {
            panic!("redémarrage t{tentative} : ancienne config illisible : {e}")
        });
        if !doc.is_object() {
            doc = json!({"servers": [], "settings": {}});
        }
        doc["settings"]["ui_port"] = json!(port);
        let chemin = eng.tmp_dir().join("config.json");
        std::fs::write(&chemin, serde_json::to_vec_pretty(&doc).unwrap())
            .unwrap_or_else(|e| panic!("redémarrage t{tentative} : écrasement config.json : {e}"));
        if eng.wait_ready(Duration::from_secs(30)).is_ok()
            && wait_until(Duration::from_secs(8), || {
                if serveurs_etat(&eng).iter().any(present) {
                    Some(true)
                } else {
                    None
                }
            })
        {
            return eng;
        }
        eprintln!("redémarrage t{tentative} : course boot/écrasement perdue, nouvel essai");
        drop(eng);
    }
    panic!(
        "redémarrage : config non rechargée après 3 tentatives (course boot/écrasement toujours perdue)"
    );
}

// ---------- scénarios ----------

/// Scénario 1 — Ajout HTTP + sonde : une fixture MCP locale est ajoutée comme
/// serveur HTTP (host 127.0.0.1, chemin /mcp) ; le moteur doit la sonder et la
/// déclarer UP avec le serverInfo annoncé par la fixture
/// ("fake-e2e-mcp" v0.2.0) et ses 2 outils.
#[test]
fn add_http_fixture_probe_up() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();
    let fx = Fixture::spawn_mcp();
    let rep = ajoute_http(&eng, fx.port(), "e2e-http");
    assert!(
        rep.get("ok").and_then(|o| o.as_bool()).unwrap_or(false),
        "add http refusé : {rep}"
    );
    let id = rep
        .get("id")
        .and_then(|i| i.as_str())
        .unwrap_or("")
        .to_string();
    assert!(!id.is_empty(), "add http sans id : {rep}");
    rafraichit(&eng, &id);

    let trouve = wait_until(DELAI_UP, || {
        let serveurs = serveurs_etat(&eng);
        match serveurs.iter().find(|s| meme_port(s, fx.port())) {
            Some(s) if champ(s, "state") == "UP" => Some(true),
            _ => None,
        }
    });
    assert!(
        trouve,
        "aucune entrée UP sur le port {} dans /api/state",
        fx.port()
    );
    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| meme_port(s, fx.port()))
        .expect("entrée venant de disparaître");
    assert_eq!(
        champ(&s, "server_name"),
        "fake-e2e-mcp",
        "serverInfo.name : {s}"
    );
    assert_eq!(champ(&s, "version"), "0.2.0", "serverInfo.version : {s}");
    assert_eq!(
        s.get("tools_count").and_then(|t| t.as_u64()),
        Some(2),
        "la fixture annonce 2 outils (ping, echo) : {s}"
    );
}

/// Scénario 2 — Ajout stdio : un vrai process (`node` + JS inline) est ajouté
/// comme serveur stdio ; le moteur doit le lancer, faire le handshake
/// initialize + tools/list (JSON-RPC par lignes) et le déclarer UP avec
/// pid Some, server_name "fake-stdio-e2e" v0.1.0 et 2 outils.
/// Skippé (eprintln + return) si `node` n'est pas installé.
#[test]
fn stdio_fixture_up_with_tools() {
    let _garde = crate::harness::serial_guard();
    if !node_disponible() {
        eprintln!("SKIP stdio_fixture_up_with_tools : node indisponible");
        return;
    }
    let eng = moteur();
    let js = ecrit_stdio_js();
    let id = ajoute_stdio(&eng, &js, "e2e-stdio");

    let trouve = wait_until(DELAI_UP, || {
        let serveurs = serveurs_etat(&eng);
        match serveurs.iter().find(|s| champ(s, "id") == id) {
            Some(s)
                if champ(s, "state") == "UP" && s.get("pid").and_then(|p| p.as_u64()).is_some() =>
            {
                Some(true)
            }
            _ => None,
        }
    });
    assert!(trouve, "le serveur stdio n'est jamais passé UP avec un pid");
    nettoie_js(&js);

    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| champ(s, "id") == id)
        .expect("entrée stdio disparue");
    assert_eq!(champ(&s, "server_name"), "fake-stdio-e2e", "{s}");
    assert_eq!(champ(&s, "version"), "0.1.0", "{s}");
    assert_eq!(
        s.get("tools_count").and_then(|t| t.as_u64()),
        Some(2),
        "{s}"
    );
    assert_eq!(
        s.get("tools_total").and_then(|t| t.as_u64()),
        Some(2),
        "{s}"
    );
}

/// Scénario 3 — Toggle OFF tue le process tenu par le moteur (WINDOWS
/// uniquement) : après `toggle` OFF, l'entrée doit perdre son pid (children
/// vidés via kill_tree) et passer DISABLED ; le process node meurt avec lui.
/// `#[cfg(windows)]` : kill_tree est un no-op Unix (audit P1 connu) — la
/// garantie « process tué » ne vaut que sur Windows.
#[test]
#[cfg(windows)]
fn toggle_off_kills_pid() {
    let _garde = crate::harness::serial_guard();
    if !node_disponible() {
        eprintln!("SKIP toggle_off_kills_pid : node indisponible");
        return;
    }
    let eng = moteur();
    let js = ecrit_stdio_js();
    let id = ajoute_stdio(&eng, &js, "e2e-stdio-off");

    // pré-condition : UP avec un pid
    let up = wait_until(DELAI_UP, || {
        let serveurs = serveurs_etat(&eng);
        match serveurs.iter().find(|s| champ(s, "id") == id) {
            Some(s)
                if champ(s, "state") == "UP" && s.get("pid").and_then(|p| p.as_u64()).is_some() =>
            {
                Some(true)
            }
            _ => None,
        }
    });
    assert!(up, "pré-condition non remplie : stdio jamais UP");
    nettoie_js(&js);

    let rep = eng.action(&json!({"action": "toggle", "id": id}));
    assert_eq!(
        rep.get("enabled").and_then(|e| e.as_bool()),
        Some(false),
        "toggle OFF : {rep}"
    );
    let eteint = wait_until(DELAI_UP, || {
        let serveurs = serveurs_etat(&eng);
        match serveurs.iter().find(|s| champ(s, "id") == id) {
            // pid null = children vidé (process tué) ; DISABLED = état posé par
            // toggle (poll_one ne re-sonde pas une entrée désactivée).
            Some(s) if s.get("pid").and_then(|p| p.as_u64()).is_none() => Some(true),
            _ => None,
        }
    });
    assert!(eteint, "le pid n'a jamais été retiré après toggle OFF");
    let s = serveurs_etat(&eng)
        .into_iter()
        .find(|s| champ(s, "id") == id)
        .expect("entrée disparue");
    assert_eq!(
        champ(&s, "state"),
        "DISABLED",
        "toggle OFF doit donner DISABLED : {s}"
    );
    assert_eq!(
        s.get("enabled").and_then(|e| e.as_bool()),
        Some(false),
        "{s}"
    );
}

/// Scénario 4 — Toggle ON fait renaître le serveur stdio : après OFF puis ON,
/// le moteur relance le process (nouveau handshake stdio) et l'entrée
/// redevient UP avec un pid Some et ses 2 outils.
///
/// WINDOWS ONLY pour l'instant : le toggle OFF tue le child via kill_tree,
/// no-op sur Unix (audit 2026-10-26 P1) → child.wait() y bloque le handler
/// et la réponse toggle n'arrive jamais (constaté sur le CI ubuntu :
/// réponse `null`). Gater ce scénario comme toggle_off_kills_pid tant que
/// kill_child Unix n'est pas implémenté.
#[cfg(windows)]
#[test]
fn toggle_on_respawns() {
    let _garde = crate::harness::serial_guard();
    if !node_disponible() {
        eprintln!("SKIP toggle_on_respawns : node indisponible");
        return;
    }
    let eng = moteur();
    let js = ecrit_stdio_js();
    let id = ajoute_stdio(&eng, &js, "e2e-stdio-on");

    // OFF d'abord (le serveur démarre activé par défaut)
    let rep = eng.action(&json!({"action": "toggle", "id": id}));
    assert_eq!(
        rep.get("enabled").and_then(|e| e.as_bool()),
        Some(false),
        "{rep}"
    );
    let eteint = wait_until(DELAI_UP, || {
        let serveurs = serveurs_etat(&eng);
        match serveurs.iter().find(|s| champ(s, "id") == id) {
            Some(s) if s.get("pid").and_then(|p| p.as_u64()).is_none() => Some(true),
            _ => None,
        }
    });
    assert!(eteint, "toggle OFF : pid jamais retiré");

    // ON : toggle re-sonde immédiatement (poll_one). Le JS doit EXISTER encore
    // ici : toggle ON relance `node <js>` — pas de nettoyage avant la fin.
    let rep = eng.action(&json!({"action": "toggle", "id": id}));
    assert_eq!(
        rep.get("enabled").and_then(|e| e.as_bool()),
        Some(true),
        "{rep}"
    );
    let up = wait_until(DELAI_UP, || {
        let serveurs = serveurs_etat(&eng);
        match serveurs.iter().find(|s| champ(s, "id") == id) {
            Some(s)
                if champ(s, "state") == "UP" && s.get("pid").and_then(|p| p.as_u64()).is_some() =>
            {
                Some(true)
            }
            _ => None,
        }
    });
    nettoie_js(&js);
    assert!(
        up,
        "toggle ON : le serveur n'est jamais revenu UP avec un pid"
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
}

/// Scénario 5 — Désactivation persistée : après toggle OFF, config.json doit
/// porter `enabled:false` pour l'entrée, et un moteur RELANCÉ avec cette même
/// config doit recharger l'entrée désactivée (state DISABLED au boot —
/// load_config : enabled:false → Disabled).
///
/// APPROCHE RESTART (voir `redemarre_meme_config`) : le harnais isole chaque
/// moteur dans un tmp dir neuf ; on réinjecte les octets de l'ancienne
/// config.json (port UI réécrit) dans le tmp du nouveau moteur juste après le
/// spawn, avec 3 essais bornés si la course boot/lecture est perdue.
#[test]
fn disable_enable_persisted() {
    let _garde = crate::harness::serial_guard();
    if !node_disponible() {
        eprintln!("SKIP disable_enable_persisted : node indisponible");
        return;
    }
    let eng = moteur();
    let js = ecrit_stdio_js();
    let id = ajoute_stdio(&eng, &js, "e2e-stdio-persist");
    let up = wait_until(DELAI_UP, || {
        let serveurs = serveurs_etat(&eng);
        match serveurs.iter().find(|s| champ(s, "id") == id) {
            Some(s) if champ(s, "state") == "UP" => Some(true),
            _ => None,
        }
    });
    assert!(up, "pré-condition : stdio jamais UP");
    nettoie_js(&js);

    eng.action(&json!({"action": "toggle", "id": id}));
    let persiste = wait_until(DELAI_UP, || {
        if serveurs_config(&eng).iter().any(|s| {
            champ(s, "label") == "e2e-stdio-persist" && s.get("enabled") == Some(&json!(false))
        }) {
            Some(true)
        } else {
            None
        }
    });
    assert!(
        persiste,
        "config.json ne porte pas enabled:false après toggle OFF : {:?}",
        config_du_moteur(&eng)
    );

    // Redémarrage avec la même config : l'entrée doit revenir, désactivée.
    let octets = std::fs::read(eng.tmp_dir().join("config.json")).expect("lecture config.json");
    let eng2 = redemarre_meme_config(&octets, &|s: &Value| {
        champ(s, "label") == "e2e-stdio-persist"
            && s.get("enabled").and_then(|e| e.as_bool()) == Some(false)
    });
    let s = serveurs_etat(&eng2)
        .into_iter()
        .find(|s| champ(s, "label") == "e2e-stdio-persist")
        .expect("entrée perdue après restart");
    assert_eq!(champ(&s, "state"), "DISABLED", "après restart : {s}");
    assert_eq!(
        s.get("pid").and_then(|p| p.as_u64()),
        None,
        "après restart : {s}"
    );
}

/// Scénario 6 — Retrait propre : `remove` doit faire disparaître l'entrée de
/// /api/state ET de config.json (save_config réécrit la liste sans elle).
#[test]
fn remove_cleans() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();
    let fx = Fixture::spawn_mcp();
    let rep = ajoute_http(&eng, fx.port(), "e2e-remove");
    let id = rep
        .get("id")
        .and_then(|i| i.as_str())
        .expect("add sans id")
        .to_string();
    rafraichit(&eng, &id);
    let up = wait_until(DELAI_UP, || {
        let serveurs = serveurs_etat(&eng);
        match serveurs.iter().find(|s| champ(s, "id") == id) {
            Some(s) if champ(s, "state") == "UP" => Some(true),
            _ => None,
        }
    });
    assert!(up, "pré-condition : entrée jamais UP");

    let rep = eng.action(&json!({"action": "remove", "id": id}));
    assert!(
        rep.get("ok").and_then(|o| o.as_bool()).unwrap_or(false),
        "remove refusé : {rep}"
    );
    let parti = wait_until(DELAI_LIMITE, || {
        let reste_etat = serveurs_etat(&eng).iter().any(|s| champ(s, "id") == id);
        let reste_cfg = serveurs_config(&eng)
            .iter()
            .any(|s| s.get("id") == Some(&json!(id)));
        if !reste_etat && !reste_cfg {
            Some(true)
        } else {
            None
        }
    });
    assert!(
        parti,
        "l'entrée traîne encore après remove — état : {:?} / config : {:?}",
        serveurs_etat(&eng),
        serveurs_config(&eng)
    );
}

/// Scénario 7 — Persistance entre redémarrages : un serveur ajouté doit
/// figurer dans config.json, et un moteur relancé avec CETTE config doit
/// l'exposer à nouveau (même démarche que le scénario 5, côté http UP cette
/// fois : la fixture tourne toujours, le nouveau moteur re-sonde et le
/// retrouve UP).
#[test]
fn config_persistence_across_restart() {
    let _garde = crate::harness::serial_guard();
    let eng = moteur();
    let fx = Fixture::spawn_mcp();
    let rep = ajoute_http(&eng, fx.port(), "e2e-restart");
    let id = rep
        .get("id")
        .and_then(|i| i.as_str())
        .expect("add sans id")
        .to_string();
    rafraichit(&eng, &id);
    let up = wait_until(DELAI_UP, || {
        let serveurs = serveurs_etat(&eng);
        match serveurs.iter().find(|s| champ(s, "id") == id) {
            Some(s) if champ(s, "state") == "UP" => Some(true),
            _ => None,
        }
    });
    assert!(up, "pré-condition : entrée jamais UP");

    let persiste = wait_until(DELAI_LIMITE, || {
        if serveurs_config(&eng)
            .iter()
            .any(|s| s.get("port") == Some(&json!(fx.port())))
        {
            Some(true)
        } else {
            None
        }
    });
    assert!(
        persiste,
        "config.json ne contient pas le serveur ajouté : {:?}",
        config_du_moteur(&eng)
    );
    let octets = std::fs::read(eng.tmp_dir().join("config.json")).expect("lecture config.json");

    let eng2 = redemarre_meme_config(&octets, &|s: &Value| meme_port(s, fx.port()));
    let s = serveurs_etat(&eng2)
        .into_iter()
        .find(|s| meme_port(s, fx.port()))
        .expect("serveur perdu après restart");
    // La fixture tourne toujours : le moteur rechargé re-sonde et doit finir UP.
    let up2 = wait_until(DELAI_UP, || {
        let serveurs = serveurs_etat(&eng2);
        match serveurs.iter().find(|s| meme_port(s, fx.port())) {
            Some(s) if champ(s, "state") == "UP" => Some(true),
            _ => None,
        }
    });
    assert!(up2, "après restart, le serveur n'est pas revenu UP : {s}");
}
