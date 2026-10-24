//! mcp-command-center — CLI headless.
//!
//! Zéro interface : moteur + serveur HTTP + journal. L'UI native (fenêtre OS)
//! est le binaire `desktop` (Tauri 2). Lancé avec `--hidden` (autostart), la
//! fenêtre console est masquée — la partie HTTP ne doit rien montrer à l'écran.

use mcp_command_center::manage;

fn main() {
    // MCC_CONFIG override ; sinon résolution runtime (dev → manifest, release → exe dir)
    let _config_path =
        manage::resolve_config_path(env!("CARGO_MANIFEST_DIR"), std::env::var("MCC_CONFIG").ok());
    #[cfg(all(windows, not(test)))]
    hide_console_if_hidden();
    let outcome = mcp_command_center::run_headless();
    if outcome.reused {
        // INSTANCE UNIQUE : un moteur tourne déjà (desktop ou headless) — il
        // supervise déjà les serveurs ; on ne double rien, on sort proprement.
        println!(
            "mcp-command-center : un moteur tourne déjà sur {} — rien à lancer, sortie.",
            outcome.url
        );
        return;
    }
    println!(
        "mcp-command-center v{} (headless)",
        env!("CARGO_PKG_VERSION")
    );
    println!("API : {}", outcome.url);
    println!("  GET  /api/state            état complet + journal");
    println!("  GET  /api/detail?id=<id>   détail d'un serveur");
    println!("  GET  /api/store            catalogue boutique");
    println!("  POST /api/action           {{\"action\": ...}}");
    println!("Headless : aucune interface ouverte. Lance `desktop` pour la fenêtre native.");
    println!("Journal disque : mcc.log (à côté du config.json).");
    mcp_command_center::park_forever();
}

/// `--hidden` (autostart / lancement silencieux) : masque la fenêtre console du
/// moteur — la partie HTTP tourne en arrière-plan sans rien montrer. Sans
/// effet quand le binaire est lancé à la main sans flag (console de dev).
#[cfg(all(windows, not(test)))]
fn hide_console_if_hidden() {
    if !std::env::args().any(|a| a == "--hidden") {
        return;
    }
    // GetConsoleWindow (kernel32) + ShowWindow(SW_HIDE) (user32), sans crate externe
    #[link(name = "kernel32")]
    extern "system" {
        fn GetConsoleWindow() -> isize;
    }
    #[link(name = "user32")]
    extern "system" {
        fn ShowWindow(hwnd: isize, ncmdshow: i32) -> i32;
    }
    const SW_HIDE: i32 = 0;
    unsafe {
        let hwnd = GetConsoleWindow();
        if hwnd != 0 {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}
