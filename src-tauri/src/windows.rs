use tauri::{AppHandle, Manager, WebviewWindowBuilder, WebviewUrl};

/// Create overlay windows on all monitors
pub fn create_overlay_windows(app: &AppHandle) -> Result<(), String> {
    // Close any existing overlay windows first
    close_overlay_windows(app);

    let monitors = app
        .available_monitors()
        .map_err(|e| format!("Failed to get monitors: {e}"))?;

    for (i, monitor) in monitors.iter().enumerate() {
        let label = format!("overlay-{i}");

        // If a window with this label still exists, destroy it first
        if let Some(existing) = app.get_webview_window(&label) {
            let _ = existing.destroy();
        }

        let pos = monitor.position();
        let size = monitor.size();
        let scale = monitor.scale_factor();

        // Convert physical pixels to logical pixels (critical on Retina/HiDPI displays)
        let logical_w = size.width as f64 / scale;
        let logical_h = size.height as f64 / scale;
        let logical_x = pos.x as f64 / scale;
        let logical_y = pos.y as f64 / scale;

        log::info!(
            "Creating overlay on monitor {i}: {logical_w}x{logical_h} (logical) at ({logical_x},{logical_y}), scale={scale}",
        );

        let url = if cfg!(debug_assertions) {
            WebviewUrl::External("http://localhost:1420/src/overlay.html".parse().unwrap())
        } else {
            WebviewUrl::App("src/overlay.html".into())
        };

        WebviewWindowBuilder::new(app, &label, url)
            .title("")
            .inner_size(logical_w, logical_h)
            .position(logical_x, logical_y)
            .decorations(false)
            .always_on_top(true)
            .resizable(false)
            .skip_taskbar(true)
            .visible(false)
            .build()
            .map_err(|e| format!("Failed to create overlay {i}: {e}"))?;
    }

    // Switch to Regular activation policy so we receive keyboard events
    #[cfg(target_os = "macos")]
    {
        use tauri::ActivationPolicy;
        let _ = app.set_activation_policy(ActivationPolicy::Regular);
    }

    // Hide menu bar and dock so overlay is truly fullscreen
    #[cfg(target_os = "macos")]
    {
        let _ = app.run_on_main_thread(|| {
            use objc2::MainThreadMarker;
            use objc2_app_kit::{NSApplication, NSApplicationPresentationOptions};
            let mtm = MainThreadMarker::new().unwrap();
            let ns_app = NSApplication::sharedApplication(mtm);
            ns_app.setPresentationOptions(
                NSApplicationPresentationOptions::HideDock
                    | NSApplicationPresentationOptions::HideMenuBar,
            );
        });
    }

    Ok(())
}

/// Show all overlay windows (called after artwork is ready)
pub fn show_overlay_windows(app: &AppHandle) {
    for (label, window) in app.webview_windows() {
        if label.starts_with("overlay-") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

/// Close all overlay windows
pub fn close_overlay_windows(app: &AppHandle) {
    let windows: Vec<_> = app
        .webview_windows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("overlay-"))
        .collect();

    // Restore menu bar and dock before closing windows
    #[cfg(target_os = "macos")]
    {
        let _ = app.run_on_main_thread(|| {
            use objc2::MainThreadMarker;
            use objc2_app_kit::{NSApplication, NSApplicationPresentationOptions};
            let mtm = MainThreadMarker::new().unwrap();
            let ns_app = NSApplication::sharedApplication(mtm);
            ns_app.setPresentationOptions(NSApplicationPresentationOptions::Default);
        });
    }

    for (_, window) in windows {
        let _ = window.close();
    }

    // Switch back to Accessory policy (no dock icon)
    #[cfg(target_os = "macos")]
    {
        use tauri::ActivationPolicy;
        let _ = app.set_activation_policy(ActivationPolicy::Accessory);
    }
}

/// Create the settings window
pub fn create_settings_window(app: &AppHandle) -> Result<(), String> {
    // If settings already open, just focus it
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.set_focus();
        return Ok(());
    }

    let url = if cfg!(debug_assertions) {
        WebviewUrl::External("http://localhost:1420/src/settings.html".parse().unwrap())
    } else {
        WebviewUrl::App("src/settings.html".into())
    };

    WebviewWindowBuilder::new(app, "settings", url)
        .title("Art — Settings")
        .inner_size(400.0, 300.0)
        .resizable(false)
        .build()
        .map_err(|e| format!("Failed to create settings window: {e}"))?;

    Ok(())
}
