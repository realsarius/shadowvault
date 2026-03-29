use std::collections::BTreeMap;

use chrono::Utc;
use serde::Serialize;
use tauri::Manager;
use tauri::State;

use crate::db::queries;
use crate::AppState;

const DIAGNOSTICS_LOG_LIMIT: i64 = 300;
const DIAGNOSTICS_RECENT_EVENTS: usize = 50;

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
    reliability: ReliabilityStats,
    triage: TriageSummary,
    recent_events: Vec<RecentEvent>,
    logs: Vec<crate::models::LogEntry>,
}

#[derive(Debug, Serialize)]
struct SchedulerSnapshot {
    paused: bool,
    scheduled_level0_tasks: usize,
    scheduled_level1_tasks: usize,
    running_jobs: usize,
    inflight_jobs: usize,
    running_destination_ids: Vec<String>,
    verifying_jobs: usize,
}

#[derive(Debug, Serialize)]
struct LogSummary {
    total: usize,
    failed: usize,
    skipped: usize,
    verified: usize,
    user_action_required: usize,
    error_classes: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize)]
struct ReliabilityStats {
    skip_reasons: BTreeMap<String, usize>,
    retryable_failures: usize,
    hard_failures: usize,
}

#[derive(Debug, Serialize)]
struct TriageSummary {
    critical_error_classes: BTreeMap<String, usize>,
    suggested_next_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RecentEvent {
    id: i64,
    started_at: String,
    ended_at: Option<String>,
    status: String,
    trigger: String,
    destination_id: String,
    backup_level: Option<String>,
    snapshot_id: Option<String>,
    error_code: Option<String>,
    user_action_required: bool,
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
    } else if msg.contains("vault_locked") {
        "vault_locked".to_string()
    } else if msg.contains("concurrency_conflict") || msg.contains("already running") {
        "concurrency_conflict".to_string()
    } else {
        "io_failure".to_string()
    }
}

fn is_user_action_error(code: &str) -> bool {
    matches!(
        code,
        "blocked_path"
            | "missing_snapshot"
            | "wrong_password"
            | "chain_incomplete"
            | "vault_locked"
            | "invalid_input"
    )
}

fn is_retryable_error(code: &str) -> bool {
    matches!(code, "io_failure" | "concurrency_conflict")
}

fn looks_sensitive_key(key: &str) -> bool {
    let k = key.to_lowercase();
    k.contains("token")
        || k.contains("password")
        || k.contains("salt")
        || k.contains("secret")
        || k.contains("access_key")
        || k.contains("refresh_token")
        || k.contains("encrypt_password_enc")
}

fn redact_sensitive_text(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("password")
        || lower.contains("token")
        || lower.contains("salt")
        || lower.contains("secret")
    {
        "[REDACTED_SENSITIVE]".to_string()
    } else {
        raw.to_string()
    }
}

fn redact_log_entry(mut log: crate::models::LogEntry) -> crate::models::LogEntry {
    if let Some(err) = &log.error_message {
        log.error_message = Some(redact_sensitive_text(err));
    }
    log
}

#[tauri::command]
#[specta::specta]
pub async fn export_diagnostics(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let settings_map = queries::get_all_settings(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    let settings = settings_map
        .into_iter()
        .map(|(k, v)| {
            if looks_sensitive_key(&k) {
                (k, "[REDACTED]".to_string())
            } else {
                (k, v)
            }
        })
        .collect::<BTreeMap<_, _>>();

    let raw_logs = queries::get_logs(
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
    let logs = raw_logs
        .iter()
        .cloned()
        .map(redact_log_entry)
        .collect::<Vec<_>>();

    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut verified = 0usize;
    let mut user_action_required = 0usize;
    let mut retryable_failures = 0usize;
    let mut hard_failures = 0usize;
    let mut error_classes = BTreeMap::<String, usize>::new();
    let mut skip_reasons = BTreeMap::<String, usize>::new();
    let mut critical_error_classes = BTreeMap::<String, usize>::new();
    let mut recent_events = Vec::<RecentEvent>::new();

    for log in &logs {
        match log.status.as_str() {
            "Failed" => failed += 1,
            "Skipped" => skipped += 1,
            "Verified" => verified += 1,
            _ => {}
        }

        let mut event_code = None;
        let mut event_user_action = false;
        if let Some(err) = &log.error_message {
            let code = classify_error_code(err);
            event_user_action = is_user_action_error(&code);
            event_code = Some(code.clone());

            *error_classes.entry(code.clone()).or_insert(0) += 1;
            if event_user_action {
                user_action_required += 1;
                *critical_error_classes.entry(code.clone()).or_insert(0) += 1;
            }
            if is_retryable_error(&code) {
                retryable_failures += 1;
            } else if log.status == "Failed" {
                hard_failures += 1;
            }
            if log.status == "Skipped" {
                *skip_reasons.entry(code).or_insert(0) += 1;
            }
        }

        if recent_events.len() < DIAGNOSTICS_RECENT_EVENTS {
            recent_events.push(RecentEvent {
                id: log.id,
                started_at: log.started_at.to_rfc3339(),
                ended_at: log.ended_at.map(|t| t.to_rfc3339()),
                status: log.status.clone(),
                trigger: log.trigger.clone(),
                destination_id: log.destination_id.clone(),
                backup_level: log.backup_level.clone(),
                snapshot_id: log.snapshot_id.clone(),
                error_code: event_code,
                user_action_required: event_user_action,
            });
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
            inflight_jobs: state.inflight_jobs.len(),
            running_destination_ids,
            verifying_jobs: state.verifying_jobs.len(),
        }
    };

    let mut suggested_next_actions = Vec::new();
    if critical_error_classes.contains_key("wrong_password") {
        suggested_next_actions.push(
            "Şifre yönetimini kontrol et ve kullanıcıya şifre doğrulama adımı sun.".to_string(),
        );
    }
    if critical_error_classes.contains_key("missing_snapshot")
        || critical_error_classes.contains_key("chain_incomplete")
    {
        suggested_next_actions.push(
            "Snapshot zincirini doğrula ve restore/verify dry-run çıktısını incele.".to_string(),
        );
    }
    if critical_error_classes.contains_key("blocked_path") {
        suggested_next_actions
            .push("Kullanıcıyı güvenli hedef path seçimine yönlendir (blocked_path).".to_string());
    }
    if suggested_next_actions.is_empty() {
        suggested_next_actions
            .push("Kritik kullanıcı aksiyonu gerektiren hata sınıfı görülmedi.".to_string());
    }

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
            user_action_required,
            error_classes,
        },
        reliability: ReliabilityStats {
            skip_reasons,
            retryable_failures,
            hard_failures,
        },
        triage: TriageSummary {
            critical_error_classes,
            suggested_next_actions,
        },
        recent_events,
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
