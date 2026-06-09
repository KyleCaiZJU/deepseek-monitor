mod api;
mod commands;
mod csv_import;
mod platform_api;
mod store;
mod tray;

use std::sync::Arc;

use commands::Settings;
use store::Store;
use chrono::Datelike;
use tauri::{Emitter, Manager, tray::TrayIcon};
use tauri_plugin_store::StoreExt;

pub struct SettingsState {
    pub settings: std::sync::Mutex<Settings>,
}

impl SettingsState {
    pub fn new(s: Settings) -> Self {
        Self {
            settings: std::sync::Mutex::new(s),
        }
    }

    pub fn get(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn update(&self, s: Settings) {
        *self.settings.lock().unwrap() = s;
    }
}

/// Holds the system tray icon handle so the timer loop can update the tooltip.
pub struct TrayIconState {
    pub tray: std::sync::Mutex<Option<TrayIcon>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_dir = dirs_data_dir();
    std::fs::create_dir_all(&db_dir).ok();
    let db_path = std::path::Path::new(&db_dir).join("deepseek-monitor.db");

    let store = Arc::new(Store::new(&db_path).expect("Failed to open database"));

    let cached_settings = Settings::default();
    if let Ok(Some((_, balance_str))) = store.get_last_balance() {
        log::info!("Loaded cached balance: {balance_str}");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            app.manage(SettingsState::new(cached_settings));
            app.manage(store.clone());

            // C01: Load persisted settings from disk
            let store_handle = app.store("settings.json").map_err(|e| e.to_string())?;
            if let Some(saved_json) = store_handle.get("settings") {
                if let Ok(saved_settings) = serde_json::from_value::<Settings>(saved_json) {
                    let state = app.state::<SettingsState>();
                    state.update(saved_settings);
                    log::info!("Loaded settings from persistent store");
                }
            }

            let tray_icon = tray::create_tray(app.handle())?;
            app.manage(TrayIconState {
                tray: std::sync::Mutex::new(Some(tray_icon)),
            });

            let store_clone = store.clone();
            let app_handle = app.handle().clone();
            let last_platform_fetch: Arc<std::sync::Mutex<Option<chrono::DateTime<chrono::Utc>>>> =
                Arc::new(std::sync::Mutex::new(None));
            tauri::async_runtime::spawn(async move {
                // Issue 2: Initial platform data fetch at startup (don't wait for timer)
                {
                    let settings = app_handle.state::<SettingsState>().get();
                    let token = settings.platform_token.clone();
                    if !token.is_empty() {
                        let store_state = app_handle.state::<Arc<Store>>();
                        match commands::fetch_platform_usage(app_handle.clone(), store_state).await {
                            Ok(()) => {
                                *last_platform_fetch.lock().unwrap() = Some(chrono::Utc::now());
                                log::info!("Initial platform data fetch completed at startup");
                            }
                            Err(e) => {
                                log::warn!("Initial platform data fetch failed: {e}");
                            }
                        }
                    }
                }
                loop {
                    let settings = app_handle.state::<SettingsState>().get();
                    let interval = settings.interval_min.max(1);
                    tokio::time::sleep(tokio::time::Duration::from_secs(interval * 60)).await;

                    // Re-read settings in case they changed during sleep
                    let settings = app_handle.state::<SettingsState>().get();
                    let api_key = settings.api_key.clone();
                    if api_key.is_empty() {
                        continue;
                    }

                    match api::fetch_balance(&api_key).await {
                        Ok(resp) => {
                            if let Some(info) = resp.balance_infos.first() {
                                let now = chrono::Utc::now()
                                    .format("%Y-%m-%dT%H:%M:%S")
                                    .to_string();
                                let _ = store_clone.insert_balance(
                                    &now,
                                    &info.total_balance,
                                    resp.is_available,
                                );
                                let bal: f64 = info.total_balance.parse().unwrap_or(0.0);
                                let _ = app_handle.emit("balance-updated", bal);

                                // C05: Update tray tooltip with live balance
                                let tray_state = app_handle.state::<TrayIconState>();
                                let guard = tray_state.tray.lock().unwrap();
                                if let Some(tray) = guard.as_ref() {
                                    let tooltip = format!("余额 ¥{:.2}", bal);
                                    let _ = tray.set_tooltip(Some(&tooltip));
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("Timer balance fetch failed: {e}");
                        }
                    }

                    // Platform usage: fetch every 30 minutes if token is set
                    let platform_token = settings.platform_token.clone();
                    if !platform_token.is_empty() {
                        let should_fetch = {
                            let guard = last_platform_fetch.lock().unwrap();
                            guard.map_or(true, |last| {
                                let elapsed = chrono::Utc::now() - last;
                                elapsed.num_minutes() >= 30
                            })
                        };
                        if should_fetch {
                            let now = chrono::Utc::now();
                            let month = now.month();
                            let year = now.year();

                            // Fetch usage data from export ZIP (cost + per-key amount in one request)
                            match platform_api::fetch_export_zip(&platform_token, month, year).await
                            {
                                Ok((amount_rows, cost_rows)) => {
                                    let ac = amount_rows.len();
                                    let cc = cost_rows.len();
                                    if ac > 0 {
                                        let _ = store_clone.upsert_usage(&amount_rows);
                                    }
                                    if cc > 0 {
                                        let _ = store_clone.upsert_cost(&cost_rows);
                                    }
                                    log::info!(
                                        "Timer: export ZIP upserted {ac} amount + {cc} cost rows"
                                    );
                                }
                                Err(e) => {
                                    log::warn!("Timer export ZIP fetch failed: {e}");
                                }
                            }

                            *last_platform_fetch.lock().unwrap() = Some(chrono::Utc::now());
                        }
                    }

                    if let Ok(dashboard) = store_clone.get_dashboard() {
                        let _ = app_handle.emit("dashboard-updated", dashboard);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard,
            commands::refresh,
            commands::fetch_platform_usage,
            commands::get_settings,
            commands::save_settings,
            commands::set_autostart,
            commands::is_autostart_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn dirs_data_dir() -> String {
    std::env::var("APPDATA")
        .map(|p| {
            std::path::Path::new(&p)
                .join("deepseek-monitor")
                .to_string_lossy()
                .to_string()
        })
        .unwrap_or_else(|_| ".".into())
}
