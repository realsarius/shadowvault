use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::Utc;
use tokio::task::AbortHandle;
use dashmap::DashMap;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};

use crate::models::{Source, Destination, Schedule};
use crate::engine::copier::CopyJob;
use crate::db::queries;

pub struct Scheduler {
    tasks: HashMap<String, AbortHandle>,
    level1_tasks: HashMap<String, AbortHandle>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_cron_duration_every_minute_is_under_70s() {
        // "0 * * * * * *" fires every minute — next fire must be ≤ 60s away
        let d = next_cron_duration("0 * * * * * *");
        assert!(d.as_secs() <= 60, "expected ≤ 60s, got {:?}", d);
        assert!(d.as_millis() > 0, "duration must be positive");
    }

    #[test]
    fn test_next_cron_duration_invalid_expression_returns_fallback() {
        let d = next_cron_duration("not_a_cron_expression");
        assert_eq!(d, Duration::from_secs(3600));
    }

    #[test]
    fn test_next_cron_duration_hourly_is_within_one_hour() {
        // "0 0 * * * * *" fires at the start of each hour
        let d = next_cron_duration("0 0 * * * * *");
        assert!(d.as_secs() <= 3600, "expected ≤ 3600s, got {:?}", d);
        assert!(d.as_millis() > 0, "duration must be positive");
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculates how long to sleep until the next cron fire time.
/// Uses local timezone so expressions like "0 9 * * *" fire at 09:00 local time.
fn next_cron_duration(expression: &str) -> Duration {
    use std::str::FromStr;
    match cron::Schedule::from_str(expression) {
        Ok(sched) => match sched.upcoming(chrono::Local).next() {
            Some(next) => {
                let ms = (next - chrono::Local::now()).num_milliseconds();
                if ms > 0 {
                    Duration::from_millis(ms as u64)
                } else {
                    Duration::from_secs(1)
                }
            }
            None => Duration::from_secs(3600),
        },
        Err(e) => {
            log::error!("Invalid cron expression '{}': {}", expression, e);
            Duration::from_secs(3600)
        }
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            tasks: HashMap::new(),
            level1_tasks: HashMap::new(),
        }
    }

    pub fn schedule_destination(
        &mut self,
        dest: Destination,
        source: Source,
        db: Arc<SqlitePool>,
        running_jobs: Arc<DashMap<String, AbortHandle>>,
        app_handle: AppHandle,
        paused: Arc<AtomicBool>,
    ) {
        // Only Interval and Cron schedules get background tasks
        match &dest.schedule {
            Schedule::OnChange | Schedule::Manual => return,
            _ => {}
        }

        let dest_id = dest.id.clone();
        self.cancel(&dest_id);

        let schedule_clone = dest.schedule.clone();
        let last_run_clone = dest.last_run;

        let task = tokio::task::spawn(async move {
            // Determine if we should run immediately or sleep first
            let should_run_immediately = match last_run_clone {
                None => true,
                Some(last_run) => match &schedule_clone {
                    Schedule::Interval { minutes } => {
                        let elapsed = (Utc::now() - last_run).num_seconds();
                        elapsed >= (*minutes as i64 * 60)
                    }
                    Schedule::Cron { expression } => {
                        use std::str::FromStr;
                        match cron::Schedule::from_str(expression) {
                            Ok(sched) => {
                                let last_local = last_run.with_timezone(&chrono::Local);
                                match sched.after(&last_local).next() {
                                    Some(next) => next <= chrono::Local::now(),
                                    None => false,
                                }
                            }
                            Err(_) => false,
                        }
                    }
                    _ => false,
                },
            };

            if !should_run_immediately {
                let initial_sleep = match &schedule_clone {
                    Schedule::Interval { minutes } => {
                        let elapsed = last_run_clone
                            .map(|lr| (Utc::now() - lr).num_seconds())
                            .unwrap_or(0);
                        let remaining = (*minutes as i64 * 60) - elapsed;
                        Duration::from_secs(remaining.max(1) as u64)
                    }
                    Schedule::Cron { expression } => next_cron_duration(expression),
                    _ => return,
                };
                tokio::time::sleep(initial_sleep).await;
            }

            loop {
                // Spin-wait while paused
                while paused.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }

                // Skip if destination is already running
                if running_jobs.contains_key(&dest.id) {
                    log::info!(
                        "Destination {} is already running, skipping scheduled run",
                        dest.id
                    );
                } else {
                    let db_clone = db.clone();
                    let source_clone = source.clone();
                    let dest_clone = dest.clone();
                    let running_jobs_clone = running_jobs.clone();
                    let app_handle_clone = app_handle.clone();
                    let dest_id_inner = dest.id.clone();

                    let sched_src_path = source_clone.path.clone();
                    let sched_dst_path = dest_clone.path.clone();
                    let sched_dest_id_start = dest_id_inner.clone();
                    let sched_ah_start = app_handle_clone.clone();

                    let job_handle = tokio::task::spawn(async move {
                        let _ = sched_ah_start.emit("copy-started", serde_json::json!({
                            "destination_id": sched_dest_id_start,
                            "source_path": sched_src_path,
                            "destination_path": sched_dst_path,
                        }));

                        let trigger = "Scheduled".to_string();
                        let source_name = source_clone.name.clone();
                        let job = CopyJob {
                            source: source_clone,
                            destination: dest_clone,
                            trigger: trigger.clone(),
                            app: Some(app_handle_clone.clone()),
                            backup_level: Some(crate::engine::block::snapshot::BackupLevel::Level0),
                        };

                        match job.execute(db_clone).await {
                            Ok(log_entry) => {
                                log::info!(
                                    "Scheduled copy completed for destination {}",
                                    dest_id_inner
                                );
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
                                log::error!(
                                    "Scheduled copy failed for destination {}: {}",
                                    dest_id_inner,
                                    e
                                );
                                crate::notifications::notify_copy_result(
                                    &app_handle_clone,
                                    &source_name,
                                    None,
                                    None,
                                    &trigger,
                                    Some(&e.to_string()),
                                );
                                crate::tray::set_tray_state(&app_handle_clone, "error");
                                let payload = serde_json::json!({
                                    "destination_id": dest_id_inner,
                                    "error": e.to_string()
                                });
                                let _ = app_handle_clone.emit("copy-error", &payload);
                            }
                        }

                        running_jobs_clone.remove(&dest_id_inner);
                    });

                    running_jobs.insert(dest.id.clone(), job_handle.abort_handle());
                }

                // Sleep until next scheduled run
                let sleep_duration = match &schedule_clone {
                    Schedule::Interval { minutes } => {
                        Duration::from_secs(*minutes as u64 * 60)
                    }
                    Schedule::Cron { expression } => next_cron_duration(expression),
                    _ => break,
                };
                tokio::time::sleep(sleep_duration).await;
            }
        });

        self.tasks.insert(dest_id, task.abort_handle());
    }

