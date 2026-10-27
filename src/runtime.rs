//! Pilotage runtime : autoboot robuste, détection de crash, historique.
//!
//! Trois problèmes traités ici :
//! 1. **Autoboot fragile** — l'ancien compteur `spawn_fails` vivait en mémoire :
//!    au redémarrage de l'app il repartait de zéro, et 3 échecs consécutifs
//!    désactivaient définitivement l'auto-restart (un serveur lent à démarrer
//!    était abandonné à tort). Ici : backoff exponentiel borné, compteur
//!    persisté, et abandon seulement sur échec RÉPÉTÉ dans la fenêtre.
//! 2. **Crash invisible** — un process qui meurt (stdio ou HTTP) laissait une
//!    entrée muette : pas de trace de la cause ni du code de sortie.
//! 3. **Pas d'historique** — impossible de savoir « quand et pourquoi ce
//!    serveur a redémarré ? ».

use serde_json::{json, Value};
use std::collections::HashMap;

/// Tentatives de démarrage consécutives échouées (persisté).
pub const MAX_ATTEMPTS: u32 = 5;
/// Délai minimal entre deux tentatives (secondes) — backoff exponentiel.
pub const BASE_BACKOFF_SECS: u32 = 5;
/// Backoff maximal (secondes).
pub const MAX_BACKOFF_SECS: u32 = 300;

/// Une entrée d'historique : un événement runtime daté.
#[derive(Clone, Debug)]
pub struct RuntimeEvent {
    /// Horodatage (secondes epoch).
    pub ts: u64,
    /// Identifiant du serveur.
    pub id: String,
    /// Libellé lisible.
    pub label: String,
    /// Type d'événement : `crash`, `restart`, `start-fail`, `backoff`, `recovered`.
    pub kind: String,
    /// Détail libre (code de sortie, message d'erreur…).
    pub detail: String,
}

impl RuntimeEvent {
    /// Sérialise pour l'API `/api/runtime` et l'export.
    pub fn to_json(&self) -> Value {
        json!({
            "ts": self.ts,
            "id": self.id,
            "label": self.label,
            "kind": self.kind,
            "detail": self.detail,
        })
    }
}

/// État de pilotage d'un serveur : tentatives, backoff, historique.
#[derive(Clone, Debug, Default)]
pub struct RuntimeState {
    /// Nombre d'échecs de démarrage consécutifs.
    pub fails: u32,
    /// Timestamp du dernier essai (secondes) — sert de garde anti-boucle.
    pub last_attempt_ts: u64,
    /// Timestamp du dernier succès.
    pub last_success_ts: u64,
    /// Historique des événements (borné).
    pub events: Vec<RuntimeEvent>,
    /// Nombre total de redémarrages (crash + relances).
    pub restarts: u32,
    /// Nombre total de crashs détectés.
    pub crashes: u32,
}

impl RuntimeState {
    /// Backoff appliqué avant la prochaine tentative (secondes) : 5, 10, 20, 40…
    /// plafonné à [`MAX_BACKOFF_SECS`].
    pub fn backoff_secs(&self) -> u32 {
        let exp = BASE_BACKOFF_SECS.saturating_mul(1u32 << self.fails.saturating_sub(1).min(6));
        exp.min(MAX_BACKOFF_SECS)
    }

    /// Peut-on tenter un démarrage maintenant ? (garde anti-boucle + backoff)
    pub fn can_attempt(&self, now: u64) -> bool {
        if self.last_attempt_ts == 0 {
            return true;
        }
        now.saturating_sub(self.last_attempt_ts) >= self.backoff_secs() as u64
    }

    /// Enregistre un événement (historique borné à 100 entrées).
    pub fn push_event(&mut self, ev: RuntimeEvent) {
        self.events.push(ev);
        if self.events.len() > 100 {
            let excess = self.events.len() - 100;
            self.events.drain(0..excess);
        }
    }
}

/// Journal runtime de tous les serveurs (persisté dans config.json).
pub type RuntimeLog = HashMap<String, RuntimeState>;

/// Sérialise le journal runtime pour la config (une seule clé `runtime`).
pub fn to_json(log: &RuntimeLog) -> Value {
    let mut arr = Vec::new();
    let mut keys: Vec<&String> = log.keys().collect();
    keys.sort();
    for k in keys {
        let Some(s) = log.get(k) else { continue };
        arr.push(json!({
            "id": k,
            "fails": s.fails,
            "last_attempt_ts": s.last_attempt_ts,
            "last_success_ts": s.last_success_ts,
            "restarts": s.restarts,
            "crashes": s.crashes,
            "events": s.events.iter().map(|e| e.to_json()).collect::<Vec<_>>(),
        }));
    }
    json!(arr)
}

