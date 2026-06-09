mod api;
mod commands;
mod csv_import;
mod store;
mod tray;
mod watcher;

use std::sync::Arc;

use commands::Settings;
use store::Store;
use tauri::{Emitter, Manager};

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

            tray::create_tray(app.handle())?;

            let downloads = std::path::PathBuf::from(&Settings::default().downloads_dir);
            if let Ok((_watcher, rx)) = watcher::start_watching(downloads) {
                let store_clone = store.clone();
                let app_handle = app.handle().clone();
                std::thread::spawn(move || loop {
                    if let Some(event) = watcher::wait_stable(&rx, std::time::Duration::from_millis(500))
                    {
                        let path_str = event.path.to_string_lossy().to_string();
                        if store_clone.was_imported(&path_str).unwrap_or(true) {
                            continue;
                        }
                        let result: Result<csv_import::ImportResult, String> = match event.kind {
                            watcher::CsvKind::Amount => {
                                csv_import::parse_amount_csv(&event.path)
                                    .map(|rows| {
                                        let _ = store_clone.upsert_usage(&rows);
                                        csv_import::ImportResult {
                                            amount_rows: rows.len(),
                                            cost_rows: 0,
                                            skipped_amount: 0,
                                            skipped_cost: 0,
                                        }
                                    })
                            }
                            watcher::CsvKind::Cost => {
                                csv_import::parse_cost_csv(&event.path)
                                    .map(|rows| {
                                        let _ = store_clone.upsert_cost(&rows);
                                        csv_import::ImportResult {
                                            amount_rows: 0,
                                            cost_rows: rows.len(),
                                            skipped_amount: 0,
                                            skipped_cost: 0,
                                        }
                                    })
                            }
                        };
                        if let Ok(imp) = result {
                            let _ = store_clone.log_import(&path_str);
                            let _ = app_handle.emit("csv-imported", imp);
                            if let Ok(dashboard) = store_clone.get_dashboard() {
                                let _ = app_handle.emit("dashboard-updated", dashboard);
                            }
                        }
                    }
                });
            }

            let store_clone = store.clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let settings = Settings::default();
                let interval = settings.interval_min.max(1);
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(interval * 60)).await;

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
                            }
                        }
                        Err(e) => {
                            log::warn!("Timer balance fetch failed: {e}");
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
            commands::import_csv,
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
        .map(|p| format!("{}\\deepseek-monitor", p))
        .unwrap_or_else(|_| ".".into())
}