    /// Schedules a Level 1 (incremental) backup task for a destination.
    /// Level 1 runs on its own timer but skips if a Level 0 is currently running.
    pub fn schedule_level1(
        &mut self,
        dest: Destination,
        source: Source,
        db: Arc<SqlitePool>,
        running_jobs: Arc<DashMap<String, AbortHandle>>,
        app_handle: AppHandle,
        paused: Arc<AtomicBool>,
    ) {
        let l1_schedule = match &dest.level1_schedule {
            Some(s @ (Schedule::Interval { .. } | Schedule::Cron { .. })) => s.clone(),
            _ => return,
        };

        let dest_id = dest.id.clone();
        self.cancel_level1(&dest_id);

        let l1_type = dest.level1_type.clone();

        let task = tokio::task::spawn(async move {
            // Initial sleep for Level 1
            let initial_sleep = match &l1_schedule {
                Schedule::Interval { minutes } => Duration::from_secs(*minutes as u64 * 60),
                Schedule::Cron { expression } => next_cron_duration(expression),
                _ => return,
            };
            tokio::time::sleep(initial_sleep).await;

            loop {
                // Spin-wait while paused
                while paused.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }

                // Skip if any backup (Level 0 or other) is already running for this dest
                if running_jobs.contains_key(&dest.id) {
                    log::info!(
                        "Level 1 skipped for {} — another backup is running",
                        dest.id
                    );
                } else {
                    let db_clone = db.clone();
                    let db_status = db.clone();
                    let source_clone = source.clone();
                    let dest_clone = dest.clone();
                    let running_jobs_clone = running_jobs.clone();
                    let app_handle_clone = app_handle.clone();
                    let dest_id_inner = dest.id.clone();
                    let dest_id_status = dest.id.clone();
                    let l1_type_clone = l1_type.clone();
                    let l1_schedule_clone = l1_schedule.clone();

                    let sched_src_path = source_clone.path.clone();
                    let sched_dst_path = dest_clone.path.clone();
                    let sched_dest_id_start = dest_id_inner.clone();
                    let sched_ah_start = app_handle_clone.clone();

                    let job_handle = tokio::task::spawn(async move {
                        let _ = sched_ah_start.emit("copy-started", serde_json::json!({
                            "destination_id": sched_dest_id_start,
                            "source_path": sched_src_path,
                            "destination_path": sched_dst_path,
                            "level": "Level1",
                        }));

                        let backup_level = match l1_type_clone.as_str() {
                            "Differential" => crate::engine::block::snapshot::BackupLevel::Level1Differential,
                            _ => crate::engine::block::snapshot::BackupLevel::Level1Cumulative,
                        };

                        let trigger = "Scheduled".to_string();
                        let source_name = source_clone.name.clone();
                        let job = CopyJob {
                            source: source_clone,
                            destination: dest_clone,
                            trigger: trigger.clone(),
                            app: Some(app_handle_clone.clone()),
                            backup_level: Some(backup_level),
                        };

                        match job.execute(db_clone).await {
                            Ok(log_entry) => {
                                log::info!(
                                    "Scheduled Level 1 backup completed for destination {}",
                                    dest_id_inner
                                );
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
                                log::error!(
                                    "Scheduled Level 1 backup failed for destination {}: {}",
                                    dest_id_inner,
                                    e
                                );
                                crate::notifications::notify_copy_result(
                                    &app_handle_clone,
                                    &source_name,
                                    None,
                                    None,
                                    &trigger,
                                    Some(&e.to_string()),
                                );
                                crate::tray::set_tray_state(&app_handle_clone, "error");
                                let payload = serde_json::json!({
                                    "destination_id": dest_id_inner,
                                    "error": e.to_string()
                                });
                                let _ = app_handle_clone.emit("copy-error", &payload);
                            }
                        }

                        // Update level1_last_run and level1_next_run
                        let now = chrono::Utc::now();
                        let l1_next = match &l1_schedule_clone {
                            Schedule::Interval { minutes } => Some(now + chrono::Duration::minutes(*minutes as i64)),
                            Schedule::Cron { expression } => {
                                use std::str::FromStr;
                                cron::Schedule::from_str(expression).ok().and_then(|s| s.after(&now).next())
                            }
                            _ => None,
                        };
                        let _ = queries::update_destination_level1_run_status(
                            &db_status, &dest_id_status, now, l1_next,
                        ).await;

                        running_jobs_clone.remove(&dest_id_inner);
                    });

                    running_jobs.insert(dest.id.clone(), job_handle.abort_handle());
                }

                // Sleep until next Level 1 run
                let sleep_duration = match &l1_schedule {
                    Schedule::Interval { minutes } => Duration::from_secs(*minutes as u64 * 60),
                    Schedule::Cron { expression } => next_cron_duration(expression),
                    _ => break,
                };
                tokio::time::sleep(sleep_duration).await;
            }
        });

