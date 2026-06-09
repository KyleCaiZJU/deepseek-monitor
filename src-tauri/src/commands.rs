use std::sync::Arc;

use chrono::Datelike;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_store::StoreExt;

use crate::api;
use crate::csv_import;
use crate::platform_api;
use crate::store::{Dashboard, Store};

#[tauri::command]
pub async fn get_dashboard(store: State<'_, Arc<Store>>) -> Result<Dashboard, String> {
    store.get_dashboard()
}

#[tauri::command]
pub async fn refresh(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
) -> Result<Dashboard, String> {
    let state = app.try_state::<crate::SettingsState>();
    let settings = match state {
        Some(s) => s.get(),
        None => Settings::default(),
    };
    let api_key = settings.api_key.clone();
    if api_key.is_empty() {
        return Err("API key not set".into());
    }

    match api::fetch_balance(&api_key).await {
        Ok(resp) => {
            if let Some(info) = resp.balance_infos.first() {
                let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
                store.insert_balance(&now, &info.total_balance, resp.is_available)?;
                let bal: f64 = info.total_balance.parse().unwrap_or(0.0);
                let _ = app.emit("balance-updated", bal);
            }
        }
        Err(e) => {
            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
            store.insert_balance(&now, "0", false)?;
            log::warn!("Balance fetch failed: {e}");
        }
    }

    let dashboard = store.get_dashboard()?;
    let _ = app.emit("dashboard-updated", &dashboard);
    Ok(dashboard)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub api_key: String,
    pub platform_token: String,
    pub interval_min: u64,
    pub downloads_dir: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            platform_token: String::new(),
            interval_min: 5,
            downloads_dir: dirs_download(),
        }
    }
}

fn dirs_download() -> String {
    std::env::var("USERPROFILE")
        .map(|p| {
            std::path::Path::new(&p)
                .join("Downloads")
                .to_string_lossy()
                .to_string()
        })
        .unwrap_or_else(|_| ".".into())
}

#[tauri::command]
pub async fn fetch_platform_usage(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
) -> Result<(), String> {
    let state = app.try_state::<crate::SettingsState>();
    let settings = match state {
        Some(s) => s.get(),
        None => Settings::default(),
    };
    let token = settings.platform_token.clone();
    if token.is_empty() {
        return Err("Platform token not set".into());
    }

    let now = chrono::Utc::now();
    let month = now.month();
    let year = now.year();

    // Fetch cost data
    match platform_api::fetch_platform_usage(&token, month, year).await {
        Ok(entries) => {
            let count = entries.len();
            let rows: Vec<csv_import::CostRow> = entries
                .into_iter()
                .map(|e| {
                    let date = chrono::NaiveDate::parse_from_str(&e.utc_date, "%Y-%m-%d")
                        .unwrap_or_else(|_| chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap());
                    let cost: rust_decimal::Decimal = e.cost.parse().unwrap_or_default();
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
            store.upsert_cost(&rows)?;
            log::info!("Platform API: upserted {count} cost rows");
        }
        Err(e) => {
            log::warn!("Platform API cost fetch failed: {e}");
        }
    }

    // Fetch amount (token / request count) data
    match platform_api::fetch_amount(&token, month, year).await {
        Ok(entries) => {
            let count = entries.len();
            let rows: Vec<csv_import::AmountRow> = entries
                .into_iter()
                .map(|e| {
                    let date = chrono::NaiveDate::parse_from_str(&e.utc_date, "%Y-%m-%d")
                        .unwrap_or_else(|_| chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap());
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
            store.upsert_usage(&rows)?;
            log::info!("Platform API: upserted {count} usage rows");
        }
        Err(e) => {
            log::warn!("Platform API amount fetch failed: {e}");
        }
    }

    let dashboard = store.get_dashboard()?;
    let _ = app.emit("dashboard-updated", &dashboard);
    Ok(())
}

#[tauri::command]
pub async fn get_settings(
    app: AppHandle,
) -> Result<Settings, String> {
    let state = app.try_state::<crate::SettingsState>();
    let settings = match state {
        Some(s) => s.get(),
        None => Settings::default(),
    };
    Ok(settings)
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    settings: Settings,
) -> Result<(), String> {
    if let Some(state) = app.try_state::<crate::SettingsState>() {
        state.update(settings.clone());
    }
    // C01: Persist to disk via tauri-plugin-store
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    let value = serde_json::to_value(&settings).map_err(|e| e.to_string())?;
    store.set("settings", value);
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_autostart(
    app: AppHandle,
    on: bool,
) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    if on {
        autostart.enable().map_err(|e| e.to_string())?;
    } else {
        autostart.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn is_autostart_enabled(
    app: AppHandle,
) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    autostart.is_enabled().map_err(|e| e.to_string())
}
