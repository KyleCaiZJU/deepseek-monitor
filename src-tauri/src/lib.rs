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
                                    let tooltip = format!("Balance ¥{:.2}", bal);
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

                            // Fetch cost data
                            match platform_api::fetch_platform_usage(&platform_token, month, year).await
                            {
                                Ok(entries) => {
                                    let count = entries.len();
                                    let rows: Vec<csv_import::CostRow> = entries
                                        .into_iter()
                                        .map(|e| {
                                            let date = chrono::NaiveDate::parse_from_str(
                                                &e.utc_date, "%Y-%m-%d",
                                            )
                                            .unwrap_or_else(|_| {
                                                chrono::NaiveDate::from_ymd_opt(year, month, 1)
                                                    .unwrap()
                                            });
                                            let cost: rust_decimal::Decimal =
                                                e.cost.parse().unwrap_or_default();
                                            csv_import::CostRow {
                                                user_id: String::new(),
                                                utc_date: date,
                                                model: e.model,
                                                wallet_type: String::new(),
                                                cost,
                                                currency: e.currency,
                                            }
                                        })
                                        .collect();
                                    let _ = store_clone.upsert_cost(&rows);
                                    log::info!("Timer: upserted {} platform cost rows", count);
                                }
                                Err(e) => {
                                    log::warn!("Timer platform cost fetch failed: {e}");
                                }
                            }

                            // Fetch amount (token / request count) data
                            match platform_api::fetch_amount(&platform_token, month, year).await {
                                Ok(entries) => {
                                    let count = entries.len();
                                    let rows: Vec<csv_import::AmountRow> = entries
                                        .into_iter()
                                        .map(|e| {
                                            let date = chrono::NaiveDate::parse_from_str(
                                                &e.utc_date, "%Y-%m-%d",
                                            )
                                            .unwrap_or_else(|_| {
                                                chrono::NaiveDate::from_ymd_opt(year, month, 1)
                                                    .unwrap()
                                            });
                                            csv_import::AmountRow {
                                                user_id: String::new(),
                                                utc_date: date,
                                                model: e.model,
                                                api_key_name: "all".to_string(),
                                                api_key: String::new(),
                                                r#type: e.db_type,
                                                price: None,
                                                amount: e.amount,
                                            }
                                        })
                                        .collect();
                                    let _ = store_clone.upsert_usage(&rows);
                                    log::info!("Timer: upserted {} platform usage rows", count);
                                }
                                Err(e) => {
                                    log::warn!("Timer platform amount fetch failed: {e}");
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
