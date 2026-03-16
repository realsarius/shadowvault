use crate::models::{S3Config, SftpConfig, WebDavConfig};
use crate::engine::{cloud_copier, sftp_copier, webdav_copier};

#[tauri::command]
#[specta::specta]
pub async fn test_cloud_connection(
    provider: String,
    bucket: String,
    region: String,
    access_key_id: String,
    secret_access_key: String,
    endpoint_url: Option<String>,
    prefix: String,
) -> Result<(), String> {
    let config = S3Config {
        provider,
        bucket,
        region,
        access_key_id,
        secret_access_key,
        endpoint_url,
        prefix,
    };

    cloud_copier::test_connection(&config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn test_sftp_connection(
    host: String,
    port: u16,
    username: String,
    auth_type: String,
    password: Option<String>,
    private_key: Option<String>,
    remote_path: String,
) -> Result<(), String> {
    let config = SftpConfig {
        host,
        port,
        username,
        auth_type,
        password,
        private_key,
        remote_path,
    };

    tokio::task::spawn_blocking(move || {
        sftp_copier::test_connection_blocking(&config)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn test_webdav_connection(
    url: String,
    username: String,
    password: String,
    root_path: String,
) -> Result<(), String> {
    let config = WebDavConfig { url, username, password, root_path };
    webdav_copier::test_connection(&config)
        .await
        .map_err(|e| e.to_string())
}
