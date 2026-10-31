// Vérification de mise à jour (release GitHub) — au boot puis chaque heure.
// Assets de release : table blanche (jamais construits depuis la requête).
// (split v0.7.20 : manage.rs reste la racine — API stable via réexport)
use super::*;

pub fn version_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim()
            .trim_start_matches('v')
            .split('.')
            .map(|p| p.trim().parse::<u64>().unwrap_or(0))
            .collect()
    };
    let (a, b) = (parse(latest), parse(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return x > y;
        }
    }
    false
}

/// Interroge la dernière release GitHub. `curl` d'abord (léger, présent sur
/// macOS/Linux/Windows 10+) avec repli PowerShell — l'ancien code n'utilisait
/// QUE PowerShell : la vérification était silencieusement inopérante partout
/// ailleurs (audit 2026-10-24). `Ok(Some(tag))` = tag lu ; `Ok(None)` = réponse
/// sans tag ; `Err` = réseau/API/outil indisponible — la distinction permet au
/// bouton « Vérifier » de ne plus prétendre « à jour » quand le check a raté.
fn github_latest_tag() -> Result<Option<String>, &'static str> {
    let mut c = Command::new("curl");
    probe::probe_hidden_cmd(&mut c);
    if let Ok(out) = c
        .args([
            "-fsSL",
            "-m",
            "10",
            "-H",
            "User-Agent: mcp-control",
            "https://api.github.com/repos/DeamonDev888/mcp-control/releases/latest",
        ])
        .output()
    {
        if out.status.success() {
            if let Ok(v) = serde_json::from_slice::<Value>(&out.stdout) {
                if let Some(tag) = v.get("tag_name").and_then(|x| x.as_str()) {
                    return Ok(Some(tag.to_string()));
                }
            }
        }
    }
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-Command",
        "(Invoke-RestMethod -TimeoutSec 10 -Headers @{ 'User-Agent' = 'mcp-control' }           'https://api.github.com/repos/DeamonDev888/mcp-control/releases/latest').tag_name",
    ]);
    probe::probe_hidden_cmd(&mut cmd);
    let Ok(out) = cmd.output() else {
        return Err("réseau injoignable (curl et PowerShell en échec)");
    };
    if !out.status.success() {
        return Err("réseau injoignable (curl et PowerShell en échec)");
    }
    let tag = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if tag.is_empty() || tag.len() > 30 {
        return Ok(None);
    }
    Ok(Some(tag))
}

/// Issue d'une vérification de release GitHub — distinguée pour que le bouton
/// « Vérifier » des réglages puisse dire « à jour (vérifié à l'instant) »,
/// « vX disponible » ou « vérification impossible (réseau ?) » au lieu du
/// silence d'avant, qui faisait croire à un clic mort.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheck {
    /// Release plus récente lue et enregistrée dans update_latest.
    Available(String),
    /// API GitHub lue : rien de plus récent que la version installée.
    UpToDate,
    /// Réseau/API injoignable ou réponse illisible — aucun verdict.
    Unreachable,
}

/// Vérifie la dernière release GitHub : `update_latest` = tag SI plus récent
/// que l'installée ; vidé si l'installée redevient la dernière (sinon le badge
/// « mise à jour » resterait affiché avec une dispo périmée). Côté journal,
/// silence hors ligne — le poller horaire ne doit pas spammer : le verdict est
/// RENVOYÉ et l'action `check_update` le traduit pour l'UI.
pub fn check_update(state: &Arc<AppState>) -> UpdateCheck {
    // Le tag GitHub arrive sous la forme « v0.7.16 » : on range la version SANS
    // son préfixe. Tous les consommateurs (message du journal, latest_version de
    // /api/state, réponse check_update) affichent « v » + version — préfixer le
    // tag tel quel produisait « vv0.7.16 » dans le journal et le badge.
    let tag = match github_latest_tag() {
        Ok(Some(t)) => t.strip_prefix('v').unwrap_or(&t).to_string(),
        Ok(None) | Err(_) => return UpdateCheck::Unreachable,
    };
    let current = env!("CARGO_PKG_VERSION");
    if version_newer(&tag, current) {
        // VERROUILLAGE (audit 2026-10-25) : ne PAS tenir update_latest pendant
        // add_log (qui prend le verrou log). state_json prend log PUIS
        // update_latest → un AB-BA était possible : check_update tenait
        // update_latest et attendait log pendant que state_json tenait log et
        // attendait update_latest. On libère update_latest AVANT de logger.
        let should_log = {
            let mut u = lockv(&state.update_latest);
            if u.as_deref() != Some(tag.as_str()) {
                *u = Some(tag.clone());
                true
            } else {
                false
            }
        };
        if should_log {
            add_log(
                state,
                "info",
                &format!(
                    "mise à jour disponible : v{} (installée : v{}) — github.com/DeamonDev888/mcp-control/releases",
                    tag, current
                ),
            );
        }
        return UpdateCheck::Available(tag);
    }
    // L'installée redevient la dernière : on vide le badge.
    {
        let mut u = lockv(&state.update_latest);
        if u.is_some() {
            *u = None;
        }
    }
    UpdateCheck::UpToDate
}

pub fn release_asset(kind: &str, os: &str) -> Option<&'static str> {
    match (kind, os) {
        ("desktop", "windows") => Some("mcp-control-desktop-windows-x86_64-setup.exe"),
        ("desktop", "macos") => Some("mcp-control-desktop-macos-universal"),
        ("desktop", "linux") => Some("mcp-control-desktop-linux-x86_64"),
        ("cli", "windows") => Some("mcp-control-cli-windows-x86_64.exe"),
        ("cli", "macos") => Some("mcp-control-cli-macos-universal"),
        ("cli", "linux") => Some("mcp-control-cli-linux-x86_64"),
        _ => None,
    }
}

/// L'OS du binaire qui tourne, tel que nommé dans la table ci-dessus.
pub fn engine_os() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "linux"
    }
    #[cfg(not(any(windows, target_os = "macos", all(unix, not(target_os = "macos")))))]
    {
        "inconnu"
    }
}
