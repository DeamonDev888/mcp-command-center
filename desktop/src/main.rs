#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Tauri 2 : fenêtre native OS. Le contenu est servi par le moteur embarqué
// (run_headless démarré avant le builder) : la fenêtre navigue vers
// http://127.0.0.1:<port>. Aucun navigateur, aucune console : un vrai
// process applicatif Windows.

fn main() {
    let ui_url = mcp_command_center::run_headless();

    tauri::Builder::default()
        .setup(move |app| {
            let win = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External(ui_url.parse().expect("URL UI invalide")),
            )
            .title("MCP Command Center")
            .inner_size(1320.0, 880.0)
            .min_inner_size(940.0, 620.0)
            .build()?;
            let _ = win.set_focus();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erreur au lancement de l'application desktop");
}
