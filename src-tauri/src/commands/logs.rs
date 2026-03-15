use tauri::State;

use crate::AppState;
use crate::models::LogEntry;
use crate::db::queries;

#[tauri::command]
pub async fn get_logs(
    state: State<'_, AppState>,
    source_id: Option<String>,
    destination_id: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<LogEntry>, String> {
    queries::get_logs(
        &state.db,
        source_id.as_deref(),
        destination_id.as_deref(),
        status.as_deref(),
        limit,
        offset,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_log_count(
    state: State<'_, AppState>,
    source_id: Option<String>,
) -> Result<i64, String> {
    queries::get_log_count(&state.db, source_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_old_logs(
    state: State<'_, AppState>,
    older_than_days: u32,
) -> Result<u64, String> {
    queries::clear_old_logs(&state.db, older_than_days)
        .await
        .map_err(|e| e.to_string())
}
