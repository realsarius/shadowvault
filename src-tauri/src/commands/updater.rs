use tauri::{AppHandle, State};
use serde::{Deserialize, Serialize};
use tauri_plugin_updater::UpdaterExt;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub body: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn check_update(app_handle: AppHandle) -> Result<UpdateInfo, String> {
    let updater = app_handle.updater().map_err(|e| e.to_string())?;

    match updater.check().await.map_err(|e| e.to_string())? {
        Some(update) => Ok(UpdateInfo {
            available: true,
            version: Some(update.version.clone()),
            body: update.body.clone(),
        }),
        None => Ok(UpdateInfo {
            available: false,
            version: None,
            body: None,
        }),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn install_update(app_handle: AppHandle, _state: State<'_, AppState>) -> Result<(), String> {
    let updater = app_handle.updater().map_err(|e| e.to_string())?;

    if let Some(update) = updater.check().await.map_err(|e| e.to_string())? {
        update
            .download_and_install(|_chunk, _total| {}, || {})
            .await
            .map_err(|e| e.to_string())?;
        app_handle.restart();
    }

    Ok(())
}
