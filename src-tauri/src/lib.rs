use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::Mutex;
use dashmap::DashMap;
use sqlx::SqlitePool;
use tauri::Manager;

pub mod models;
pub mod db;
pub mod commands;
pub mod engine;

use engine::scheduler::Scheduler;

pub struct AppState {
    pub db: Arc<SqlitePool>,
    pub scheduler: Arc<Mutex<Scheduler>>,
    pub running_jobs: Arc<DashMap<String, tokio::task::AbortHandle>>,
    pub paused: Arc<AtomicBool>,
}

pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                // Determine database path
                let db_path = if let Ok(override_path) = std::env::var("SHADOWVAULT_DB_PATH") {
                    override_path
                } else {
                    // Try to get app data directory
                    match app_handle.path().app_data_dir() {
                        Ok(data_dir) => {
                            // Ensure the directory exists
                            if let Err(e) = std::fs::create_dir_all(&data_dir) {
                                log::warn!("Failed to create app data dir: {}, falling back to current dir", e);
                                let cwd = std::env::current_dir()
                                    .unwrap_or_else(|_| std::path::PathBuf::from("."));
                                cwd.join("shadowvault.db").to_string_lossy().to_string()
                            } else {
                                data_dir.join("shadowvault.db").to_string_lossy().to_string()
                            }
                        }
                        Err(e) => {
                            log::warn!("Could not resolve app data dir: {}, using current dir", e);
                            let cwd = std::env::current_dir()
                                .unwrap_or_else(|_| std::path::PathBuf::from("."));
                            cwd.join("shadowvault.db").to_string_lossy().to_string()
                        }
                    }
                };

                log::info!("Using database at: {}", db_path);

                // Initialize database
                let pool = match db::init_db(&db_path).await {
                    Ok(p) => Arc::new(p),
                    Err(e) => {
                        log::error!("Failed to initialize database: {}", e);
                        return;
                    }
                };

                let running_jobs: Arc<DashMap<String, tokio::task::AbortHandle>> =
                    Arc::new(DashMap::new());
                let paused = Arc::new(AtomicBool::new(false));
                let scheduler = Arc::new(Mutex::new(Scheduler::new()));

                // Initial scheduler load
                {
                    let mut sched = scheduler.lock().await;
                    sched
                        .reload_all(
                            pool.clone(),
                            running_jobs.clone(),
                            app_handle.clone(),
                            paused.clone(),
                        )
                        .await;
                }

                let app_state = AppState {
                    db: pool,
                    scheduler,
                    running_jobs,
                    paused,
                };

                app_handle.manage(app_state);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::sources::get_sources,
            commands::sources::create_source,
            commands::sources::update_source,
            commands::sources::delete_source,
            commands::sources::add_destination,
            commands::sources::update_destination,
            commands::sources::delete_destination,
            commands::jobs::run_now,
            commands::jobs::run_source_now,
            commands::jobs::pause_all,
            commands::jobs::resume_all,
            commands::logs::get_logs,
            commands::logs::get_log_count,
            commands::logs::clear_old_logs,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::fs::pick_directory,
            commands::fs::pick_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
