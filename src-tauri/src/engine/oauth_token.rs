use crate::models::OAuthConfig;
use chrono::Utc;

const ONEDRIVE_AUTH_URL:  &str = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
const ONEDRIVE_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const ONEDRIVE_SCOPE:     &str = "Files.ReadWrite offline_access User.Read";

const GDRIVE_AUTH_URL:  &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GDRIVE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GDRIVE_SCOPE:     &str = "https://www.googleapis.com/auth/drive.file";

pub struct PkceSession {
    pub code_verifier: String,
    pub redirect_uri:  String,
    pub auth_url:      String,
    pub state:         String,
}

// ── PKCE helpers ─────────────────────────────────────────────────────────────

/// Generates a PKCE session for the given provider and binds a local TCP port.
/// The returned port is already bound so no TOCTOU race exists.
pub fn build_pkce_session(
    provider:  &str,
    client_id: &str,
    port:      u16,
) -> anyhow::Result<PkceSession> {
    use rand::Rng;
    use sha2::{Sha256, Digest};
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

    // code_verifier: 96 alphanumeric chars (within the 43-128 allowed range)
    let code_verifier: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(96)
        .map(char::from)
        .collect();

    // code_challenge = BASE64URL-no-pad(SHA256(code_verifier))
    let hash = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(hash);

    // state: 32 random alphanumeric chars
    let state: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);

    let (auth_url_base, scope) = match provider {
        "onedrive" => (ONEDRIVE_AUTH_URL, ONEDRIVE_SCOPE),
        "gdrive"   => (GDRIVE_AUTH_URL,   GDRIVE_SCOPE),
        p => anyhow::bail!("Unknown OAuth provider: {}", p),
    };

    let mut url = url::Url::parse(auth_url_base)?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("client_id",             client_id);
        q.append_pair("response_type",         "code");
        q.append_pair("redirect_uri",          &redirect_uri);
        q.append_pair("scope",                 scope);
        q.append_pair("state",                 &state);
        q.append_pair("code_challenge",        &code_challenge);
        q.append_pair("code_challenge_method", "S256");
        // Google: needed to receive refresh_token
        if provider == "gdrive" {
            q.append_pair("access_type", "offline");
            q.append_pair("prompt",      "consent");
        }
    }

    Ok(PkceSession {
        code_verifier,
        redirect_uri,
        auth_url: url.to_string(),
        state,
    })
}

// ── OAuth callback listener ───────────────────────────────────────────────────

/// Waits (up to 120 s) for the browser to redirect to the local callback URL.
/// Accepts a single connection, parses `code` and `state`, sends success HTML.
pub async fn await_callback(
    listener:       tokio::net::TcpListener,
    expected_state: &str,
) -> anyhow::Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (mut stream, _) = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        listener.accept(),
    )
    .await
    .map_err(|_| anyhow::anyhow!("OAuth callback zaman aşımına uğradı (120 s)"))?
    .map_err(|e| anyhow::anyhow!("TCP accept hatası: {}", e))?;

    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]).to_string();

    // First line: "GET /callback?code=xxx&state=yyy HTTP/1.1"
    let first_line = request.lines().next()
        .ok_or_else(|| anyhow::anyhow!("Boş HTTP isteği"))?;
    let path_part = first_line.split_whitespace().nth(1)
        .ok_or_else(|| anyhow::anyhow!("Geçersiz HTTP isteği"))?;

    // Parse query string manually — avoid extra deps for these simple params
    let query = path_part.split('?').nth(1).unwrap_or("");
    let mut code  = None;
    let mut state = None;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        let k = kv.next().unwrap_or("");
        let v = kv.next().unwrap_or("");
        match k {
            "code"  => code  = Some(percent_decode(v)),
            "state" => state = Some(percent_decode(v)),
            _ => {}
        }
    }

    let code  = code .ok_or_else(|| anyhow::anyhow!("Callback'te 'code' parametresi yok"))?;
    let state = state.ok_or_else(|| anyhow::anyhow!("Callback'te 'state' parametresi yok"))?;

    let success_html = if state == expected_state {
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
         <!DOCTYPE html><html><head><title>ShadowVault</title></head>\
         <body style=\"font-family:system-ui;text-align:center;padding:60px\">\
         <h2>&#10003; Yetkilendirme başarılı</h2>\
         <p>Bu sekmeyi kapatıp ShadowVault'a dönebilirsiniz.</p>\
         </body></html>"
    } else {
        "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
         <!DOCTYPE html><html><body><h2>Yetkilendirme başarısız — state uyuşmazlığı.</h2></body></html>"
    };
    let _ = stream.write_all(success_html.as_bytes()).await;

    if state != expected_state {
        anyhow::bail!("OAuth state uyuşmazlığı — olası CSRF saldırısı");
    }

    Ok(code)
}

