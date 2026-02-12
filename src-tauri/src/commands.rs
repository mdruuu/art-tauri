use crate::art_cache::ArtCache;
use crate::hotkey;
use crate::windows;
use crate::Artwork;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_store::StoreExt;

#[tauri::command]
pub async fn get_current_artwork(cache: State<'_, ArtCache>) -> Result<Option<Artwork>, String> {
    Ok(cache.current().await)
}

#[tauri::command]
pub async fn next_artwork(app: AppHandle, cache: State<'_, ArtCache>) -> Result<Artwork, String> {
    let artwork = cache.next().await?;
    // Emit to all overlay windows
    let _ = app.emit("artwork-changed", &artwork);
    Ok(artwork)
}

#[tauri::command]
pub async fn prev_artwork(app: AppHandle, cache: State<'_, ArtCache>) -> Result<Artwork, String> {
    let artwork = cache.prev().await?;
    let _ = app.emit("artwork-changed", &artwork);
    Ok(artwork)
}

#[tauri::command]
pub fn overlay_ready(app: AppHandle) {
    windows::show_overlay_windows(&app);
}

#[tauri::command]
pub async fn dismiss_overlays(app: AppHandle) -> Result<(), String> {
    // Defer the close so the IPC response is sent before the webview is destroyed.
    // Without this, calling dismiss from inside the overlay's own webview panics
    // because destroy() kills the IPC channel before Ok(()) can be returned.
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        windows::close_overlay_windows(&app);
    });
    Ok(())
}

#[tauri::command]
pub async fn get_hotkey(app: AppHandle) -> Result<String, String> {
    let store = app
        .store("settings.json")
        .map_err(|e| format!("Store error: {e}"))?;
    let hotkey = store
        .get("hotkey")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| hotkey::DEFAULT_HOTKEY.to_string());
    Ok(hotkey)
}

#[tauri::command]
pub async fn set_hotkey(app: AppHandle, hotkey: String) -> Result<(), String> {
    // Try to register the new hotkey first
    hotkey::register_hotkey(&app, &hotkey)?;

    // Save to store
    let store = app
        .store("settings.json")
        .map_err(|e| format!("Store error: {e}"))?;
    store.set("hotkey", serde_json::Value::String(hotkey));

    Ok(())
}

/// Toggle overlay display - called from hotkey and tray
pub async fn toggle_overlays(app: AppHandle) {
    // Check if overlays are currently shown
    let has_overlays = app
        .webview_windows()
        .keys()
        .any(|label| label.starts_with("overlay-"));

    if has_overlays {
        windows::close_overlay_windows(&app);
    } else {
        show_art(app).await;
    }
}

/// Show artwork on all monitors
pub async fn show_art(app: AppHandle) {
    // Get artwork first
    let cache = app.state::<ArtCache>();
    let artwork = match cache.next().await {
        Ok(art) => art,
        Err(e) => {
            log::error!("Failed to get artwork: {e}");
            return;
        }
    };

    // Create overlay windows (created hidden — frontend calls overlay_ready once image loads)
    if let Err(e) = windows::create_overlay_windows(&app) {
        log::error!("Failed to create overlays: {e}");
        return;
    }

    // Emit artwork immediately; the frontend also calls get_current_artwork on mount as fallback
    let _ = app.emit("artwork-changed", &artwork);
}
