use std::sync::atomic::Ordering;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

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
    let sep3 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[
        &status, &sep1, &run_all, &pause, &sep2, &show, &sep3, &quit,
    ])?;

    TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("ShadowVault")
        .on_menu_event(move |app, event| match event.id.as_ref() {
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

                                    let handle = tokio::task::spawn(async move {
                                        let job = crate::engine::copier::CopyJob {
                                            source,
                                            destination: dest,
                                            trigger: "Manual".to_string(),
                                        };
                                        match job.execute(db2).await {
                                            Ok(entry) => {
                                                let _ = ah.emit("copy-completed", &entry);
                                            }
                                            Err(e) => {
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
                    log::info!("Scheduler {} via tray", if now_paused { "paused" } else { "resumed" });
                }
            }
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
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

    Ok(())
}
