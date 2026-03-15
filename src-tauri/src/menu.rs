use tauri::{AppHandle, Emitter, Manager};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};

struct Lang {
    // ShadowVault submenu
    about:          &'static str,
    upgrade_pro:    &'static str,
    quit:           &'static str,
    // File / Dosya
    file:           &'static str,
    new_source:     &'static str,
    run_all:        &'static str,
    hide_window:    &'static str,
    // View / Görünüm
    view:           &'static str,
    dashboard:      &'static str,
    sources:        &'static str,
    logs:           &'static str,
    license:        &'static str,
    settings:       &'static str,
    toggle_sidebar: &'static str,
    // Window / Pencere
    window:         &'static str,
    minimize:       &'static str,
    bring_all:      &'static str,
}

const TR: Lang = Lang {
    about:          "ShadowVault Hakkında...",
    upgrade_pro:    "Pro Plana Geç",
    quit:           "ShadowVault'tan Çık",
    file:           "Dosya",
    new_source:     "Yeni Kaynak",
    run_all:        "Hepsini Çalıştır",
    hide_window:    "Pencereyi Gizle",
    view:           "Görünüm",
    dashboard:      "Genel Bakış",
    sources:        "Kaynaklar",
    logs:           "Loglar",
    license:        "Lisans",
    settings:       "Ayarlar",
    toggle_sidebar: "Kenar Çubuğunu Aç/Kapat",
    window:         "Pencere",
    minimize:       "Küçült",
    bring_all:      "Tümünü Öne Getir",
};

const EN: Lang = Lang {
    about:          "About ShadowVault...",
    upgrade_pro:    "Upgrade to Pro",
    quit:           "Quit ShadowVault",
    file:           "File",
    new_source:     "New Source",
    run_all:        "Run All Now",
    hide_window:    "Hide Window",
    view:           "View",
    dashboard:      "Dashboard",
    sources:        "Sources",
    logs:           "Logs",
    license:        "License",
    settings:       "Settings",
    toggle_sidebar: "Toggle Sidebar",
    window:         "Window",
    minimize:       "Minimize",
    bring_all:      "Bring All to Front",
};

pub fn build_menu(app: &AppHandle, lang: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let l: &Lang = if lang == "en" { &EN } else { &TR };

    // ── ShadowVault ──────────────────────────────────────────────────────────
    let about       = MenuItem::with_id(app, "menu_about",   l.about,       true, None::<&str>)?;
    let upgrade     = MenuItem::with_id(app, "menu_upgrade", l.upgrade_pro, true, None::<&str>)?;
    let sep_a1      = PredefinedMenuItem::separator(app)?;
    let hide        = PredefinedMenuItem::hide(app, None)?;
    let hide_others = PredefinedMenuItem::hide_others(app, None)?;
    let show_all    = PredefinedMenuItem::show_all(app, None)?;
    let sep_a2      = PredefinedMenuItem::separator(app)?;
    let quit        = MenuItem::with_id(app, "menu_quit",    l.quit, true, Some("CmdOrCtrl+Q"))?;

    let sv_sub = Submenu::with_items(app, "ShadowVault", true, &[
        &about, &upgrade, &sep_a1,
        &hide, &hide_others, &show_all, &sep_a2,
        &quit,
    ])?;

    // ── Dosya / File ─────────────────────────────────────────────────────────
    let new_src  = MenuItem::with_id(app, "menu_new_source",  l.new_source,  true, Some("CmdOrCtrl+N"))?;
    let sep_f1   = PredefinedMenuItem::separator(app)?;
    let run_all  = MenuItem::with_id(app, "menu_run_all",     l.run_all,     true, Some("CmdOrCtrl+Shift+R"))?;
    let sep_f2   = PredefinedMenuItem::separator(app)?;
    let hide_win = MenuItem::with_id(app, "menu_hide_window", l.hide_window, true, Some("CmdOrCtrl+W"))?;

    let file_sub = Submenu::with_items(app, l.file, true, &[
        &new_src, &sep_f1, &run_all, &sep_f2, &hide_win,
    ])?;

    // ── Görünüm / View ───────────────────────────────────────────────────────
    let nav_dash = MenuItem::with_id(app, "menu_nav_dashboard", l.dashboard, true, Some("CmdOrCtrl+1"))?;
    let nav_src  = MenuItem::with_id(app, "menu_nav_sources",   l.sources,   true, Some("CmdOrCtrl+2"))?;
    let nav_logs = MenuItem::with_id(app, "menu_nav_logs",      l.logs,      true, Some("CmdOrCtrl+3"))?;
    let nav_lic  = MenuItem::with_id(app, "menu_nav_license",   l.license,   true, Some("CmdOrCtrl+4"))?;
    let nav_set  = MenuItem::with_id(app, "menu_nav_settings",  l.settings,  true, Some("CmdOrCtrl+5"))?;
    let sep_v1   = PredefinedMenuItem::separator(app)?;
    let sidebar  = MenuItem::with_id(app, "menu_toggle_sidebar", l.toggle_sidebar, true, Some("CmdOrCtrl+\\"))?;

    let view_sub = Submenu::with_items(app, l.view, true, &[
        &nav_dash, &nav_src, &nav_logs, &nav_lic, &nav_set,
        &sep_v1, &sidebar,
    ])?;

    // ── Pencere / Window ─────────────────────────────────────────────────────
    let minimize  = PredefinedMenuItem::minimize(app, Some(l.minimize))?;
    let sep_w1    = PredefinedMenuItem::separator(app)?;
    let bring_all = MenuItem::with_id(app, "menu_bring_all", l.bring_all, true, None::<&str>)?;

    let win_sub = Submenu::with_items(app, l.window, true, &[
        &minimize, &sep_w1, &bring_all,
    ])?;

    Menu::with_items(app, &[&sv_sub, &file_sub, &view_sub, &win_sub])
}

pub fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "menu_about" => {
            show_main(app);
            let _ = app.emit("show-about", ());
        }
        "menu_upgrade" => {
            show_main(app);
            let _ = app.emit("menu-navigate", "license");
            let _ = app.emit("menu-open-buy-url", ());
        }
        "menu_quit" => {
            let ah = app.clone();
            tauri::async_runtime::spawn(async move {
                crate::tray::graceful_shutdown(&ah).await;
            });
        }
        "menu_new_source" => {
            show_main(app);
            let _ = app.emit("menu-navigate", "sources");
            let _ = app.emit("menu-open-add-source", ());
        }
        "menu_run_all" => {
            let _ = app.emit("menu-run-all", ());
        }
        "menu_hide_window" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.hide();
            }
        }
        "menu_nav_dashboard" => { show_main(app); let _ = app.emit("menu-navigate", "dashboard"); }
        "menu_nav_sources"   => { show_main(app); let _ = app.emit("menu-navigate", "sources"); }
        "menu_nav_logs"      => { show_main(app); let _ = app.emit("menu-navigate", "logs"); }
        "menu_nav_license"   => { show_main(app); let _ = app.emit("menu-navigate", "license"); }
        "menu_nav_settings"  => { show_main(app); let _ = app.emit("menu-navigate", "settings"); }
        "menu_toggle_sidebar" => { let _ = app.emit("menu-toggle-sidebar", ()); }
        "menu_bring_all"     => { show_main(app); }
        _ => {}
    }
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}
