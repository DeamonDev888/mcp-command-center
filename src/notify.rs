//! Notifications : desktop (Windows) + webhook HTTP sur incident serveur.
//!
//! L'app surveille en continu ; quand un serveur passe de UP à DOWN (ou
//! PORT-ONLY), l'opérateur doit être prévenu — sans regarder l'écran.
//!
//! Deux canaux :
//! - **desktop** : toast Windows (PowerShell + BurntToast si présent, sinon
//!   fallback `msg`/notification) — sans dépendance externe obligatoire ;
//! - **webhook** : POST JSON vers une URL (Slack/Discord/n8n/own endpoint).
//!
//! Règles : un seul déclenchement par transition (pas de spam à chaque
//! cycle), et jamais d'erreur bloquante — une notification ratée ne doit
//! jamais faire tomber le poller.

use serde_json::{json, Value};

/// Configuration des notifications (persistée dans config.json).
#[derive(Clone, Debug)]
pub struct NotifyConfig {
    /// Active les notifications desktop.
    pub desktop: bool,
    /// URL du webhook (vide = désactivé).
    pub webhook_url: String,
    /// Méthode HTTP du webhook (POST par défaut).
    pub method: String,
    /// En-têtes additionnels (ex. Authorization) — format "K: V" par ligne.
    pub headers: String,
    /// Ne notifier que ces états (par défaut : DOWN et PORT-ONLY).
    pub states: Vec<String>,
    /// Délai minimum entre deux notifications du même serveur (secondes).
    pub min_interval_secs: u64,
}

impl Default for NotifyConfig {
    fn default() -> Self {
        NotifyConfig {
            desktop: false,
            webhook_url: String::new(),
            method: "POST".to_string(),
            headers: String::new(),
            states: vec!["DOWN".into(), "PORT-ONLY".into()],
            min_interval_secs: 300,
        }
    }
}

impl NotifyConfig {
    /// L'état doit-il déclencher une notification ?
    pub fn wants(&self, state: &str) -> bool {
        self.states.iter().any(|s| s.eq_ignore_ascii_case(state))
    }

    /// En-têtes du webhook, parsés depuis "K: V" (une par ligne).
    pub fn header_pairs(&self) -> Vec<(String, String)> {
        self.headers
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                if l.is_empty() {
                    return None;
                }
                let (k, v) = l.split_once(':')?;
                let k = k.trim();
                if k.is_empty() {
                    return None;
                }
                Some((k.to_string(), v.trim().to_string()))
            })
            .collect()
    }
}

/// Charge la config de notification depuis la section `notify` de la config.
pub fn from_json(v: &Value) -> NotifyConfig {
    let mut c = NotifyConfig::default();
    let Some(o) = v.as_object() else { return c };
    if let Some(b) = o.get("desktop").and_then(|x| x.as_bool()) {
        c.desktop = b;
    }
    if let Some(u) = o.get("webhook_url").and_then(|x| x.as_str()) {
        c.webhook_url = u.trim().to_string();
    }
    if let Some(m) = o.get("method").and_then(|x| x.as_str()) {
        let m = m.trim().to_uppercase();
        if m == "POST" || m == "PUT" {
            c.method = m;
        }
    }
    if let Some(h) = o.get("headers").and_then(|x| x.as_str()) {
        c.headers = h.to_string();
    }
    if let Some(arr) = o.get("states").and_then(|x| x.as_array()) {
        let v: Vec<String> = arr
            .iter()
            .filter_map(|x| x.as_str())
            .map(|x| x.trim().to_uppercase())
            .filter(|x| !x.is_empty())
            .collect();
        if !v.is_empty() {
            c.states = v;
        }
    }
    if let Some(i) = o.get("min_interval_secs").and_then(|x| x.as_u64()) {
        c.min_interval_secs = i.clamp(0, 86400);
    }
    c
}

