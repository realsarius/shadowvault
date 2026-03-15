use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Sends a native OS notification after a copy job finishes.
/// Only shows on non-manual triggers to avoid spamming during manual runs.
pub fn notify_copy_result(
    app: &AppHandle,
    source_name: &str,
    files_copied: Option<i32>,
    bytes_copied: Option<i64>,
    trigger: &str,
    error: Option<&str>,
) {
    // Skip notifications for manual triggers — user initiated and is watching
    if trigger == "Manual" && error.is_none() {
        return;
    }

    let (title, body) = match error {
        Some(err) => (
            "ShadowVault — Yedekleme başarısız".to_string(),
            format!("{}: {}", source_name, err),
        ),
        None => {
            let files = files_copied.unwrap_or(0);
            let mb = bytes_copied.unwrap_or(0) as f64 / 1_048_576.0;
            let size_str = if mb >= 1.0 {
                format!("{:.1} MB", mb)
            } else {
                format!("{} KB", bytes_copied.unwrap_or(0) / 1024)
            };
            (
                "ShadowVault — Yedekleme tamamlandı".to_string(),
                format!("{}: {} dosya, {}", source_name, files, size_str),
            )
        }
    };

    let _ = app
        .notification()
        .builder()
        .title(&title)
        .body(&body)
        .show();
}
