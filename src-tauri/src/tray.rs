use std::sync::OnceLock;
use std::sync::atomic::Ordering;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent, TrayIconId},
    AppHandle, Emitter, Manager,
};

use crate::icons_gen::make_tray_rgba;

static TRAY_ID: OnceLock<TrayIconId> = OnceLock::new();

fn make_icon(state: &str) -> tauri::image::Image<'static> {
    let rgba = make_tray_rgba(state);
    // Vec<u8>'i 'static ömürlü hale getiriyoruz (tray ömrü boyunca geçerli)
    let leaked: &'static [u8] = Box::leak(rgba.into_boxed_slice());
    tauri::image::Image::new(leaked, 32, 32)
}

pub fn set_tray_state(app: &AppHandle, state: &str) {
    let Some(id) = TRAY_ID.get() else { return };
    let Some(tray) = app.tray_by_id(id) else { return };
    let _ = tray.set_icon(Some(make_icon(state)));
    let tooltip = match state {
        "paused" => "ShadowVault (Duraklatıldı)",
        "error"  => "ShadowVault (Hata)",
        _        => "ShadowVault",
    };
    let _ = tray.set_tooltip(Some(tooltip));
}

pub async fn graceful_shutdown(app: &AppHandle) {
    use std::time::{Duration, Instant};

    if let Some(state) = app.try_state::<crate::AppState>() {
        let deadline = Instant::now() + Duration::from_secs(30);
        log::info!(
            "Graceful shutdown: waiting for {} running jobs...",
            state.running_jobs.len()
        );

        while !state.running_jobs.is_empty() {
            if Instant::now() >= deadline {
                log::warn!(
                    "Shutdown timeout: aborting {} remaining jobs",
                    state.running_jobs.len()
                );
                let ids: Vec<String> = state.running_jobs.iter().map(|e| e.key().clone()).collect();
                for id in ids {
                    if let Some((_, handle)) = state.running_jobs.remove(&id) {
                        handle.abort();
                    }
                }
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        match crate::db::queries::cancel_running_logs(&state.db, chrono::Utc::now()).await {
            Ok(n) if n > 0 => log::info!("Marked {} running logs as Cancelled on shutdown", n),
            Err(e) => log::warn!("Failed to cancel running logs on shutdown: {}", e),
            _ => {}
        }
    }

    app.exit(0);
}

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "status", "● ShadowVault Aktif", false, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let run_all = MenuItem::with_id(
        app,
        "run_all",
        "Şimdi Hepsini Çalıştır",
        true,
        None::<&str>,
    )?;
    let pause = MenuItem::with_id(app, "pause", "Duraklet", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let show = MenuItem::with_id(app, "show", "Pencereyi Göster", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "Hakkında ShadowVault", true, None::<&str>)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[
        &status, &sep1, &run_all, &pause, &sep2, &show, &about, &sep3, &quit,
    ])?;

    let tray = TrayIconBuilder::new()
        .icon(make_icon("normal"))
        .menu(&menu)
        .tooltip("ShadowVault")
        .on_menu_event(move |app: &AppHandle, event: tauri::menu::MenuEvent| match event.id.as_ref() {
            "run_all" => {
                if let Some(state) = app.try_state::<crate::AppState>() {
                    let db = state.db.clone();
                    let running_jobs = state.running_jobs.clone();
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        match crate::db::queries::get_all_active_destinations(&db).await {
                            Ok(pairs) => {
                                for (source, dest) in pairs {
                                    if running_jobs.contains_key(&dest.id) {
                                        continue;
                                    }
                                    let db2 = db.clone();
                                    let rj = running_jobs.clone();
                                    let ah = app_handle.clone();
                                    let dest_id_spawn = dest.id.clone();
                                    let dest_id_insert = dest.id.clone();
                                    let src_path = source.path.clone();
                                    let dst_path = dest.path.clone();
                                    let ah_start = ah.clone();
                                    let dest_id_start = dest_id_spawn.clone();

                                    let handle = tokio::task::spawn(async move {
                                        let _ = ah_start.emit("copy-started", serde_json::json!({
                                            "destination_id": dest_id_start,
                                            "source_path": src_path,
                                            "destination_path": dst_path,
                                        }));
                                        let job = crate::engine::copier::CopyJob {
                                            source,
                                            destination: dest,
                                            trigger: "Manual".to_string(),
                                            app: Some(ah_start.clone()),
                                        };
                                        match job.execute(db2).await {
                                            Ok(entry) => {
                                                let _ = ah.emit("copy-completed", &entry);
                                            }
                                            Err(e) => {
                                                crate::tray::set_tray_state(&ah, "error");
                                                let _ = ah.emit(
                                                    "copy-error",
                                                    serde_json::json!({
                                                        "destination_id": dest_id_spawn,
                                                        "error": e.to_string()
                                                    }),
                                                );
                                            }
                                        }
                                        rj.remove(&dest_id_spawn);
                                    });

                                    running_jobs.insert(dest_id_insert, handle.abort_handle());
                                }
                            }
                            Err(e) => log::error!("Tray run_all error: {}", e),
                        }
                    });
                }
            }
            "pause" => {
                if let Some(state) = app.try_state::<crate::AppState>() {
                    let was_paused = state.paused.load(Ordering::SeqCst);
                    state.paused.store(!was_paused, Ordering::SeqCst);
                    let now_paused = !was_paused;
                    let _ = app.emit(
                        "scheduler-status",
                        serde_json::json!({ "paused": now_paused }),
                    );
                    set_tray_state(app, if now_paused { "paused" } else { "normal" });
                    log::info!("Scheduler {} via tray", if now_paused { "paused" } else { "resumed" });
                }
            }
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "about" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                let _ = app.emit("show-about", ());
            }
            "quit" => {
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    graceful_shutdown(&app_clone).await;
                });
            }
            _ => {}
        })
        .on_tray_icon_event(|tray: &tauri::tray::TrayIcon, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    let _ = TRAY_ID.set(tray.id().clone());

    Ok(())
}
