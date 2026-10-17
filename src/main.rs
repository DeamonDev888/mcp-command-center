//! mcp-command-center — CLI headless.
//!
//! Zéro fenêtre, zéro navigateur, zéro interface : moteur + serveur HTTP + log
//! console. L'UI native (fenêtre OS) est le binaire `desktop` (Tauri 2).

use mcp_command_center::manage;

fn main() {
    // MCC_CONFIG override ; sinon résolution runtime (dev → manifest, release → exe dir)
    let _config_path =
        manage::resolve_config_path(env!("CARGO_MANIFEST_DIR"), std::env::var("MCC_CONFIG").ok());
    let ui_url = mcp_command_center::run_headless();
    println!(
        "mcp-command-center v{} (headless)",
        env!("CARGO_PKG_VERSION")
    );
    println!("API : {}", ui_url);
    println!("  GET  /api/state            état complet + journal");
    println!("  GET  /api/detail?id=<id>   détail d'un serveur");
    println!("  GET  /api/store            catalogue boutique");
    println!("  POST /api/action           {{\"action\": ...}}");
    println!("Headless : aucune interface ouverte. Lance `desktop` pour la fenêtre native.");
    mcp_command_center::park_forever();
}
