use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::api;
use crate::csv_import;
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

#[tauri::command]
pub async fn import_csv(
    store: State<'_, Arc<Store>>,
    path: String,
) -> Result<csv_import::ImportResult, String> {
    use std::path::Path;
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("File not found: {path}"));
    }

    let file_name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path);

    let mut amount_rows = 0usize;
    let mut cost_rows = 0usize;

    if file_name.starts_with("amount-") {
        let rows = csv_import::parse_amount_csv(p)?;
        amount_rows = rows.len();
        store.upsert_usage(&rows)?;
        store.log_import(&path)?;
    } else if file_name.starts_with("cost-") {
        let rows = csv_import::parse_cost_csv(p)?;
        cost_rows = rows.len();
        store.upsert_cost(&rows)?;
        store.log_import(&path)?;
    } else {
        let parent = p.parent().unwrap_or(Path::new("."));
        let name = p.file_stem().and_then(|n| n.to_str()).unwrap_or("");
        let amount_path = parent.join(format!("amount-{}.csv", name));
        let cost_path = parent.join(format!("cost-{}.csv", name));

        if amount_path.exists() {
            let rows = csv_import::parse_amount_csv(&amount_path)?;
            amount_rows = rows.len();
            store.upsert_usage(&rows)?;
            store.log_import(amount_path.to_str().unwrap_or(""))?;
        }
        if cost_path.exists() {
            let rows = csv_import::parse_cost_csv(&cost_path)?;
            cost_rows = rows.len();
            store.upsert_cost(&rows)?;
            store.log_import(cost_path.to_str().unwrap_or(""))?;
        }
    }

    Ok(csv_import::ImportResult {
        amount_rows,
        cost_rows,
        skipped_amount: 0,
        skipped_cost: 0,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub api_key: String,
    pub interval_min: u64,
    pub downloads_dir: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            interval_min: 5,
            downloads_dir: dirs_download(),
        }
    }
}

fn dirs_download() -> String {
    std::env::var("USERPROFILE")
        .map(|p| format!("{}\\Downloads", p))
        .unwrap_or_else(|_| ".".into())
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
        state.update(settings);
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