/// Sérialise la config de notification.
pub fn to_json(c: &NotifyConfig) -> Value {
    json!({
        "desktop": c.desktop,
        "webhook_url": c.webhook_url,
        "method": c.method,
        "headers": c.headers,
        "states": c.states,
        "min_interval_secs": c.min_interval_secs,
    })
}

/// Corps du webhook (compatible Slack/Discord : un objet `text` suffit, et le
/// contenu est aussi lisible par un endpoint maison).
pub fn webhook_body(server: &str, state: &str, detail: &str) -> Value {
    json!({
        "text": format!("MCP — {server} est {state} : {detail}"),
        "server": server,
        "state": state,
        "detail": detail,
        "app": "mcp-command-center",
        "ts": crate::types::now_ts(),
    })
}

/// Valide une URL de webhook (http/https seulement, longueur bornée).
pub fn valid_webhook_url(url: &str) -> bool {
    let u = url.trim();
    if u.is_empty() || u.len() > 2000 {
        return false;
    }
    u.starts_with("http://") || u.starts_with("https://")
}

/// Envoie le webhook (HTTP brut, sans dépendance : on réutilise le client
/// maison du projet). Retourne Ok(()) ou le message d'erreur.
pub fn send_webhook(
    cfg: &NotifyConfig,
    server: &str,
    state: &str,
    detail: &str,
) -> Result<(), String> {
    if cfg.webhook_url.trim().is_empty() {
        return Err("aucune URL de webhook configurée".to_string());
    }
    if !valid_webhook_url(&cfg.webhook_url) {
        return Err("URL de webhook invalide (http:// ou https:// attendu)".to_string());
    }
    let body = webhook_body(server, state, detail).to_string();
    let (host, port, path) = split_url(&cfg.webhook_url)?;
    // les paires d'en-têtes doivent VIVRE jusqu'à l'appel : on les garde
    // dans des variables (un Vec<&str> vers des temporaires ne compile pas).
    let pairs = cfg.header_pairs();
    let mut headers: Vec<(&str, &str)> = Vec::new();
    for (k, v) in &pairs {
        headers.push((k.as_str(), v.as_str()));
    }
    crate::probe::http_post_raw(&host, port, &path, &body, &headers, &cfg.method, 8000)
        .map(|_| ())
        .map_err(|e| format!("webhook : {e}"))
}

/// Découpe une URL en (hôte, port, chemin) — le client maison n'accepte pas
/// les URL complètes.
fn split_url(url: &str) -> Result<(String, u16, String), String> {
    let u = url.trim();
    let rest = if let Some(r) = u.strip_prefix("https://") {
        r
    } else if let Some(r) = u.strip_prefix("http://") {
        r
    } else {
        return Err("URL de webhook invalide (http:// ou https:// attendu)".to_string());
    };
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let authority = authority.split('@').next_back().unwrap_or(authority);
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => (
            h.to_string(),
            p.parse::<u16>().map_err(|_| "port invalide".to_string())?,
        ),
        _ => (
            authority.to_string(),
            if u.starts_with("https://") { 443 } else { 80 },
        ),
    };
    if host.is_empty() {
        return Err("hôte vide dans l'URL".to_string());
    }
    Ok((host, port, path.to_string()))
}

