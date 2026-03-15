use crate::models::OAuthConfig;
use crate::engine::{oauth_token, oauth_copier};

/// Runs the full OAuth2 PKCE flow:
/// 1. Binds a local port
/// 2. Builds PKCE auth URL
/// 3. Opens the system browser
/// 4. Waits for the callback (max 120 s)
/// 5. Exchanges the code for tokens
/// Returns the resulting OAuthConfig (without folder_path — set via separate field).
#[tauri::command]
pub async fn run_oauth_flow(
    provider:    String,   // "onedrive" | "gdrive"
    client_id:   String,
    folder_path: String,
) -> Result<OAuthConfig, String> {
    // Bind a random local port — keeps it open throughout the flow
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Port bağlanamadı: {}", e))?;
    let port = listener.local_addr()
        .map_err(|e| format!("Port alınamadı: {}", e))?
        .port();

    // Build PKCE session + auth URL
    let session = oauth_token::build_pkce_session(&provider, &client_id, port)
        .map_err(|e| e.to_string())?;

    // Open system browser
    open::that(&session.auth_url)
        .map_err(|e| format!("Tarayıcı açılamadı: {}", e))?;

    // Wait for callback
    let code = oauth_token::await_callback(listener, &session.state)
        .await
        .map_err(|e| e.to_string())?;

    // Exchange code for tokens
    let mut config = oauth_token::exchange_code(
        &provider, &client_id, &code,
        &session.code_verifier, &session.redirect_uri,
    )
    .await
    .map_err(|e| e.to_string())?;

    config.folder_path = if folder_path.is_empty() {
        "/ShadowVault".to_string()
    } else {
        folder_path
    };

    Ok(config)
}

/// Tests that the stored OAuth config can reach the remote drive.
/// Refreshes the token silently if needed.
#[tauri::command]
pub async fn test_oauth_connection(
    oauth_config: serde_json::Value,
) -> Result<(), String> {
    let config: OAuthConfig = serde_json::from_value(oauth_config)
        .map_err(|e| e.to_string())?;
    oauth_copier::test_connection(&config)
        .await
        .map_err(|e| e.to_string())
}
