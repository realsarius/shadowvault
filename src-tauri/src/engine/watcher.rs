use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc as tokio_mpsc;
use tokio::task::AbortHandle;

use crate::db::queries;
use crate::engine::copier::CopyJob;
use crate::engine::job_control;
use crate::models::{Destination, Source};

pub struct FileWatcher {
    _watcher: Option<RecommendedWatcher>,
    task_handle: Option<AbortHandle>,
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl FileWatcher {
    pub fn new() -> Self {
        FileWatcher {
            _watcher: None,
            task_handle: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn start(
        &mut self,
        db: Arc<SqlitePool>,
        running_jobs: Arc<DashMap<String, AbortHandle>>,
        inflight_jobs: Arc<DashMap<String, Instant>>,
        app_handle: AppHandle,
        paused: Arc<AtomicBool>,
    ) {
        self.stop();

        let pairs = match queries::get_onchange_destinations(&db).await {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to load OnChange destinations: {}", e);
                return;
            }
        };

        if pairs.is_empty() {
            log::info!("No OnChange destinations found, watcher not started");
            return;
        }

        let mut path_map: HashMap<String, Vec<(Source, Destination)>> = HashMap::new();
        for (source, dest) in pairs {
            path_map
                .entry(source.path.clone())
                .or_default()
                .push((source, dest));
        }

        let (tx, mut rx) = tokio_mpsc::unbounded_channel::<Event>();

        let mut watcher = match notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        }) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Failed to create file watcher: {}", e);
                return;
            }
        };

        for path in path_map.keys() {
            if let Err(e) = watcher.watch(Path::new(path), RecursiveMode::Recursive) {
                let err_str = e.to_string();
                if err_str.contains("inotify")
                    || err_str.contains("No space left")
                    || err_str.contains("too many")
                {
                    log::warn!("inotify limit may be reached: {}", err_str);
                    let _ = app_handle.emit("watcher-warning", serde_json::json!({
                        "message": "Linux inotify izleyici limiti aşıldı. Artırmak için: sudo sysctl -w fs.inotify.max_user_watches=524288"
                    }));
                }
                log::warn!("Failed to watch path {}: {}", path, e);
            } else {
                log::info!("Watching path for changes: {}", path);
            }
        }

        let path_map_len = path_map.len();
        let task = tokio::task::spawn(async move {
            let mut last_triggered: HashMap<String, Instant> = HashMap::new();
            let debounce = Duration::from_millis(500);

            while let Some(event) = rx.recv().await {
                if paused.load(Ordering::SeqCst) {
                    continue;
                }

                let is_relevant = matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_));
                if !is_relevant {
                    continue;
                }

                for (path_str, pairs) in &path_map {
                    let watch_path = Path::new(path_str);
                    let affected = event.paths.iter().any(|p| p.starts_with(watch_path));
                    if !affected {
                        continue;
                    }

                    let now = Instant::now();
                    if let Some(&last_time) = last_triggered.get(path_str) {
                        if now.duration_since(last_time) < debounce {
                            continue;
                        }
                    }
                    last_triggered.insert(path_str.clone(), now);

                    for (source, dest) in pairs {
                        if !dest.enabled {
                            continue;
                        }

                        if !job_control::try_claim_destination(&inflight_jobs, &dest.id) {
                            log::info!(
                                "Destination {} already running, skipping OnChange trigger",
                                dest.id
                            );
                            let db_skip = db.clone();
                            let source_skip = source.clone();
                            let dest_skip = dest.clone();
                            tokio::spawn(async move {
                                let _ = queries::insert_skipped_log_entry(
                                    &db_skip,
                                    &source_skip.id,
                                    &dest_skip.id,
                                    &source_skip.path,
                                    &dest_skip.path,
                                    "OnChange",
                                    "Skipped: destination already has a running job",
                                    None,
                                    None,
                                )
                                .await;
                            });
                            continue;
                        }

                        let db_clone = db.clone();
                        let source_clone = source.clone();
                        let dest_clone = dest.clone();
                        let running_jobs_clone = running_jobs.clone();
                        let inflight_jobs_clone = inflight_jobs.clone();
                        let app_handle_clone = app_handle.clone();
                        let dest_id = dest.id.clone();
                        let dest_id_inner = dest.id.clone();

                        let watch_src_path = source_clone.path.clone();
                        let watch_dst_path = dest_clone.path.clone();
                        let watch_dest_id_start = dest_id_inner.clone();
                        let watch_ah_start = app_handle_clone.clone();

                        job_control::spawn_tracked_job(
                            dest_id,
                            running_jobs_clone,
                            inflight_jobs_clone,
                            async move {
                                let _ = watch_ah_start.emit(
                                    "copy-started",
                                    serde_json::json!({
                                        "destination_id": watch_dest_id_start,
                                        "source_path": watch_src_path,
                                        "destination_path": watch_dst_path,
                                    }),
                                );

                                let job = CopyJob {
                                    source: source_clone,
                                    destination: dest_clone,
                                    trigger: "OnChange".to_string(),
                                    app: Some(watch_ah_start.clone()),
                                    backup_level: None,
                                };
                                match job.execute(db_clone).await {
                                    Ok(log_entry) => {
                                        log::info!(
                                            "OnChange copy completed for destination {}",
                                            dest_id_inner
                                        );
                                        let _ = app_handle_clone.emit("copy-completed", &log_entry);
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "OnChange copy failed for destination {}: {}",
                                            dest_id_inner,
                                            e
                                        );
                                        crate::tray::set_tray_state(&app_handle_clone, "error");
                                        let payload = serde_json::json!({
                                            "destination_id": dest_id_inner,
                                            "error": e.to_string()
                                        });
                                        let _ = app_handle_clone.emit("copy-error", &payload);
                                    }
                                }
                            },
                        );
                    }
                }
            }
        });

        self._watcher = Some(watcher);
        self.task_handle = Some(task.abort_handle());
        log::info!("File watcher started for {} path(s)", path_map_len);
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        self._watcher = None;
    }
}
