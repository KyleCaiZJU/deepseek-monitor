use std::sync::Arc;

use chrono::Datelike;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_store::StoreExt;

use crate::api;
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
    #[cfg(target_os = "windows")]
    {
        return std::env::var("USERPROFILE")
            .map(|p| {
                std::path::Path::new(&p)
                    .join("Downloads")
                    .to_string_lossy()
                    .to_string()
            })
            .unwrap_or_else(|_| ".".into());
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::download_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                std::path::Path::new(&home)
                    .join("Downloads")
                    .to_string_lossy()
                    .to_string()
            })
    }
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

    // Fetch usage data from the export ZIP (gives both cost and per-key amount in one request)
    match platform_api::fetch_export_zip(&token, month, year).await {
        Ok((amount_rows, cost_rows)) => {
            let ac = amount_rows.len();
            let cc = cost_rows.len();
            if ac > 0 {
                store.upsert_usage(&amount_rows)?;
            }
            if cc > 0 {
                store.upsert_cost(&cost_rows)?;
            }
            log::info!("Platform export ZIP: upserted {ac} amount + {cc} cost rows");
        }
        Err(e) => {
            log::warn!("Platform export ZIP fetch failed: {e}");
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

    // C13: Verify the save was successful by reading back
    if let Some(saved) = store.get("settings") {
        let verified = serde_json::from_value::<Settings>(saved).is_ok();
        log::info!(
            "Settings saved via plugin-store. Verified: {verified}. api_key set: {}",
            !settings.api_key.is_empty()
        );
        if !verified {
            return Err("Settings save verification failed: could not deserialize saved value".into());
        }
    } else {
        log::warn!("Settings save verification: key 'settings' not found after save");
    }

    // C13: Dual-path persistence -- also write to plain JSON file alongside the DB
    let backup_dir = crate::dirs_data_dir();
    let _ = std::fs::create_dir_all(&backup_dir);
    let backup_path = std::path::Path::new(&backup_dir).join("settings.json");
    match serde_json::to_string_pretty(&settings) {
        Ok(json_str) => {
            if let Err(e) = std::fs::write(&backup_path, &json_str) {
                log::warn!("Failed to write backup settings.json: {e}");
            } else {
                log::info!("Backup settings.json written to {}", backup_path.display());
            }
        }
        Err(e) => log::warn!("Failed to serialize settings for backup: {e}"),
    }

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
