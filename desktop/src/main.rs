#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Tauri 2 : fenêtre native OS. Le contenu est servi par le moteur embarqué
// (run_headless démarré avant le builder) : la fenêtre navigue vers
// http://127.0.0.1:<port>. Aucun navigateur, aucune console : un vrai
// process applicatif Windows.
//
// Cycle de vie : fermer la fenêtre la MASQUE (l'app reste en zone de
// notification, icône tray) ; quitter se fait depuis le menu du tray.
// L'inscription au démarrage de l'OS est gérée par le core (bouton
// « Auto-boot ON/OFF » de la barre de titre, argument `--hidden` : au boot
// l'app vit dans le tray et gère les serveurs sans ouvrir de fenêtre).

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

fn main() {
    // Instance unique garantie à deux niveaux : (1) tauri_plugin_single_instance
    // ci-dessous réaffiche la fenêtre existante si le desktop est relancé ;
    // (2) run_headless se RECONNECTE à un moteur déjà en cours (headless ou
    // desktop) au lieu d'en lancer un second — un seul processus supervise.
    let ui_url = mcp_command_center::run_headless().url;
    // lancé au boot de Windows (entrée autostart) → pas de fenêtre, tray seulement
    let hidden = std::env::args().any(|a| a == "--hidden");

    tauri::Builder::default()
        // deuxième lancement pendant que l'app tourne : réafficher la fenêtre
        // existante au lieu d'un deuxième moteur qui mourrait sur les ports UI
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .setup(move |app| {
            let win = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External(ui_url.parse().expect("URL UI invalide")),
            )
            .title("Console MCP")
            .inner_size(1320.0, 880.0)
            .min_inner_size(940.0, 620.0)
            // Barre de titre native supprimée : le header sombre de l'UI sert de
            // barre de fenêtre (drag + réduire/agrandir/fermer via la capability
            // main-remote-ui). Évite la barre blanche OS au-dessus du header.
            .decorations(false)
            .visible(!hidden)
            // IMPÉRATIF pour le drag & drop interne : le handler de drop de
            // fichiers de WebView2 (activé par défaut) avale les événements
            // HTML5 dragstart/over/drop — on le désactive pour que les lignes
            // de l'explorateur soient déplaçables dans la fenêtre native.
            .disable_drag_drop_handler()
            // icône explicite sur la fenêtre : barre des tâches et Alt+Tab
            // prennent l'icône personnalisée (hub MCP), pas un glyphe générique
            .icon(
                app.default_window_icon()
                    .expect("icône embarquée manquante")
                    .clone(),
            )?
            .build()?;
            let _ = win.set_focus();

            // icône tray : afficher le tableau de bord, quitter
            let show = MenuItem::with_id(
                app,
                "show",
                "Afficher le tableau de bord",
                true,
                None::<&str>,
            )?;
            let quit = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::with_id("mcc-tray")
                .icon(
                    app.default_window_icon()
                        .expect("icône embarquée manquante")
                        .clone(),
                )
                .tooltip("Console MCP — gestionnaire de serveurs MCP")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, ev| match ev.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                // clic gauche sur l'icône : afficher / masquer la fenêtre
                .on_tray_icon_event(|tray, ev| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = ev
                    {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.unminimize();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        // fermer la fenêtre = masquer dans le tray ; vraie sortie via le tray
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("erreur au lancement de l'application desktop");
}
