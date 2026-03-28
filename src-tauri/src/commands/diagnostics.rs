use std::collections::BTreeMap;

use chrono::Utc;
use serde::Serialize;
use tauri::Manager;
use tauri::State;

use crate::db::queries;
use crate::AppState;

const DIAGNOSTICS_LOG_LIMIT: i64 = 300;

#[derive(Debug, Serialize, specta::Type)]
pub struct DiagnosticsExportResult {
    pub path: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticsPayload {
    generated_at: String,
    app_version: String,
    scheduler: SchedulerSnapshot,
    settings: BTreeMap<String, String>,
    log_summary: LogSummary,
    logs: Vec<crate::models::LogEntry>,
}

#[derive(Debug, Serialize)]
struct SchedulerSnapshot {
    paused: bool,
    scheduled_level0_tasks: usize,
    scheduled_level1_tasks: usize,
    running_jobs: usize,
    running_destination_ids: Vec<String>,
    verifying_jobs: usize,
}

#[derive(Debug, Serialize)]
struct LogSummary {
    total: usize,
    failed: usize,
    skipped: usize,
    verified: usize,
    error_classes: BTreeMap<String, usize>,
}

fn classify_error_code(raw: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(code) = value.get("error_code").and_then(|v| v.as_str()) {
            return code.to_string();
        }
    }

    let msg = raw.to_lowercase();
    if msg.contains("blocked_path") || msg.contains("sistem dizin") {
        "blocked_path".to_string()
    } else if msg.contains("missing_snapshot") || msg.contains("snapshot") {
        "missing_snapshot".to_string()
    } else if msg.contains("wrong_password") || msg.contains("şifre") || msg.contains("password") {
        "wrong_password".to_string()
    } else if msg.contains("chain_incomplete") || msg.contains("zincir") || msg.contains("chain") {
        "chain_incomplete".to_string()
    } else {
        "io_failure".to_string()
    }
}

#[tauri::command]
#[specta::specta]
pub async fn export_diagnostics(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let mut settings = queries::get_all_settings(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    settings.remove("encrypt_password_enc");
    settings.remove("encrypt_salt");
    let settings = settings.into_iter().collect::<BTreeMap<_, _>>();

    let logs = queries::get_logs(
        &state.db,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(DIAGNOSTICS_LOG_LIMIT),
        Some(0),
    )
    .await
    .map_err(|e| e.to_string())?;

    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut verified = 0usize;
    let mut error_classes = BTreeMap::<String, usize>::new();
    for log in &logs {
        match log.status.as_str() {
            "Failed" => failed += 1,
            "Skipped" => skipped += 1,
            "Verified" => verified += 1,
            _ => {}
        }
        if let Some(err) = &log.error_message {
            let code = classify_error_code(err);
            *error_classes.entry(code).or_insert(0) += 1;
        }
    }

    let scheduler_snapshot = {
        let scheduler = state.scheduler.lock().await;
        let (scheduled_level0_tasks, scheduled_level1_tasks) = scheduler.scheduled_task_counts();
        let running_destination_ids = state
            .running_jobs
            .iter()
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        SchedulerSnapshot {
            paused: state.paused.load(std::sync::atomic::Ordering::SeqCst),
            scheduled_level0_tasks,
            scheduled_level1_tasks,
            running_jobs: running_destination_ids.len(),
            running_destination_ids,
            verifying_jobs: state.verifying_jobs.len(),
        }
    };

    let payload = DiagnosticsPayload {
        generated_at: Utc::now().to_rfc3339(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        scheduler: scheduler_snapshot,
        settings,
        log_summary: LogSummary {
            total: logs.len(),
            failed,
            skipped,
            verified,
            error_classes,
        },
        logs,
    };

    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("diagnostics_exports");
    std::fs::create_dir_all(&base_dir).map_err(|e| e.to_string())?;

    let file_path = base_dir.join(format!(
        "shadowvault-diagnostics-{}.json",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));

    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&file_path, json).map_err(|e| e.to_string())?;

    Ok(file_path.to_string_lossy().to_string())
}
