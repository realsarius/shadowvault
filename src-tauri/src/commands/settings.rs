use std::sync::atomic::Ordering;

use tauri::{AppHandle, State};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri_plugin_autostart::ManagerExt;

use crate::AppState;
use crate::db::queries;

/// Reads a single setting value by key. Returns null if not set.
#[tauri::command]
pub async fn get_setting_value(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, String> {
    queries::get_setting(&state.db, &key)
        .await
        .map_err(|e| e.to_string())
}

/// Writes a single setting key/value pair.
#[tauri::command]
pub async fn set_setting_value(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    queries::upsert_setting(&state.db, &key, &value)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub run_on_startup: bool,
    pub minimize_to_tray: bool,
    pub theme: String,
    pub log_retention_days: i64,
    pub language: String,
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let run_on_startup = queries::get_setting(&state.db, "run_on_startup")
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "false".to_string());

    let minimize_to_tray = queries::get_setting(&state.db, "minimize_to_tray")
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "true".to_string());

    let theme = queries::get_setting(&state.db, "theme")
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "dark".to_string());

    let log_retention_days = queries::get_setting(&state.db, "log_retention_days")
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "30".to_string());

    let language = queries::get_setting(&state.db, "language")
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "tr".to_string());

    Ok(AppSettings {
        run_on_startup: run_on_startup.trim() == "true",
        minimize_to_tray: minimize_to_tray.trim() == "true",
        theme,
        log_retention_days: log_retention_days.trim().parse::<i64>().unwrap_or(30),
        language,
    })
}

/// Returns the current schema version recorded in the `schema_versions` table.
#[tauri::command]
pub async fn get_schema_version(state: State<'_, AppState>) -> Result<i64, String> {
    queries::get_schema_version(&state.db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    settings: Value,
) -> Result<(), String> {
    let obj = settings
        .as_object()
        .ok_or_else(|| "Settings must be a JSON object".to_string())?;

    for (key, value) in obj {
        let value_str = match value {
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::Null => "null".to_string(),
            other => other.to_string(),
        };

        queries::upsert_setting(&state.db, key, &value_str)
            .await
            .map_err(|e| e.to_string())?;

        // Sync minimize_to_tray into AppState so window-close handler sees it immediately
        if key == "minimize_to_tray" {
            state
                .minimize_to_tray
                .store(value_str == "true", Ordering::SeqCst);
        }

        // Enable or disable OS autostart
        if key == "run_on_startup" {
            let autolaunch = app_handle.autolaunch();
            if value_str == "true" {
                if let Err(e) = autolaunch.enable() {
                    log::warn!("Failed to enable autostart: {}", e);
                }
            } else if let Err(e) = autolaunch.disable() {
                log::warn!("Failed to disable autostart: {}", e);
            }
        }
    }

    Ok(())
}