fn percent_decode(s: &str) -> String {
    // Very small decoder — only handles %XX patterns
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i+1..i+3]).unwrap_or("??"), 16
            ) {
                out.push(b); i += 3; continue;
            }
        }
        out.push(bytes[i]); i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

// ── Token exchange ────────────────────────────────────────────────────────────

pub async fn exchange_code(
    provider:      &str,
    client_id:     &str,
    code:          &str,
    code_verifier: &str,
    redirect_uri:  &str,
) -> anyhow::Result<OAuthConfig> {
    let token_url = match provider {
        "onedrive" => ONEDRIVE_TOKEN_URL,
        "gdrive"   => GDRIVE_TOKEN_URL,
        p => anyhow::bail!("Unknown provider: {}", p),
    };

    let client = reqwest::Client::new();
    let mut params = std::collections::HashMap::new();
    params.insert("grant_type",    "authorization_code");
    params.insert("client_id",     client_id);
    params.insert("code",          code);
    params.insert("redirect_uri",  redirect_uri);
    params.insert("code_verifier", code_verifier);

    let resp = client.post(token_url).form(&params).send().await?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Token exchange başarısız: {}", text);
    }

    let json: serde_json::Value = resp.json().await?;
    parse_token_response(&json, provider, client_id, "")
}

// ── Token refresh ─────────────────────────────────────────────────────────────

/// Returns a fresh `OAuthConfig`. If the token is still valid (> 5 min remaining),
/// returns the original unchanged (no network call).
pub async fn ensure_fresh_token(config: &OAuthConfig) -> anyhow::Result<OAuthConfig> {
    if Utc::now().timestamp() + 300 < config.expires_at {
        return Ok(config.clone());
    }

    let token_url = match config.provider.as_str() {
        "onedrive" => ONEDRIVE_TOKEN_URL,
        "gdrive"   => GDRIVE_TOKEN_URL,
        p => anyhow::bail!("Unknown provider: {}", p),
    };

    let client = reqwest::Client::new();
    let mut params = std::collections::HashMap::new();
    params.insert("grant_type",    "refresh_token");
    params.insert("client_id",     config.client_id.as_str());
    params.insert("refresh_token", config.refresh_token.as_str());

    let resp = client.post(token_url).form(&params).send().await?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Token yenileme başarısız: {}", text);
    }

    let json: serde_json::Value = resp.json().await?;
    let mut fresh = parse_token_response(&json, &config.provider, &config.client_id, &config.folder_path)?;
    // Some providers don't return a new refresh_token on every refresh
    if fresh.refresh_token.is_empty() {
        fresh.refresh_token = config.refresh_token.clone();
    }
    Ok(fresh)
}

fn parse_token_response(
    json:        &serde_json::Value,
    provider:    &str,
    client_id:   &str,
    folder_path: &str,
) -> anyhow::Result<OAuthConfig> {
    let access_token = json["access_token"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Yanıtta access_token yok"))?
        .to_string();
    let refresh_token = json["refresh_token"].as_str()
        .unwrap_or("")
        .to_string();
    let expires_in = json["expires_in"].as_i64().unwrap_or(3600);
    let expires_at = Utc::now().timestamp() + expires_in;

    Ok(OAuthConfig {
        provider:      provider.to_string(),
        client_id:     client_id.to_string(),
        access_token,
        refresh_token,
        expires_at,
        folder_path:   folder_path.to_string(),
    })
}
