use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            tasks: HashMap::new(),
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
        // Only schedule Interval and Cron schedules automatically
        let interval_secs: u64 = match &dest.schedule {
            Schedule::Interval { minutes } => *minutes as u64 * 60,
            Schedule::Cron { .. } => 60, // check every minute for cron
            Schedule::OnChange | Schedule::Manual => return,
        };

        let dest_id = dest.id.clone();

        // Cancel any existing task for this destination
        self.cancel(&dest_id);

        let schedule_clone = dest.schedule.clone();
        let last_run_clone = dest.last_run;

        let task = tauri::async_runtime::spawn(async move {
            // Determine initial sleep: if last_run is None or overdue, run immediately
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
                            Ok(sched) => match sched.after(&last_run).next() {
                                Some(next) => next <= Utc::now(),
                                None => false,
                            },
                            Err(_) => false,
                        }
                    }
                    _ => false,
                },
            };

            if !should_run_immediately {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
            }

            loop {
                // Check paused flag; spin-wait until unpaused
                while paused.load(Ordering::SeqCst) {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }

                // Check if destination is already running
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

                    let job_handle = tauri::async_runtime::spawn(async move {
                        let job = CopyJob {
                            source: source_clone,
                            destination: dest_clone,
                            trigger: "Scheduled".to_string(),
                        };

                        match job.execute(db_clone).await {
                            Ok(log_entry) => {
                                log::info!(
                                    "Scheduled copy completed for destination {}",
                                    dest_id_inner
                                );
                                let _ = app_handle_clone.emit("copy-completed", &log_entry);
                            }
                            Err(e) => {
                                log::error!(
                                    "Scheduled copy failed for destination {}: {}",
                                    dest_id_inner,
                                    e
                                );
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

                // Sleep for the interval before the next run
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
            }
        });

        self.tasks.insert(dest_id, task.abort_handle());
    }

    pub fn cancel(&mut self, destination_id: &str) {
        if let Some(handle) = self.tasks.remove(destination_id) {
            handle.abort();
        }
    }

    pub fn cancel_all(&mut self) {
        for (_, handle) in self.tasks.drain() {
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
                    self.schedule_destination(
                        dest,
                        source,
                        db.clone(),
                        running_jobs.clone(),
                        app_handle.clone(),
                        paused.clone(),
                    );
                }
            }
            Err(e) => {
                log::error!("Failed to reload scheduler destinations: {}", e);
            }
        }
    }
}
