use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition,
};

pub fn create_tray(app: &AppHandle) -> Result<TrayIcon, Box<dyn std::error::Error>> {
    let refresh_item = MenuItemBuilder::with_id("refresh", "刷新").build(app)?;
    let settings_item = MenuItemBuilder::with_id("settings", "设置").build(app)?;
    let autostart_item = MenuItemBuilder::with_id("autostart", "开机自启").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[
            &refresh_item,
            &settings_item,
            &autostart_item,
            &quit_item,
        ])
        .build()?;

    let icon = {
        let cursor = std::io::Cursor::new(include_bytes!("../icons/32x32.png"));
        let decoder = png::Decoder::new(cursor);
        let mut reader = decoder
            .read_info()
            .map_err(|e| format!("Failed to read PNG info: {e}"))?;
        let buf_size = reader.output_buffer_size();
        let mut buf = vec![0; buf_size];
        reader
            .next_frame(&mut buf)
            .map_err(|e| format!("Failed to decode PNG: {e}"))?;
        let info = reader.info();
        Image::new_owned(buf, info.width, info.height)
    };

    let tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("DeepSeek 监控")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "refresh" => {
                log::info!("Menu: refresh clicked");
                let _ = app.emit("menu-refresh", ());
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
                app.exit(0);
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
                        // Multi-monitor aware positioning: find the monitor
                        // that contains the current cursor position (which
                        // is where the user just clicked the tray icon).
                        let ws = window.outer_size().unwrap_or(tauri::PhysicalSize {
                            width: 380,
                            height: 660,
                        });

                        let monitors = window.available_monitors().unwrap_or_default();
                        let cursor = window
                            .cursor_position()
                            .unwrap_or(tauri::PhysicalPosition { x: 0.0, y: 0.0 });

                        let target = monitors
                            .iter()
                            .find(|m| {
                                let p = m.position();
                                let s = m.size();
                                cursor.x >= p.x as f64
                                    && cursor.x < (p.x + s.width as i32) as f64
                                    && cursor.y >= p.y as f64
                                    && cursor.y < (p.y + s.height as i32) as f64
                            })
                            .or_else(|| monitors.first());

                        if let Some(monitor) = target {
                            let size = monitor.size();
                            let pos = monitor.position();
                            let scale = monitor.scale_factor();
                            let x = (pos.x as f64 + size.width as f64 / scale
                                - ws.width as f64
                                - 12.0)
                                .max(0.0);
                            let y = (pos.y as f64 + size.height as f64 / scale
                                - ws.height as f64
                                - 12.0)
                                .max(0.0);
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

    Ok(tray)
}
