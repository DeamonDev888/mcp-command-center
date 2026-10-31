// Démarrage automatique de l'app avec l'OS (registre Windows / autostart freedesktop).
// (split v0.7.20 : manage.rs reste la racine — API stable via réexport)
use super::*;

const AUTOSTART_RUN_NAME: &str = "MCP Control";

/// Arguments de l'entrée de démarrage automatique (compilé hors tests seulement :
/// en build test le corps réel est neutralisé, la fonction deviendrait du code mort).
#[cfg(not(test))]
fn autostart_entry_args() -> String {
    let exe = std::env::current_exe()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    format!("\"{}\" --hidden", exe)
}

/// L'entrée de démarrage automatique existe-t-elle actuellement ?
pub fn autostart_active() -> bool {
    #[cfg(windows)]
    {
        let mut c = Command::new("reg");
        probe::probe_hidden_cmd(&mut c);
        c.args([
            "query",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/v",
            AUTOSTART_RUN_NAME,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        autostart_file_path().map(|p| p.exists()).unwrap_or(false)
    }
}

/// Applique la préférence : crée ou supprime l'entrée de démarrage automatique.
/// Windows : registre HKCU …\Run. Linux : ~/.config/autostart/*.desktop.
/// macOS : ~/Library/LaunchAgents/*.plist.
pub fn apply_autostart(state: &Arc<AppState>, enable: bool) {
    // les tests ne doivent JAMAIS toucher le registre/fichiers de l'OS réels
    #[cfg(test)]
    {
        let _ = (state, enable);
    }
    // corps réel séparé : le garde cfg(test) ne laisse rien d'inatteignable
    // (l'ancien `return` en build test déclenchait unreachable_code — clippy)
    #[cfg(not(test))]
    apply_autostart_os(state, enable);
}

/// Corps réel de l'alignement autostart (compilé hors tests seulement).
#[cfg(not(test))]
#[allow(unused_must_use)]
fn apply_autostart_os(state: &Arc<AppState>, enable: bool) {
    #[cfg_attr(not(windows), allow(unused_variables))] // lu dans le bloc cfg(windows)
    let args = autostart_entry_args();
    #[cfg(windows)]
    {
        let root = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        let mut c = Command::new("reg");
        probe::probe_hidden_cmd(&mut c);
        let status = if enable {
            c.args([
                "add",
                root,
                "/v",
                AUTOSTART_RUN_NAME,
                "/t",
                "REG_SZ",
                "/d",
                &args,
                "/f",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        } else {
            c.args(["delete", root, "/v", AUTOSTART_RUN_NAME, "/f"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        };
        if status.map(|s| s.success()).unwrap_or(false) {
            add_log(
                state,
                "info",
                &format!(
                    "[autostart] entrée de démarrage {}",
                    if enable { "créée" } else { "supprimée" }
                ),
            );
        } else {
            add_log(
                state,
                "warn",
                "[autostart] reg a échoué — l'entrée n'a pas changé",
            );
        }
    }
    #[cfg(not(windows))]
    {
        let Some(path) = autostart_file_path() else {
            return;
        };
        if enable {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir);
            }
            let exe = std::env::current_exe()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            let content = if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                format!(
                    "[Desktop Entry]
Type=Application
Name=MCP Control
Exec=\"{}\" --hidden
Hidden=false
X-GNOME-Autostart-enabled=true
",
                    exe
                )
            } else {
                format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\"><dict>
<key>Label</key><string>com.deamondev.mcp-control</string>
<key>ProgramArguments</key><array>
<string>{}</string><string>--hidden</string>
</array>
<key>RunAtLoad</key><true/>
</dict></plist>
",
                    exe
                )
            };
            std::fs::write(&path, content);
            add_log(state, "info", "[autostart] entrée de démarrage créée");
        } else if path.exists() {
            std::fs::remove_file(&path);
            add_log(state, "info", "[autostart] entrée de démarrage supprimée");
        }
    }
}

/// Chemin du fichier autostart (Linux : XDG ; macOS : LaunchAgent). Windows :
/// registre — None (géré par reg).
#[cfg(not(windows))]
fn autostart_file_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    #[cfg(target_os = "macos")]
    {
        Some(
            std::path::PathBuf::from(home)
                .join("Library/LaunchAgents/com.deamondev.mcp-control.plist"),
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        Some(std::path::PathBuf::from(home).join(".config/autostart/mcp-control-desktop.desktop"))
    }
}

/// Au boot : aligne l'entrée sur la préférence persistée (créée au premier
/// lancement — settings.autostart vaut true par défaut).
pub fn sync_autostart_at_boot(state: &Arc<AppState>) {
    let want = lockv(&state.settings).autostart;
    if autostart_active() != want {
        apply_autostart(state, want);
    }
}