/// Recharge le journal runtime depuis la config (section `runtime`).
pub fn from_json(v: &Value) -> RuntimeLog {
    let mut out = RuntimeLog::new();
    let Some(arr) = v.as_array() else {
        return out;
    };
    for e in arr {
        let Some(id) = e.get("id").and_then(|x| x.as_str()) else {
            continue;
        };
        let mut events = Vec::new();
        if let Some(evs) = e.get("events").and_then(|x| x.as_array()) {
            for ev in evs {
                events.push(RuntimeEvent {
                    ts: ev.get("ts").and_then(|x| x.as_u64()).unwrap_or(0),
                    id: ev
                        .get("id")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    label: ev
                        .get("label")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    kind: ev
                        .get("kind")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    detail: ev
                        .get("detail")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }
        out.insert(
            id.to_string(),
            RuntimeState {
                fails: e.get("fails").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                last_attempt_ts: e
                    .get("last_attempt_ts")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0),
                last_success_ts: e
                    .get("last_success_ts")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0),
                events,
                restarts: e.get("restarts").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                crashes: e.get("crashes").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            },
        );
    }
    out
}

/// Décide si l'auto-restart doit être abandonné pour ce serveur.
///
/// On n'abandonne plus au bout de 3 échecs *consécutifs* quel que soit le
/// délai : on exige [`MAX_ATTEMPTS`] échecs et on laisse le backoff monté
/// (jusqu'à 5 min) — un serveur lent ou une dépendance absente ont ainsi le
/// temps de revenir. L'utilisateur peut toujours forcer un ▶ manuel.
pub fn should_disable_autoboot(fails: u32) -> bool {
    fails >= MAX_ATTEMPTS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(ts: u64, kind: &str) -> RuntimeEvent {
        RuntimeEvent {
            ts,
            id: "srv".into(),
            label: "srv".into(),
            kind: kind.into(),
            detail: String::new(),
        }
    }

    #[test]
    fn backoff_croit_et_est_plafonne() {
        let mut s = RuntimeState::default();
        assert_eq!(s.backoff_secs(), BASE_BACKOFF_SECS);
        s.fails = 1;
        assert_eq!(s.backoff_secs(), 5);
        s.fails = 2;
        assert_eq!(s.backoff_secs(), 10);
        s.fails = 3;
        assert_eq!(s.backoff_secs(), 20);
        s.fails = 4;
        assert_eq!(s.backoff_secs(), 40);
        s.fails = 10;
        assert_eq!(s.backoff_secs(), MAX_BACKOFF_SECS, "plafond respecté");
    }

    #[test]
    fn premiere_tentative_autorisee_puis_backoff() {
        let mut s = RuntimeState::default();
        assert!(s.can_attempt(1000), "première tentative OK");
        s.last_attempt_ts = 1000;
        s.fails = 1;
        assert!(!s.can_attempt(1002), "backoff de 5 s non respecté");
        assert!(s.can_attempt(1005), "après 5 s, nouvelle tentative OK");
        // backoff de 20 s (fails = 3) : interdit jusqu'à 1020, permis après
        s.fails = 3;
        s.last_attempt_ts = 1000;
        assert!(!s.can_attempt(1010), "backoff de 20 s respecté");
        assert!(s.can_attempt(1021));
    }

    #[test]
    fn abandon_seulement_apres_plusieurs_echecs() {
        assert!(!should_disable_autoboot(0));
        assert!(!should_disable_autoboot(3), "3 échecs ne suffisent plus");
        assert!(should_disable_autoboot(MAX_ATTEMPTS));
        assert!(should_disable_autoboot(MAX_ATTEMPTS + 3));
    }

    #[test]
    fn historique_borne_a_100_entrees() {
        let mut s = RuntimeState::default();
        for i in 0..150u64 {
            s.push_event(ev(i, "crash"));
        }
        assert_eq!(s.events.len(), 100, "historique borné");
        assert_eq!(s.events[0].ts, 50, "les plus anciens sont retirés");
        assert_eq!(s.events[99].ts, 149, "le plus récent est gardé");
    }

    #[test]
    fn round_trip_json_preserve_le_dernier_etat() {
        let mut log = RuntimeLog::new();
        let mut s = RuntimeState {
            fails: 2,
            restarts: 5,
            crashes: 3,
            last_success_ts: 1234,
            ..Default::default()
        };
        s.push_event(ev(999, "crash"));
        log.insert("srv-a".into(), s);
        let j = to_json(&log);
        let back = from_json(&j);
        let s2 = back.get("srv-a").expect("serveur rechargé");
        assert_eq!(s2.fails, 2);
        assert_eq!(s2.restarts, 5);
        assert_eq!(s2.crashes, 3);
        assert_eq!(s2.last_success_ts, 1234);
        assert_eq!(s2.events.len(), 1);
        assert_eq!(s2.events[0].kind, "crash");
    }

    #[test]
    fn json_invalide_ne_casse_pas_le_rechargement() {
        let log = from_json(&json!("pas un tableau"));
        assert!(log.is_empty());
        let log2 = from_json(&json!([{"id": "x"}]));
        assert!(
            log2.contains_key("x"),
            "entrée sans champs = valeurs par défaut"
        );
        assert_eq!(log2.get("x").unwrap().fails, 0);
    }
}
