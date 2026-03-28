use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::db::queries;
use crate::models::LogEntry;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub async fn get_logs(
    state: State<'_, AppState>,
    source_id: Option<String>,
    destination_id: Option<String>,
    status: Option<String>,
    started_after: Option<String>,
    started_before: Option<String>,
    search_text: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<LogEntry>, String> {
    queries::get_logs(
        &state.db,
        source_id.as_deref(),
        destination_id.as_deref(),
        status.as_deref(),
        started_after.as_deref(),
        started_before.as_deref(),
        search_text.as_deref(),
        limit,
        offset,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_log_count(
    state: State<'_, AppState>,
    source_id: Option<String>,
    destination_id: Option<String>,
    status: Option<String>,
    started_after: Option<String>,
    started_before: Option<String>,
    search_text: Option<String>,
) -> Result<i64, String> {
    queries::get_log_count(
        &state.db,
        source_id.as_deref(),
        destination_id.as_deref(),
        status.as_deref(),
        started_after.as_deref(),
        started_before.as_deref(),
        search_text.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn clear_old_logs(
    state: State<'_, AppState>,
    older_than_days: u32,
) -> Result<u64, String> {
    queries::clear_old_logs(&state.db, older_than_days)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_log_entry(state: State<'_, AppState>, log_id: i64) -> Result<u64, String> {
    queries::delete_log_entry(&state.db, log_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn clear_logs(
    state: State<'_, AppState>,
    source_id: Option<String>,
    destination_id: Option<String>,
    status: Option<String>,
    started_after: Option<String>,
    started_before: Option<String>,
    search_text: Option<String>,
) -> Result<u64, String> {
    queries::clear_logs(
        &state.db,
        source_id.as_deref(),
        destination_id.as_deref(),
        status.as_deref(),
        started_after.as_deref(),
        started_before.as_deref(),
        search_text.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

fn csv_escape(raw: &str) -> String {
    let escaped = raw.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

fn logs_to_csv(logs: &[LogEntry]) -> String {
    let mut out = String::from(
        "id,source_id,destination_id,source_path,destination_path,status,trigger,started_at,ended_at,bytes_copied,files_copied,error_message,checksum,backup_level,snapshot_id\n",
    );

    for log in logs {
        let ended = log.ended_at.map(|v| v.to_rfc3339()).unwrap_or_default();
        let bytes = log.bytes_copied.map(|v| v.to_string()).unwrap_or_default();
        let files = log.files_copied.map(|v| v.to_string()).unwrap_or_default();
        let error = log.error_message.clone().unwrap_or_default();
        let checksum = log.checksum.clone().unwrap_or_default();
        let backup_level = log.backup_level.clone().unwrap_or_default();
        let snapshot_id = log.snapshot_id.clone().unwrap_or_default();

        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            log.id,
            csv_escape(&log.source_id),
            csv_escape(&log.destination_id),
            csv_escape(&log.source_path),
            csv_escape(&log.destination_path),
            csv_escape(&log.status),
            csv_escape(&log.trigger),
            csv_escape(&log.started_at.to_rfc3339()),
            csv_escape(&ended),
            bytes,
            files,
            csv_escape(&error),
            csv_escape(&checksum),
            csv_escape(&backup_level),
            csv_escape(&snapshot_id),
        ));
    }

    out
}

#[tauri::command]
#[specta::specta]
pub async fn export_logs(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    source_id: Option<String>,
    destination_id: Option<String>,
    status: Option<String>,
    started_after: Option<String>,
    started_before: Option<String>,
    search_text: Option<String>,
    format: String,
) -> Result<String, String> {
    let fmt = format.to_lowercase();
    if fmt != "csv" && fmt != "json" {
        return Err("unsupported format".to_string());
    }

    let logs = queries::get_logs(
        &state.db,
        source_id.as_deref(),
        destination_id.as_deref(),
        status.as_deref(),
        started_after.as_deref(),
        started_before.as_deref(),
        search_text.as_deref(),
        None,
        None,
    )
    .await
    .map_err(|e| e.to_string())?;

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let file_name = if fmt == "csv" {
        format!("shadowvault-logs-{}.csv", timestamp)
    } else {
        format!("shadowvault-logs-{}.json", timestamp)
    };
    let filter_name = if fmt == "csv" { "CSV" } else { "JSON" };
    let ext = if fmt == "csv" { "csv" } else { "json" };

    let file_path = app
        .dialog()
        .file()
        .set_file_name(&file_name)
        .add_filter(filter_name, &[ext])
        .blocking_save_file();

    let path = match file_path {
        Some(p) => p,
        None => return Err("cancelled".to_string()),
    };

    let path_str = path
        .as_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());

    let payload = if fmt == "csv" {
        logs_to_csv(&logs)
    } else {
        serde_json::to_string_pretty(&logs).map_err(|e| e.to_string())?
    };

    std::fs::write(&path_str, payload).map_err(|e| e.to_string())?;
    Ok(path_str)
}
