mod art_api;
mod art_cache;
mod commands;
mod hotkey;
mod windows;

use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artwork {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub date: String,
    pub medium: String,
    pub source: String,
    pub image_base64: String,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(art_cache::ArtCache::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_current_artwork,
            commands::next_artwork,
            commands::prev_artwork,
            commands::overlay_ready,
            commands::dismiss_overlays,
            commands::get_hotkey,
            commands::set_hotkey,
        ])
        .setup(|app| {
            // Set accessory activation policy (no dock icon)
            #[cfg(target_os = "macos")]
            {
                use tauri::ActivationPolicy;
                app.set_activation_policy(ActivationPolicy::Accessory);
            }

            // Set up tray
            setup_tray(app.handle())?;

            // Register global hotkey
            let handle = app.handle().clone();
            let store = tauri_plugin_store::StoreExt::store(app.handle(), "settings.json")
                .expect("Failed to open store");
            let shortcut = store
                .get("hotkey")
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| hotkey::DEFAULT_HOTKEY.to_string());

            if let Err(e) = hotkey::register_hotkey(&handle, &shortcut) {
                log::error!("Failed to register hotkey: {e}");
            }

            // Start background prefetch
            let cache = app.state::<art_cache::ArtCache>();
            cache.start_prefetch();

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}

fn setup_tray(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::TrayIconBuilder;

    let show = MenuItemBuilder::with_id("show", "Show Art").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show)
        .separator()
        .item(&settings)
        .separator()
        .item(&quit)
        .build()?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| {
            let app = app.clone();
            match event.id().as_ref() {
                "show" => {
                    tauri::async_runtime::spawn(async move {
                        commands::toggle_overlays(app).await;
                    });
                }
                "settings" => {
                    if let Err(e) = windows::create_settings_window(&app) {
                        log::error!("Failed to open settings: {e}");
                    }
                }
                "quit" => {
                    std::process::exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}
