use tauri::{State, AppHandle, Emitter};
use std::time::{Duration, Instant};

use crate::AppState;
use crate::engine::copier::CopyJob;
use crate::db::queries;

const RUN_NOW_COOLDOWN_SECS: u64 = 5;

#[tauri::command]
#[specta::specta]
pub async fn run_now(
    destination_id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    // Check if already running
    if state.running_jobs.contains_key(&destination_id) {
        return Err(format!("Destination {} is already running", destination_id));
    }

    // Rate limit: 5 second cooldown per destination
    if let Some(last) = state.last_manual_run.get(&destination_id) {
        if last.elapsed() < Duration::from_secs(RUN_NOW_COOLDOWN_SECS) {
            return Err(format!(
                "Lütfen {} saniye bekleyin.",
                RUN_NOW_COOLDOWN_SECS - last.elapsed().as_secs()
            ));
        }
    }
    state.last_manual_run.insert(destination_id.clone(), Instant::now());

    // Look up destination and source
    let dest_row = sqlx::query(
        "SELECT id, source_id, path, schedule_json, retention_json, enabled, last_run, last_status, next_run FROM destinations WHERE id = ?"
    )
    .bind(&destination_id)
    .fetch_optional(state.db.as_ref())
    .await
    .map_err(|e| e.to_string())?;

    let dest_row = dest_row.ok_or_else(|| format!("Destination {} not found", destination_id))?;

    use sqlx::Row;
    let source_id: String = dest_row.try_get("source_id").map_err(|e| e.to_string())?;

    let source = queries::get_source_by_id(&state.db, &source_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Source {} not found", source_id))?;

    let dests = queries::get_destinations_for_source(&state.db, &source_id)
        .await
        .map_err(|e| e.to_string())?;

    let dest = dests
        .into_iter()
        .find(|d| d.id == destination_id)
        .ok_or_else(|| format!("Destination {} not found in source", destination_id))?;

    let db = state.db.clone();
    let running_jobs = state.running_jobs.clone();
    let dest_id_clone = destination_id.clone();
    let app_handle_clone = app_handle.clone();

    let src_path = source.path.clone();
    let dst_path = dest.path.clone();
    let dest_id_start = destination_id.clone();
    let app_handle_start = app_handle.clone();

    let job_handle = tokio::task::spawn(async move {
        let _ = app_handle_start.emit("copy-started", serde_json::json!({
            "destination_id": dest_id_start,
            "source_path": src_path,
            "destination_path": dst_path,
        }));

        let trigger = "Manual".to_string();
        let source_name = source.name.clone();
        let job = CopyJob {
            source,
            destination: dest,
            trigger: trigger.clone(),
            app: Some(app_handle_start.clone()),
        };

        match job.execute(db).await {
            Ok(log_entry) => {
                log::info!("Manual copy completed for destination {}", dest_id_clone);
                crate::notifications::notify_copy_result(
                    &app_handle_clone,
                    &source_name,
                    log_entry.files_copied,
                    log_entry.bytes_copied,
                    &trigger,
                    None,
                );
                let _ = app_handle_clone.emit("copy-completed", &log_entry);
            }
            Err(e) => {
                log::error!("Manual copy failed for destination {}: {}", dest_id_clone, e);
                crate::notifications::notify_copy_result(
                    &app_handle_clone,
                    &source_name,
                    None,
                    None,
                    &trigger,
                    Some(&e.to_string()),
                );
                let payload = serde_json::json!({
                    "destination_id": dest_id_clone,
                    "error": e.to_string()
                });
                let _ = app_handle_clone.emit("copy-error", &payload);
            }
        }

        running_jobs.remove(&dest_id_clone);
    });

    state
        .running_jobs
        .insert(destination_id, job_handle.abort_handle());

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn run_source_now(
    source_id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let source = queries::get_source_by_id(&state.db, &source_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Source {} not found", source_id))?;

    let dests = queries::get_destinations_for_source(&state.db, &source_id)
        .await
        .map_err(|e| e.to_string())?;

    for dest in dests {
        if !dest.enabled {
            continue;
        }

        let dest_id = dest.id.clone();

        // Skip if already running
        if state.running_jobs.contains_key(&dest_id) {
            log::info!("Destination {} is already running, skipping run_source_now", dest_id);
            continue;
        }

        // Rate limit: 5 second cooldown per destination
        if let Some(last) = state.last_manual_run.get(&dest_id) {
            if last.elapsed() < Duration::from_secs(RUN_NOW_COOLDOWN_SECS) {
                log::info!("Destination {} rate limited, skipping run_source_now", dest_id);
                continue;
            }
        }
        state.last_manual_run.insert(dest_id.clone(), Instant::now());

        let db = state.db.clone();
        let running_jobs = state.running_jobs.clone();
        let source_clone = source.clone();
        let dest_id_clone = dest_id.clone();
        let app_handle_clone = app_handle.clone();

        let src_path2 = source_clone.path.clone();
        let dst_path2 = dest.path.clone();
        let dest_id_start2 = dest_id_clone.clone();
        let app_handle_start2 = app_handle_clone.clone();

        let job_handle = tokio::task::spawn(async move {
            let _ = app_handle_start2.emit("copy-started", serde_json::json!({
                "destination_id": dest_id_start2,
                "source_path": src_path2,
                "destination_path": dst_path2,
            }));

            let trigger = "Manual".to_string();
            let source_name = source_clone.name.clone();
            let job = CopyJob {
                source: source_clone,
                destination: dest,
                trigger: trigger.clone(),
                app: Some(app_handle_start2.clone()),
            };

            match job.execute(db).await {
                Ok(log_entry) => {
                    log::info!("Source run completed for destination {}", dest_id_clone);
                    crate::notifications::notify_copy_result(
                        &app_handle_clone,
                        &source_name,
                        log_entry.files_copied,
                        log_entry.bytes_copied,
                        &trigger,
                        None,
                    );
                    let _ = app_handle_clone.emit("copy-completed", &log_entry);
                }
                Err(e) => {
                    log::error!("Source run failed for destination {}: {}", dest_id_clone, e);
                    crate::notifications::notify_copy_result(
                        &app_handle_clone,
                        &source_name,
                        None,
                        None,
                        &trigger,
                        Some(&e.to_string()),
                    );
                    let payload = serde_json::json!({
                        "destination_id": dest_id_clone,
                        "error": e.to_string()
                    });
                    let _ = app_handle_clone.emit("copy-error", &payload);
                }
            }

            running_jobs.remove(&dest_id_clone);
        });

        state
            .running_jobs
            .insert(dest_id, job_handle.abort_handle());
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn pause_all(state: State<'_, AppState>, app_handle: AppHandle) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    state.paused.store(true, Ordering::SeqCst);

    let mut scheduler = state.scheduler.lock().await;
    scheduler.cancel_all();

    crate::tray::set_tray_state(&app_handle, "paused");
    log::info!("All scheduled jobs paused");
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn resume_all(
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    state.paused.store(false, Ordering::SeqCst);

    // Reload scheduler from DB
    let db = state.db.clone();
    let running_jobs = state.running_jobs.clone();
    let paused = state.paused.clone();

    let mut scheduler = state.scheduler.lock().await;
    scheduler
        .reload_all(db, running_jobs, app_handle.clone(), paused)
        .await;

    crate::tray::set_tray_state(&app_handle, "normal");
    log::info!("All scheduled jobs resumed");
    Ok(())
}
