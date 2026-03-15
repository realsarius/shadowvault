use tauri::State;
use serde_json::Value;
use uuid::Uuid;
use chrono::Utc;
use std::str::FromStr;
use sqlx::Row;

use crate::AppState;
use crate::models::{Source, Destination, SourceType, JobStatus};
use crate::models::schedule::{Schedule, RetentionPolicy};
use crate::db::queries;

#[tauri::command]
pub async fn get_sources(state: State<'_, AppState>) -> Result<Vec<Source>, String> {
    queries::get_all_sources(&state.db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_source(
    state: State<'_, AppState>,
    name: String,
    path: String,
    source_type: String,
) -> Result<Source, String> {
    let st = SourceType::from_str(&source_type).map_err(|e| e.to_string())?;

    let source = Source {
        id: Uuid::new_v4().to_string(),
        name,
        path,
        source_type: st,
        enabled: true,
        created_at: Utc::now(),
        destinations: vec![],
    };

    queries::insert_source(&state.db, &source)
        .await
        .map_err(|e| e.to_string())?;

    Ok(source)
}

#[tauri::command]
pub async fn update_source(
    state: State<'_, AppState>,
    id: String,
    name: String,
    enabled: bool,
) -> Result<(), String> {
    queries::update_source(&state.db, &id, &name, enabled)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_source(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    // Cancel any scheduled tasks for this source's destinations
    {
        if let Ok(dests) = queries::get_destinations_for_source(&state.db, &id).await {
            let mut scheduler = state.scheduler.lock().await;
            for dest in dests {
                scheduler.cancel(&dest.id);
                state.running_jobs.remove(&dest.id);
            }
        }
    }

    queries::delete_source(&state.db, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_destination(
    state: State<'_, AppState>,
    source_id: String,
    path: String,
    schedule: Value,
    retention: Value,
) -> Result<Destination, String> {
    let schedule: Schedule = serde_json::from_value(schedule).map_err(|e| e.to_string())?;
    let retention: RetentionPolicy =
        serde_json::from_value(retention).map_err(|e| e.to_string())?;

    let dest = Destination {
        id: Uuid::new_v4().to_string(),
        source_id,
        path,
        schedule,
        retention,
        enabled: true,
        last_run: None,
        last_status: None,
        next_run: None,
    };

    queries::insert_destination(&state.db, &dest)
        .await
        .map_err(|e| e.to_string())?;

    Ok(dest)
}

#[tauri::command]
pub async fn update_destination(
    state: State<'_, AppState>,
    id: String,
    path: String,
    schedule: Value,
    retention: Value,
    enabled: bool,
) -> Result<(), String> {
    // Fetch the existing row to preserve source_id and run metadata
    let dest_row = sqlx::query(
        "SELECT id, source_id, last_run, last_status, next_run FROM destinations WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(state.db.as_ref())
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("Destination {} not found", id))?;

    let source_id: String = dest_row.try_get("source_id").map_err(|e| e.to_string())?;
    let last_run_str: Option<String> = dest_row.try_get("last_run").map_err(|e| e.to_string())?;
    let last_status_str: Option<String> =
        dest_row.try_get("last_status").map_err(|e| e.to_string())?;
    let next_run_str: Option<String> =
        dest_row.try_get("next_run").map_err(|e| e.to_string())?;

    let schedule_parsed: Schedule =
        serde_json::from_value(schedule).map_err(|e| e.to_string())?;
    let retention_parsed: RetentionPolicy =
        serde_json::from_value(retention).map_err(|e| e.to_string())?;

    let last_run =
        last_run_str.and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok());
    let last_status =
        last_status_str.and_then(|s| JobStatus::from_str(&s).ok());
    let next_run =
        next_run_str.and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok());

    let dest = Destination {
        id: id.clone(),
        source_id,
        path,
        schedule: schedule_parsed,
        retention: retention_parsed,
        enabled,
        last_run,
        last_status,
        next_run,
    };

    // Cancel existing scheduled task; it will be re-added on next reload
    {
        let mut scheduler = state.scheduler.lock().await;
        scheduler.cancel(&id);
    }

    queries::update_destination(&state.db, &dest)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_destination(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    // Cancel any scheduled task for this destination
    {
        let mut scheduler = state.scheduler.lock().await;
        scheduler.cancel(&id);
        state.running_jobs.remove(&id);
    }

    queries::delete_destination(&state.db, &id)
        .await
        .map_err(|e| e.to_string())
}
