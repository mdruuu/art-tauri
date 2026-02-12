use tauri::AppHandle;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub const DEFAULT_HOTKEY: &str = "CmdOrCtrl+Shift+G";

pub fn register_hotkey(app: &AppHandle, shortcut: &str) -> Result<(), String> {
    // Unregister all first
    let _ = app.global_shortcut().unregister_all();

    let app_handle = app.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                log::info!("Hotkey pressed, toggling artwork display");
                let app = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    crate::commands::toggle_overlays(app).await;
                });
            }
        })
        .map_err(|e| format!("Failed to register shortcut '{shortcut}': {e}"))?;

    log::info!("Registered hotkey: {shortcut}");
    Ok(())
}
