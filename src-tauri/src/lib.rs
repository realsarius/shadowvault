use dashmap::DashMap;
use sqlx::SqlitePool;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;
use tokio::sync::{watch, Mutex};
use vault::session::SessionStore;

pub mod commands;
pub mod crypto_utils;
pub mod db;
pub mod engine;
pub mod icons_gen;
pub mod menu;
pub mod models;
pub mod notifications;
pub mod tray;
pub mod vault;

use engine::scheduler::Scheduler;
use engine::watcher::FileWatcher;

pub struct AppState {
    pub db: Arc<SqlitePool>,
    pub scheduler: Arc<Mutex<Scheduler>>,
    pub watcher: Arc<Mutex<FileWatcher>>,
    pub running_jobs: Arc<DashMap<String, tokio::task::AbortHandle>>,
    pub inflight_jobs: Arc<DashMap<String, Instant>>,
    pub verifying_jobs: Arc<DashMap<String, Instant>>,
    pub paused: Arc<AtomicBool>,
    pub pause_signal: watch::Sender<bool>,
    pub minimize_to_tray: Arc<AtomicBool>,
    pub last_manual_run: Arc<DashMap<String, Instant>>,
}

pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        commands::sources::get_sources,
        commands::sources::create_source,
        commands::sources::update_source,
        commands::sources::delete_source,
        // add_destination / update_destination excluded: >10 params (specta limit)
        commands::sources::delete_destination,
        commands::jobs::run_now,
        commands::jobs::run_source_now,
        commands::jobs::pause_all,
        commands::jobs::resume_all,
        commands::logs::get_logs,
        commands::logs::get_log_count,
        commands::logs::clear_old_logs,
        commands::logs::delete_log_entry,
        commands::logs::clear_logs,
        commands::logs::export_logs,
        commands::settings::get_settings,
        commands::settings::update_settings,
        commands::settings::get_setting_value,
        commands::settings::set_setting_value,
        commands::settings::get_schema_version,
        commands::fs::pick_directory,
        commands::fs::pick_file,
        commands::fs::get_disk_info,
        commands::fs::check_path_type,
        commands::fs::open_path,
        commands::updater::check_update,
        commands::updater::install_update,
        commands::license::get_hardware_id,
        commands::license::activate_license,
        commands::license::validate_license,
        commands::license::store_license,
        commands::license::get_stored_license,
        commands::license::clear_license,
        commands::license::deactivate_license,
        commands::restore::restore_backup,
        commands::restore::restore_dry_run,
        commands::restore::restore_block_backup,
        commands::restore::restore_block_dry_run,
        commands::restore::verify_backup,
        commands::config::export_config,
        commands::config::import_config,
        commands::notifications::send_test_email,
        commands::preview::preview_backup,
        commands::cloud::test_cloud_connection,
        commands::cloud::test_sftp_connection,
        commands::cloud::test_webdav_connection,
        commands::oauth::run_oauth_flow,
        commands::oauth::test_oauth_connection,
        commands::vault::create_vault,
        commands::vault::list_vaults,
        commands::vault::unlock_vault,
        commands::vault::lock_vault,
        commands::vault::list_entries,
        commands::vault::import_file_cmd,
        commands::vault::import_directory_cmd,
        commands::vault::export_file_cmd,
        commands::vault::open_file_cmd,
        commands::vault::rename_entry_cmd,
        commands::vault::move_entry_cmd,
        commands::vault::delete_entry_cmd,
        commands::vault::create_directory_cmd,
        commands::vault::get_thumbnail,
        commands::vault::delete_vault,
        commands::vault::change_vault_password,
        commands::vault::get_open_files,
        commands::vault::sync_and_lock_vault,
        commands::backup_decrypt::decrypt_backup,
        commands::diagnostics::export_diagnostics,
        rebuild_app_menu,
    ])
}

pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Setup system tray
            if let Err(e) = tray::setup_tray(&app_handle) {
                log::warn!("Failed to setup system tray: {}", e);
            }

            // Register menu event handler once — persists for the app lifetime
            let ah_menu = app_handle.clone();
            app.on_menu_event(move |_app, event| {
                menu::handle_menu_event(&ah_menu, event.id().as_ref());
            });

            // Deep link handler: shadowvault://activate?key=SV-XXXX
            let ah_deeplink = app_handle.clone();
            app.deep_link()
                .on_open_url(move |event: tauri_plugin_deep_link::OpenUrlEvent| {
                    let urls = event.urls();
                    for url in &urls {
                        if url.scheme() != "shadowvault" {
                            continue;
                        }
                        if url.host_str() != Some("activate") {
                            continue;
                        }
                        let key: Option<String> = url
                            .query_pairs()
                            .find(|(k, _)| k == "key")
                            .map(|(_, v)| v.into_owned());
                        if let Some(k) = key {
                            let ah = ah_deeplink.clone();
                            tauri::async_runtime::spawn(async move {
                                handle_deep_link_activate(&ah, k).await;
                            });
                        }
                    }
                });

            // Build initial menu (language resolved async after DB loads)
            if let Ok(m) = menu::build_menu(&app_handle, "tr") {
                let _ = app.set_menu(m);
            }

            tauri::async_runtime::spawn(async move {
                let db_path = if let Ok(p) = std::env::var("SHADOWVAULT_DB_PATH") {
                    p
                } else {
                    match app_handle.path().app_data_dir() {
                        Ok(data_dir) => {
                            if let Err(e) = std::fs::create_dir_all(&data_dir) {
                                log::warn!("Failed to create app data dir: {}", e);
                                std::env::current_dir()
                                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                                    .join("shadowvault.db")
                                    .to_string_lossy()
                                    .to_string()
                            } else {
                                data_dir
                                    .join("shadowvault.db")
                                    .to_string_lossy()
                                    .to_string()
                            }
                        }
                        Err(e) => {
                            log::warn!("Could not resolve app data dir: {}", e);
                            std::env::current_dir()
                                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                                .join("shadowvault.db")
                                .to_string_lossy()
                                .to_string()
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

                // Rebuild menu with the stored language once DB is ready
                let lang = db::queries::get_setting(&pool, "language")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "tr".to_string());
                if lang != "tr" {
                    if let Ok(m) = menu::build_menu(&app_handle, &lang) {
                        let _ = app_handle.set_menu(m);
                    }
                }

                let running_jobs: Arc<DashMap<String, tokio::task::AbortHandle>> =
                    Arc::new(DashMap::new());
                let inflight_jobs: Arc<DashMap<String, Instant>> = Arc::new(DashMap::new());
                let paused = Arc::new(AtomicBool::new(false));
                let (pause_signal, pause_rx) = watch::channel(false);
                let scheduler = Arc::new(Mutex::new(Scheduler::new()));
                let watcher = Arc::new(Mutex::new(FileWatcher::new()));

                let minimize_to_tray = Arc::new(AtomicBool::new(true));
                if let Ok(Some(val)) = db::queries::get_setting(&pool, "minimize_to_tray").await {
                    minimize_to_tray.store(val == "true", Ordering::SeqCst);
                }

                {
                    let mut sched = scheduler.lock().await;
                    sched
                        .reload_all(
                            pool.clone(),
                            running_jobs.clone(),
                            inflight_jobs.clone(),
                            app_handle.clone(),
                            paused.clone(),
                            pause_rx.clone(),
                        )
                        .await;
                }

                {
                    let mut w = watcher.lock().await;
                    w.start(
                        pool.clone(),
                        running_jobs.clone(),
                        inflight_jobs.clone(),
                        app_handle.clone(),
                        paused.clone(),
                    )
                    .await;
                }

                let last_manual_run: Arc<DashMap<String, Instant>> = Arc::new(DashMap::new());
                let verifying_jobs: Arc<DashMap<String, Instant>> = Arc::new(DashMap::new());
                app_handle.manage(AppState {
                    db: pool,
                    scheduler,
                    watcher,
                    running_jobs,
                    inflight_jobs,
                    verifying_jobs,
                    paused,
                    pause_signal,
                    minimize_to_tray,
                    last_manual_run,
                });
                app_handle.manage(Arc::new(SessionStore::new()));
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

                // Pencere gizlenmeden/kapanmadan önce açık vault dosyalarını sync et
                sync_open_vault_files(&app);

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
            commands::logs::delete_log_entry,
            commands::logs::clear_logs,
            commands::logs::export_logs,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::get_setting_value,
            commands::settings::set_setting_value,
            commands::settings::get_schema_version,
            commands::fs::pick_directory,
            commands::fs::pick_file,
            commands::fs::get_disk_info,
            commands::fs::check_path_type,
            commands::fs::open_path,
            commands::updater::check_update,
            commands::updater::install_update,
            commands::license::get_hardware_id,
            commands::license::activate_license,
            commands::license::validate_license,
            commands::license::store_license,
            commands::license::get_stored_license,
            commands::license::clear_license,
            commands::license::deactivate_license,
            commands::restore::restore_backup,
            commands::restore::restore_dry_run,
            commands::restore::restore_block_backup,
            commands::restore::restore_block_dry_run,
            commands::restore::verify_backup,
            commands::config::export_config,
            commands::config::import_config,
            commands::notifications::send_test_email,
            commands::preview::preview_backup,
            commands::cloud::test_cloud_connection,
            commands::cloud::test_sftp_connection,
            commands::cloud::test_webdav_connection,
            commands::oauth::run_oauth_flow,
            commands::oauth::test_oauth_connection,
            commands::vault::create_vault,
            commands::vault::list_vaults,
            commands::vault::unlock_vault,
            commands::vault::lock_vault,
            commands::vault::list_entries,
            commands::vault::import_file_cmd,
            commands::vault::import_directory_cmd,
            commands::vault::export_file_cmd,
            commands::vault::open_file_cmd,
            commands::vault::rename_entry_cmd,
            commands::vault::move_entry_cmd,
            commands::vault::delete_entry_cmd,
            commands::vault::create_directory_cmd,
            commands::vault::get_thumbnail,
            commands::vault::delete_vault,
            commands::vault::change_vault_password,
            commands::vault::get_open_files,
            commands::vault::sync_and_lock_vault,
            commands::backup_decrypt::decrypt_backup,
            commands::diagnostics::export_diagnostics,
            rebuild_app_menu,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Handles `shadowvault://activate?key=SV-XXXX` deep links from LemonSqueezy.
async fn handle_deep_link_activate(app: &tauri::AppHandle, key: String) {
    use commands::license::activate_license_with_key;

    log::info!("Deep link activation for key: {}", &key[..key.len().min(8)]);

    let state = match app.try_state::<AppState>() {
        Some(s) => s,
        None => {
            log::warn!("Deep link received before AppState was ready — ignoring");
            return;
        }
    };

    match activate_license_with_key(&state.db, &key).await {
        Ok(true) => {
            log::info!("Deep link license activation succeeded");
            let _ = app.emit("license-activated", serde_json::json!({ "key": key }));
            // Bring the main window to front
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        Ok(false) => {
            log::warn!("Deep link activation: invalid key");
        }
        Err(e) => {
            log::error!("Deep link activation error: {}", e);
        }
    }
}

/// Called from frontend after a language change so the menu reflects the new locale.
#[tauri::command]
#[specta::specta]
async fn rebuild_app_menu(app: tauri::AppHandle, lang: String) -> Result<(), String> {
    let m = menu::build_menu(&app, &lang).map_err(|e| e.to_string())?;
    app.set_menu(m).map(|_| ()).map_err(|e| e.to_string())
}

/// Pencere kapanmadan / gizlenmeden önce tüm açık vault dosyalarını otomatik kaydet.
/// Bu fonksiyon senkronize I/O yapar (event handler zaten sync bağlamdadır).
fn sync_open_vault_files(app: &tauri::AppHandle) {
    use vault::fs::{reencrypt_from_temp, secure_delete_temp};

    let sess = match app.try_state::<Arc<SessionStore>>() {
        Some(s) => s,
        None => return,
    };

    let (all_files, keys): (Vec<_>, std::collections::HashMap<String, [u8; 32]>) = {
        let guard = match sess.0.lock() {
            Ok(g) => g,
            Err(_) => {
                log::error!("Vault session lock is poisoned during auto-sync");
                return;
            }
        };
        let files = guard.get_all_open_files();
        let keys = files
            .iter()
            .filter_map(|(_, e)| guard.get_key(&e.vault_id).map(|k| (e.vault_id.clone(), k)))
            .collect();
        (files, keys)
    };

    for (tmp_path, entry) in &all_files {
        if let Some(key) = keys.get(&entry.vault_id) {
            let algorithm = vault::fs::VaultMeta::load(&entry.vault_path, key)
                .map(|m| m.algorithm)
                .unwrap_or_else(|_| "AES-256-GCM".to_string());
            if let Err(e) = reencrypt_from_temp(
                &entry.vault_path,
                &entry.entry_id,
                tmp_path,
                key,
                &algorithm,
            ) {
                log::warn!("Vault auto-sync failed for {}: {}", entry.file_name, e);
            }
        }
        secure_delete_temp(tmp_path).ok();
    }

    // Session'ı temizle
    if !all_files.is_empty() {
        let mut guard = match sess.0.lock() {
            Ok(g) => g,
            Err(_) => {
                log::error!("Vault session lock is poisoned while unregistering open files");
                return;
            }
        };
        for (tmp_path, _) in &all_files {
            guard.unregister_open_file(tmp_path);
        }
        log::info!(
            "Vault auto-sync: {} file(s) saved before hide/close",
            all_files.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run `cargo test export_bindings` to regenerate src/generated/bindings.ts
    #[test]
    fn export_bindings() {
        specta_builder()
            .export(
                specta_typescript::Typescript::default()
                    .header("// This file is auto-generated by specta. Do not edit manually.\n")
                    .bigint(specta_typescript::BigIntExportBehavior::Number),
                "../src/generated/bindings.ts",
            )
            .expect("Failed to export TypeScript bindings");
    }
}
