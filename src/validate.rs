//! Validation MCP stricte et dé-duplication des serveurs découverts.
//!
//! Le scan réseau ouvre beaucoup de ports : la plupart ne parlent pas MCP.
//! Une validation « stricte » évite les faux positifs (une page HTML qui
//! contient un `{`, un serveur HTTP générique qui répond `{"result":{}}`,
//! une API REST…) et les faux négatifs (un serveur MCP légitime mais qui
//! répond sans `serverInfo`).

use serde_json::Value;

/// Résultat de la validation d'une réponse `initialize`.
#[derive(Clone, Debug, PartialEq)]
pub enum McpValid {
    /// Réponse MCP valide : nom + version du serveur.
    Valid {
        name: Option<String>,
        version: Option<String>,
        protocol: Option<String>,
    },
    /// Réponse JSON-RPC correcte mais pas un serveur MCP (pas de serverInfo).
    NotMcp,
    /// Ce n'est pas du JSON-RPC du tout.
    NotJsonRpc,
}

/// Contrôle qu'une réponse `initialize` prouve qu'il s'agit bien d'un serveur
/// MCP.
///
/// Exigence : `jsonrpc == "2.0"` ET (`result.serverInfo.name` présent OU
/// `result.protocolVersion` présent OU `result.capabilities` présent).
/// Un `result` vide, une liste de tools, une page HTML → refus.
///
/// Le nom est la preuve la plus forte : les serveurs MCP l'annoncent
/// toujours dans `serverInfo.name` (champ obligatoire de la spec).
pub fn validate_initialize(res: &Value) -> McpValid {
    // 1. JSON-RPC 2.0 ? (accepte l'absence — certains serveurs omettent le champ)
    if let Some(v) = res.get("jsonrpc").and_then(|x| x.as_str()) {
        if v != "2.0" {
            return McpValid::NotJsonRpc;
        }
    }
    // 2. une erreur JSON-RPC n'est pas une preuve de serveur MCP
    if res.get("error").is_some() {
        return McpValid::NotMcp;
    }
    let Some(result) = res.get("result") else {
        return McpValid::NotJsonRpc;
    };
    if !result.is_object() {
        return McpValid::NotJsonRpc;
    }
    let si = result.get("serverInfo");
    let name = si
        .and_then(|s| s.get("name"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let version = si
        .and_then(|s| s.get("version"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let protocol = result
        .get("protocolVersion")
        .and_then(|x| x.as_str())
        .map(String::from);
    let has_caps = result.get("capabilities").is_some();
    // serverInfo.name : preuve canonique. protocolVersion/capabilities :
    // preuves acceptables pour les implémentations partielles.
    if name.is_none() && protocol.is_none() && !has_caps {
        return McpValid::NotMcp;
    }
    // Un name vide est un mensonge : on le traite comme absent.
    let name = name.filter(|n| !n.trim().is_empty());
    McpValid::Valid {
        name,
        version,
        protocol,
    }
}

/// Vrai si la réponse est celle d'un serveur MCP (raccourci).
pub fn is_mcp(res: &Value) -> bool {
    matches!(validate_initialize(res), McpValid::Valid { .. })
}

/// Clé d'identité stable d'un serveur : nom + (version ou port).
/// Deux entrées avec la même clé sont le MÊME serveur (doublon à fusionner),
/// même si elles ont été découvertes par des chemins différents (scan LAN,
/// import d'un pair, découverte SSH).
pub fn identity_key(name: Option<&str>, host: Option<&str>, port: u16) -> String {
    let n = name.unwrap_or("?").trim().to_lowercase();
    match host {
        Some(h) => format!("{n}@{h}:{port}"),
        None => format!("{n}@local:{port}"),
    }
}

/// Clé de dé-duplication « forte » : même nom sur la même machine, quel que
/// soit le port. Sert à fusionner les entrées d'un serveur qui a changé de
/// port (l'ancien port devient une entrée fantôme).
pub fn identity_key_weak(name: Option<&str>, host: Option<&str>) -> String {
    let n = name.unwrap_or("?").trim().to_lowercase();
    match host {
        Some(h) => format!("{n}@{h}"),
        None => format!("{n}@local"),
    }
}

/// Décide si une découverte doit REMPLACER une entrée existante (même
/// serveur, port changé) ou être ignorée (doublon strict).
///
/// - même host + même port → doublon strict : on ignore la nouvelle entrée ;
/// - même host + nom identique + port différent → le serveur a changé de
///   port : la NOUVELLE entrée remplace l'ancienne (l'ancienne devient
///   fantôme et doit disparaître) ;
/// - sinon → nouvelle entrée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DedupDecision {
    /// Entrée strictement dupliquée : ne rien faire.
    Duplicate,
    /// Remplace l'entrée existante (index dans la liste) — port changé.
    Replace(usize),
    /// Nouvelle entrée : à ajouter.
    Add,
}

/// Applique la règle de dé-duplication sur une liste de serveurs connus.
///
/// `known` : (nom, host, port) de chaque serveur déjà connu.
/// `cand_name` / `cand_host` / `cand_port` : le candidat à insérer.
pub fn dedup_decide(
    known: &[(Option<String>, Option<String>, u16)],
    cand_name: Option<&str>,
    cand_host: Option<&str>,
    cand_port: u16,
) -> DedupDecision {
    // 1. doublon strict : même hôte + même port
    if known
        .iter()
        .any(|(_n, h, p)| h.as_deref() == cand_host && *p == cand_port)
    {
        return DedupDecision::Duplicate;
    }
    // 2. même machine + même nom, port différent → l'ancien est fantôme
    let weak = identity_key_weak(cand_name, cand_host);
    let mut fallback: Option<usize> = None;
    for (i, (n, h, p)) in known.iter().enumerate() {
        if identity_key_weak(n.as_deref(), h.as_deref()) == weak && *p != cand_port {
            fallback = Some(i);
            break;
        }
    }
    match fallback {
        Some(i) => DedupDecision::Replace(i),
        None => DedupDecision::Add,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn initialize_valide_avec_serverinfo() {
        let v = json!({"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18",
            "capabilities":{"tools":{}},"serverInfo":{"name":"demo","version":"1.0"}}});
        match validate_initialize(&v) {
            McpValid::Valid {
                name,
                version,
                protocol,
            } => {
                assert_eq!(name.as_deref(), Some("demo"));
                assert_eq!(version.as_deref(), Some("1.0"));
                assert_eq!(protocol.as_deref(), Some("2025-06-18"));
            }
            other => panic!("attendu Valid, obtenu {:?}", other),
        }
    }

    #[test]
    fn result_vide_est_refuse() {
        // le faux positif le plus fréquent : un serveur HTTP générique qui
        // répond {"result":{}} à n'importe quoi
        let v = json!({"jsonrpc":"2.0","id":1,"result":{}});
        assert_eq!(validate_initialize(&v), McpValid::NotMcp);
        assert!(!is_mcp(&v));
    }

    #[test]
    fn html_et_liste_ne_sont_pas_mcp() {
        assert_eq!(
            validate_initialize(&json!({"result":{"items":[]}})),
            McpValid::NotMcp
        );
        assert_eq!(
            validate_initialize(&json!({"tools":[]})),
            McpValid::NotJsonRpc
        );
        assert_eq!(
            validate_initialize(&json!({"jsonrpc":"2.0","error":{"code":-32601}})),
            McpValid::NotMcp
        );
        assert_eq!(
            validate_initialize(&json!({"jsonrpc":"1.0","result":{"serverInfo":{"name":"x"}}})),
            McpValid::NotJsonRpc
        );
    }

    #[test]
    fn capabilities_seules_suffisent_pour_une_implémentation_partielle() {
        let v =
            json!({"jsonrpc":"2.0","result":{"protocolVersion":"2024-11-05","capabilities":{}}});
        assert!(is_mcp(&v));
    }

    #[test]
    fn nom_vide_traite_comme_absent() {
        let v = json!({"jsonrpc":"2.0","result":{"serverInfo":{"name":"   "},"capabilities":{}}});
        match validate_initialize(&v) {
            McpValid::Valid { name, .. } => assert!(name.is_none()),
            other => panic!("attendu Valid, obtenu {:?}", other),
        }
    }

    #[test]
    fn dedup_meme_port_ignore() {
        let known = vec![(Some("discord".into()), Some("10.0.0.5".into()), 3141u16)];
        assert_eq!(
            dedup_decide(&known, Some("discord"), Some("10.0.0.5"), 3141),
            DedupDecision::Duplicate
        );
    }

    #[test]
    fn dedup_port_change_remplace_l_ancien() {
        let known = vec![(Some("discord".into()), Some("10.0.0.5".into()), 3141u16)];
        assert_eq!(
            dedup_decide(&known, Some("discord"), Some("10.0.0.5"), 9999),
            DedupDecision::Replace(0)
        );
    }

    #[test]
    fn dedup_nouveau_serveur_ajoute() {
        let known = vec![(Some("discord".into()), Some("10.0.0.5".into()), 3141u16)];
        assert_eq!(
            dedup_decide(&known, Some("gmail"), Some("10.0.0.6"), 3147),
            DedupDecision::Add
        );
        // même nom, machine différente → PAS un doublon
        assert_eq!(
            dedup_decide(&known, Some("discord"), Some("10.0.0.6"), 3141),
            DedupDecision::Add
        );
    }

    #[test]
    fn dedup_local_et_distant_ne_se_confondent_pas() {
        let known = vec![(Some("demo".into()), None, 3000u16)];
        // même nom, mais hôte distant → entrée distincte
        assert_eq!(
            dedup_decide(&known, Some("demo"), Some("10.0.0.5"), 3000),
            DedupDecision::Add
        );
        // même nom, même hôte, port différent → fantôme à remplacer
        assert_eq!(
            dedup_decide(&known, Some("demo"), None, 3001),
            DedupDecision::Replace(0)
        );
    }
}