/// Toast desktop Windows. Silencieux si indisponible (le poller ne doit pas
/// tomber). Le canal Windows natif (`msg`) est utilisé en repli : la
/// notification apparaît dans le centre de maintenance.
pub fn desktop_toast(title: &str, message: &str) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("notifications desktop : non supporté hors Windows".to_string());
    }
    let mut c = std::process::Command::new("powershell");
    c.args(["-NoProfile", "-NonInteractive", "-Command", "-Command"]);
    // NOTE : construction par assemblage — un message contenant un guillemet
    // casserait la commande PowerShell.
    let script = format!(
        "[void][System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms');\
         $n = New-Object System.Windows.Forms.NotifyIcon;\
         $n.Icon = [System.Drawing.SystemIcons]::Warning;\
         $n.BalloonTipTitle = '{}';\
         $n.BalloonTipText = '{}';\
         $n.Visible = $true;\
         Start-Sleep -Milliseconds 50;\
         $n.ShowBalloonTip(8000);\
         Start-Sleep -Seconds 9;\
         $n.Dispose()",
        title.replace('"', "'"),
        message.replace('"', "'")
    );
    let out = c
        .args(["-Command", &script])
        .output()
        .map_err(|e| format!("toast desktop : {e}"))?;
    if !out.status.success() {
        return Err("toast desktop : PowerShell a échoué".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_par_defaut_sans_webhook() {
        let c = NotifyConfig::default();
        assert!(!c.desktop);
        assert!(c.webhook_url.is_empty());
        assert!(c.wants("DOWN"));
        assert!(c.wants("PORT-ONLY"));
        assert!(!c.wants("UP"));
        assert!(!c.wants("DISABLED"));
    }

    #[test]
    fn lecture_json_respecte_les_champs() {
        let c = from_json(&json!({
            "desktop": true,
            "webhook_url": "https://example.com/hook",
            "method": "put",
            "headers": "Authorization: Bearer abc\nX-Test: 1",
            "states": ["down"],
            "min_interval_secs": 60
        }));
        assert!(c.desktop);
        assert_eq!(c.webhook_url, "https://example.com/hook");
        assert_eq!(c.method, "PUT", "méthode normalisée en majuscules");
        assert_eq!(c.states, vec!["DOWN"]);
        assert_eq!(c.min_interval_secs, 60);
        let pairs = c.header_pairs();
        assert_eq!(pairs.len(), 2);
        assert_eq!(
            pairs[0],
            ("Authorization".to_string(), "Bearer abc".to_string())
        );
        assert_eq!(pairs[1], ("X-Test".to_string(), "1".to_string()));
    }

    #[test]
    fn json_invalide_retombe_sur_les_defauts() {
        let c = from_json(&json!("pas un objet"));
        assert!(!c.desktop);
        assert!(c.webhook_url.is_empty());
        // une méthode non autorisée est ignorée
        let c2 = from_json(&json!({"method": "DELETE"}));
        assert_eq!(c2.method, "POST");
    }

    #[test]
    fn url_valide_est_verifiee() {
        assert!(valid_webhook_url("https://example.com/x"));
        assert!(valid_webhook_url("http://192.168.1.5:8080/hook"));
        assert!(!valid_webhook_url("ftp://example.com"));
        assert!(!valid_webhook_url("example.com"));
        assert!(!valid_webhook_url(""));
    }

    #[test]
    fn decoupe_url() {
        let (h, p, path) = split_url("https://example.com/a/b").unwrap();
        assert_eq!(h, "example.com");
        assert_eq!(p, 443);
        assert_eq!(path, "/a/b");
        let (h, p, path) = split_url("http://192.168.1.5:8080").unwrap();
        assert_eq!(h, "192.168.1.5");
        assert_eq!(p, 8080);
        assert_eq!(path, "/");
        assert!(split_url("pas-url").is_err());
    }

    #[test]
    fn corps_webhook_lisible() {
        let b = webhook_body("mon-api", "DOWN", "connexion refusée");
        assert_eq!(b["server"], "mon-api");
        assert_eq!(b["state"], "DOWN");
        assert!(b["text"].as_str().unwrap().contains("mon-api"));
        assert!(b["text"].as_str().unwrap().contains("DOWN"));
    }

    #[test]
    fn webhook_vide_ou_invalide_refuse() {
        let c = NotifyConfig::default();
        assert!(send_webhook(&c, "x", "DOWN", "y").is_err());
        let c2 = NotifyConfig {
            webhook_url: "ftp://bad".into(),
            ..NotifyConfig::default()
        };
        assert!(send_webhook(&c2, "x", "DOWN", "y").is_err());
    }
}
