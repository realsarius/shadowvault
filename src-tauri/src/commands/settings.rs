use tauri::State;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::AppState;
use crate::db::queries;

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

    let settings = AppSettings {
        run_on_startup: run_on_startup.trim() == "true",
        minimize_to_tray: minimize_to_tray.trim() == "true",
        theme,
        log_retention_days: log_retention_days.trim().parse::<i64>().unwrap_or(30),
        language,
    };

    Ok(settings)
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
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
    }

    Ok(())
}
