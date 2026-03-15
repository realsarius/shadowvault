use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use dashmap::DashMap;
use sqlx::SqlitePool;
use tauri::Manager;

pub mod models;
pub mod db;
pub mod commands;
pub mod engine;
pub mod tray;

use engine::scheduler::Scheduler;
use engine::watcher::FileWatcher;

pub struct AppState {
    pub db: Arc<SqlitePool>,
    pub scheduler: Arc<Mutex<Scheduler>>,
    pub watcher: Arc<Mutex<FileWatcher>>,
    pub running_jobs: Arc<DashMap<String, tokio::task::AbortHandle>>,
    pub paused: Arc<AtomicBool>,
    pub minimize_to_tray: Arc<AtomicBool>,
}

pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Setup system tray
            if let Err(e) = tray::setup_tray(&app_handle) {
                log::warn!("Failed to setup system tray: {}", e);
            }

            tauri::async_runtime::spawn(async move {
                // Determine database path
                let db_path = if let Ok(override_path) = std::env::var("SHADOWVAULT_DB_PATH") {
                    override_path
                } else {
                    match app_handle.path().app_data_dir() {
                        Ok(data_dir) => {
                            if let Err(e) = std::fs::create_dir_all(&data_dir) {
                                log::warn!(
                                    "Failed to create app data dir: {}, falling back to current dir",
                                    e
                                );
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
                let watcher = Arc::new(Mutex::new(FileWatcher::new()));

                // Load minimize_to_tray setting; default true
                let minimize_to_tray = Arc::new(AtomicBool::new(true));
                if let Ok(Some(val)) = db::queries::get_setting(&pool, "minimize_to_tray").await {
                    minimize_to_tray.store(val == "true", Ordering::SeqCst);
                }

                // Start scheduler
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

                // Start file watcher for OnChange destinations
                {
                    let mut w = watcher.lock().await;
                    w.start(pool.clone(), running_jobs.clone(), app_handle.clone()).await;
                }

                let app_state = AppState {
                    db: pool,
                    scheduler,
                    watcher,
                    running_jobs,
                    paused,
                    minimize_to_tray,
                };

                app_handle.manage(app_state);
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let should_minimize = app
                    .try_state::<AppState>()
                    .map(|s| s.minimize_to_tray.load(Ordering::SeqCst))
                    .unwrap_or(true);

                if should_minimize {
                    api.prevent_close();
                    let _ = window.hide();
                    log::info!("Window hidden to tray");
                }
            }
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
            commands::fs::get_disk_info,
            commands::updater::check_update,
            commands::updater::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
