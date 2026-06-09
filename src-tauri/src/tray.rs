use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition,
};

pub fn create_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let refresh_item = MenuItemBuilder::with_id("refresh", "Refresh").build(app)?;
    let import_item = MenuItemBuilder::with_id("import", "Import CSV...").build(app)?;
    let settings_item = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
    let autostart_item = MenuItemBuilder::with_id("autostart", "Auto Start").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[
            &refresh_item,
            &import_item,
            &settings_item,
            &autostart_item,
            &quit_item,
        ])
        .build()?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("DeepSeek Monitor")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "refresh" => {
                log::info!("Menu: refresh clicked");
                let _ = app.emit("menu-refresh", ());
            }
            "import" => {
                log::info!("Menu: import clicked");
                let _ = app.emit("menu-import", ());
            }
            "settings" => {
                log::info!("Menu: settings clicked");
                let _ = app.emit("menu-settings", ());
            }
            "autostart" => {
                log::info!("Menu: autostart clicked");
                let _ = app.emit("menu-autostart", ());
            }
            "quit" => {
                log::info!("Menu: quit clicked");
                std::process::exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        // Position to bottom-right
                        if let Ok(Some(monitor)) = window.primary_monitor() {
                            let size = monitor.size();
                            let scale = monitor.scale_factor();
                            let ws = window.outer_size().unwrap_or(tauri::PhysicalSize {
                                width: 380,
                                height: 640,
                            });
                            let x = (size.width as f64 / scale - ws.width as f64 - 12.0).max(0.0);
                            let y = (size.height as f64 / scale - ws.height as f64 - 12.0).max(0.0);
                            let _ = window.set_position(PhysicalPosition::new(x as i32, y as i32));
                        }
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