        self.level1_tasks.insert(dest_id, task.abort_handle());
    }

    pub fn cancel(&mut self, destination_id: &str) {
        if let Some(handle) = self.tasks.remove(destination_id) {
            handle.abort();
        }
    }

    pub fn cancel_level1(&mut self, destination_id: &str) {
        if let Some(handle) = self.level1_tasks.remove(destination_id) {
            handle.abort();
        }
    }

    pub fn cancel_all(&mut self) {
        for (_, handle) in self.tasks.drain() {
            handle.abort();
        }
        for (_, handle) in self.level1_tasks.drain() {
            handle.abort();
        }
    }

    pub async fn reload_all(
        &mut self,
        db: Arc<SqlitePool>,
        running_jobs: Arc<DashMap<String, AbortHandle>>,
        app_handle: AppHandle,
        paused: Arc<AtomicBool>,
    ) {
        self.cancel_all();

        match queries::get_all_active_destinations(&db).await {
            Ok(pairs) => {
                for (source, dest) in pairs {
                    // Schedule Level 0
                    self.schedule_destination(
                        dest.clone(),
                        source.clone(),
                        db.clone(),
                        running_jobs.clone(),
                        app_handle.clone(),
                        paused.clone(),
                    );

                    // Schedule Level 1 if enabled
                    if dest.level1_enabled {
                        self.schedule_level1(
                            dest,
                            source,
                            db.clone(),
                            running_jobs.clone(),
                            app_handle.clone(),
                            paused.clone(),
                        );
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to reload scheduler destinations: {}", e);
            }
        }
    }
}
