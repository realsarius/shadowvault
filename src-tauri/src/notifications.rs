use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Minimal email format check: non-empty local part, single '@', domain with a dot.
pub(crate) fn is_valid_email(email: &str) -> bool {
    let t = email.trim();
    if t.is_empty() { return false; }
    let mut parts = t.splitn(2, '@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && domain.len() > 2
}

const RESEND_API_KEY: &str = match option_env!("SHADOWVAULT_RESEND_API_KEY") {
    Some(key) => key,
    None => "",
};

const FROM_ADDRESS: &str = "ShadowVault <noreply@mail.berkansozer.com>";
const RESEND_ENDPOINT: &str = "https://api.resend.com/emails";

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

/// Sends a backup result email via Resend if a notification email is configured.
/// Silently logs and returns on any failure — never blocks the backup flow.
pub async fn send_backup_email(
    db: &sqlx::SqlitePool,
    source_name: &str,
    files_copied: Option<i32>,
    bytes_copied: Option<i64>,
    error: Option<&str>,
) {
    let email = match crate::db::queries::get_setting(db, "notification_email").await {
        Ok(Some(e)) if !e.trim().is_empty() => e.trim().to_string(),
        _ => return,
    };

    if RESEND_API_KEY.is_empty() {
        log::warn!("Email notification configured but SHADOWVAULT_RESEND_API_KEY not set at build time");
        return;
    }

    let (subject, html) = build_email_content(source_name, files_copied, bytes_copied, error);

    if let Err(e) = call_resend_api(&email, &subject, &html).await {
        log::warn!("Failed to send backup notification email to {}: {}", email, e);
    } else {
        log::info!("Backup notification email sent to {}", email);
    }
}

/// Sends a test email to verify Resend configuration. Returns error string on failure.
pub async fn send_test_email(to: &str) -> Result<(), String> {
    if RESEND_API_KEY.is_empty() {
        return Err("E-posta API anahtarı derleme zamanında yapılandırılmamış (SHADOWVAULT_RESEND_API_KEY).".to_string());
    }

    let subject = "ShadowVault — Test E-postası";
    let html = "<p>Bu bir test e-postasıdır.</p><p><strong>ShadowVault</strong> e-posta bildirimleri başarıyla yapılandırıldı. Yedekleme tamamlandığında veya hata oluştuğunda bu adrese otomatik bildirim gönderilecek.</p>";

    call_resend_api(to, subject, html)
        .await
        .map_err(|e| e.to_string())
}

fn build_email_content(
    source_name: &str,
    files_copied: Option<i32>,
    bytes_copied: Option<i64>,
    error: Option<&str>,
) -> (String, String) {
    match error {
        Some(err) => (
            format!("ShadowVault — \"{}\" yedeklemesi başarısız", source_name),
            format!(
                "<p><strong>{}</strong> kaynağının yedeklemesi başarısız oldu.</p><p style=\"color:#e53e3e\">Hata: {}</p><hr/><p style=\"color:#888;font-size:12px\">ShadowVault otomatik bildirim</p>",
                source_name, err
            ),
        ),
        None => {
            let files = files_copied.unwrap_or(0);
            let bytes = bytes_copied.unwrap_or(0);
            let size_str = if bytes >= 1_048_576 {
                format!("{:.1} MB", bytes as f64 / 1_048_576.0)
            } else {
                format!("{} KB", bytes / 1024)
            };
            (
                format!("ShadowVault — \"{}\" yedeklemesi tamamlandı", source_name),
                format!(
                    "<p><strong>{}</strong> kaynağının yedeklemesi başarıyla tamamlandı.</p><p>✅ {} dosya &nbsp;•&nbsp; {}</p><hr/><p style=\"color:#888;font-size:12px\">ShadowVault otomatik bildirim</p>",
                    source_name, files, size_str
                ),
            )
        }
    }
}

async fn call_resend_api(to: &str, subject: &str, html: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "from": FROM_ADDRESS,
        "to": [to],
        "subject": subject,
        "html": html,
    });

    let resp = client
        .post(RESEND_ENDPOINT)
        .bearer_auth(RESEND_API_KEY)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Resend API {} — {}", status, text);
    }

    Ok(())
}
