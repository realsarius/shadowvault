use tauri::{AppHandle, State};
use serde_json::Value;
use uuid::Uuid;
use chrono::Utc;
use std::str::FromStr;
use sqlx::Row;

use crate::AppState;
use crate::models::{Source, Destination, SourceType, JobStatus, DestinationType, S3Config, SftpConfig, OAuthConfig, WebDavConfig};
use crate::models::schedule::{Schedule, RetentionPolicy};
use crate::db::queries;

fn encrypt_password_for_storage(password: &str) -> anyhow::Result<(String, String)> {
    use rand::RngCore;
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use sysinfo::System;
    use sha2::{Sha256, Digest};
    use aes_gcm::{aead::{Aead, AeadCore, KeyInit, OsRng}, Aes256Gcm, Key};

    // Generate random 32-byte Argon2id salt
    let mut salt_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut salt_bytes);
    let salt_b64 = BASE64.encode(salt_bytes);

    // Encrypt the password with HW key
    let hw_raw = {
        let mut sys = System::new();
        sys.refresh_memory();
        let hostname = System::host_name().unwrap_or_else(|| "unknown-host".to_string());
        format!("shadowvault:{}:{}:{}", hostname, sys.total_memory(), sys.cpus().len())
    };
    let key_bytes: [u8; 32] = {
        let mut hasher = Sha256::new();
        hasher.update(hw_raw.as_bytes());
        hasher.finalize().into()
    };
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher.encrypt(&nonce, password.as_bytes())
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let mut combined = nonce.to_vec();
    combined.extend(ciphertext);
    let enc = BASE64.encode(combined);

    Ok((enc, salt_b64))
}

#[tauri::command]
#[specta::specta]
pub async fn get_sources(state: State<'_, AppState>) -> Result<Vec<Source>, String> {
    queries::get_all_sources(&state.db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn create_source(
    state: State<'_, AppState>,
    name: String,
    path: String,
    source_type: String,
) -> Result<Source, String> {
    let st = SourceType::from_str(&source_type).map_err(|e| e.to_string())?;

    let source = Source {
        id: Uuid::new_v4().to_string(),
        name,
        path,
        source_type: st,
        enabled: true,
        created_at: Utc::now(),
        destinations: vec![],
    };

    queries::insert_source(&state.db, &source)
        .await
        .map_err(|e| e.to_string())?;

    Ok(source)
}

#[tauri::command]
#[specta::specta]
pub async fn update_source(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    id: String,
    name: String,
    path: String,
    source_type: String,
    enabled: bool,
) -> Result<(), String> {
    queries::update_source(&state.db, &id, &name, &path, &source_type, enabled)
        .await
        .map_err(|e| e.to_string())?;

    // Restart watcher so it picks up the new source path
    let db = state.db.clone();
    let running_jobs = state.running_jobs.clone();
    let mut watcher = state.watcher.lock().await;
    watcher.start(db, running_jobs, app_handle).await;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_source(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    id: String,
) -> Result<(), String> {
    // Cancel any scheduled tasks for this source's destinations
    if let Ok(dests) = queries::get_destinations_for_source(&state.db, &id).await {
        let mut scheduler = state.scheduler.lock().await;
        for dest in dests {
            scheduler.cancel(&dest.id);
            state.running_jobs.remove(&dest.id);
        }
    }

    queries::delete_source(&state.db, &id)
        .await
        .map_err(|e| e.to_string())?;

    // Restart watcher — source removed may have had OnChange destinations
    let db = state.db.clone();
    let running_jobs = state.running_jobs.clone();
    let mut watcher = state.watcher.lock().await;
    watcher.start(db, running_jobs, app_handle).await;

    Ok(())
}

#[tauri::command]
pub async fn add_destination(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    source_id: String,
    path: String,
    schedule: Value,
    retention: Value,
    exclusions: Option<Vec<String>>,
    incremental: Option<bool>,
    destination_type: Option<String>,
    cloud_config: Option<Value>,
    sftp_config: Option<Value>,
    oauth_config: Option<Value>,
    webdav_config: Option<Value>,
    encrypt: Option<bool>,
    encrypt_password: Option<String>,
) -> Result<Destination, String> {
    let schedule: Schedule = serde_json::from_value(schedule).map_err(|e| e.to_string())?;
    let retention: RetentionPolicy =
        serde_json::from_value(retention).map_err(|e| e.to_string())?;

    let dest_type = match destination_type.as_deref() {
        Some("S3")          => DestinationType::S3,
        Some("R2")          => DestinationType::R2,
        Some("Sftp")        => DestinationType::Sftp,
        Some("OneDrive")    => DestinationType::OneDrive,
        Some("GoogleDrive") => DestinationType::GoogleDrive,
        Some("Dropbox")     => DestinationType::Dropbox,
        Some("WebDav")      => DestinationType::WebDav,
        _ => DestinationType::Local,
    };
    let cloud_cfg: Option<S3Config> = if matches!(dest_type, DestinationType::S3 | DestinationType::R2) {
        cloud_config.and_then(|v| serde_json::from_value(v).ok())
    } else { None };
    let sftp_cfg: Option<SftpConfig> = if dest_type == DestinationType::Sftp {
        sftp_config.and_then(|v| serde_json::from_value(v).ok())
    } else { None };
    let oauth_cfg: Option<OAuthConfig> = if matches!(dest_type, DestinationType::OneDrive | DestinationType::GoogleDrive | DestinationType::Dropbox) {
        oauth_config.and_then(|v| serde_json::from_value(v).ok())
    } else { None };
    let webdav_cfg: Option<WebDavConfig> = if dest_type == DestinationType::WebDav {
        webdav_config.and_then(|v| serde_json::from_value(v).ok())
    } else { None };

    let do_encrypt = encrypt.unwrap_or(false);
    let (enc_password, enc_salt) = if do_encrypt {
        match encrypt_password.as_deref() {
            Some(pwd) if !pwd.is_empty() => {
                let (enc, salt) = encrypt_password_for_storage(pwd).map_err(|e| e.to_string())?;
                (Some(enc), Some(salt))
            }
            _ => (None, None),
        }
    } else {
        (None, None)
    };

    let dest = Destination {
        id: Uuid::new_v4().to_string(),
        source_id,
        path,
        schedule,
        retention,
        exclusions: exclusions.unwrap_or_default(),
        enabled: true,
        incremental: incremental.unwrap_or(false),
        last_run: None,
        last_status: None,
        next_run: None,
        destination_type: dest_type,
        cloud_config: cloud_cfg,
        sftp_config: sftp_cfg,
        oauth_config: oauth_cfg,
        webdav_config: webdav_cfg,
        encrypt: do_encrypt,
        encrypt_password_enc: enc_password,
        encrypt_salt: enc_salt,
    };

    queries::insert_destination(&state.db, &dest)
        .await
        .map_err(|e| e.to_string())?;

    // If this is an OnChange destination, restart watcher to pick it up
    if matches!(dest.schedule, Schedule::OnChange) {
        let db = state.db.clone();
        let running_jobs = state.running_jobs.clone();
        let mut watcher = state.watcher.lock().await;
        watcher.start(db, running_jobs, app_handle).await;
    }

    Ok(dest)
}

#[tauri::command]
pub async fn update_destination(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    id: String,
    path: String,
    schedule: Value,
    retention: Value,
    enabled: bool,
    exclusions: Option<Vec<String>>,
    incremental: Option<bool>,
    destination_type: Option<String>,
    cloud_config: Option<Value>,
    sftp_config: Option<Value>,
    oauth_config: Option<Value>,
    webdav_config: Option<Value>,
    encrypt: Option<bool>,
    encrypt_password: Option<String>,
) -> Result<(), String> {
    // Fetch existing row to preserve source_id and run metadata
    let dest_row = sqlx::query(
        "SELECT id, source_id, last_run, last_status, next_run, exclusions_json, incremental, encrypt, encrypt_password_enc, encrypt_salt FROM destinations WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(state.db.as_ref())
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("Destination {} not found", id))?;

    let source_id: String = dest_row.try_get("source_id").map_err(|e| e.to_string())?;
    let last_run_str: Option<String> = dest_row.try_get("last_run").map_err(|e| e.to_string())?;
    let last_status_str: Option<String> =
        dest_row.try_get("last_status").map_err(|e| e.to_string())?;
    let next_run_str: Option<String> =
        dest_row.try_get("next_run").map_err(|e| e.to_string())?;
    let existing_exclusions_json: String =
        dest_row.try_get("exclusions_json").unwrap_or_else(|_| "[]".to_string());
    let existing_incremental: i64 = dest_row.try_get("incremental").unwrap_or(0);
    let existing_encrypt: i64 = dest_row.try_get("encrypt").unwrap_or(0);
    let existing_encrypt_password_enc: Option<String> = dest_row.try_get("encrypt_password_enc").unwrap_or(None);
    let existing_encrypt_salt: Option<String> = dest_row.try_get("encrypt_salt").unwrap_or(None);

    let schedule_parsed: Schedule =
        serde_json::from_value(schedule).map_err(|e| e.to_string())?;
    let retention_parsed: RetentionPolicy =
        serde_json::from_value(retention).map_err(|e| e.to_string())?;

    let last_run =
        last_run_str.and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok());
    let last_status =
        last_status_str.and_then(|s| JobStatus::from_str(&s).ok());
    let next_run =
        next_run_str.and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok());

    // Use provided exclusions or preserve existing ones
    let resolved_exclusions = exclusions.unwrap_or_else(|| {
        serde_json::from_str(&existing_exclusions_json).unwrap_or_default()
    });
    let resolved_incremental = incremental.unwrap_or(existing_incremental != 0);

    let dest_type = match destination_type.as_deref() {
        Some("S3")          => DestinationType::S3,
        Some("R2")          => DestinationType::R2,
        Some("Sftp")        => DestinationType::Sftp,
        Some("OneDrive")    => DestinationType::OneDrive,
        Some("GoogleDrive") => DestinationType::GoogleDrive,
        Some("Dropbox")     => DestinationType::Dropbox,
        Some("WebDav")      => DestinationType::WebDav,
        _ => DestinationType::Local,
    };
    let cloud_cfg: Option<S3Config> = if matches!(dest_type, DestinationType::S3 | DestinationType::R2) {
        cloud_config.and_then(|v| serde_json::from_value(v).ok())
    } else { None };
    let sftp_cfg: Option<SftpConfig> = if dest_type == DestinationType::Sftp {
        sftp_config.and_then(|v| serde_json::from_value(v).ok())
    } else { None };
    let oauth_cfg: Option<OAuthConfig> = if matches!(dest_type, DestinationType::OneDrive | DestinationType::GoogleDrive | DestinationType::Dropbox) {
        oauth_config.and_then(|v| serde_json::from_value(v).ok())
    } else { None };
    let webdav_cfg: Option<WebDavConfig> = if dest_type == DestinationType::WebDav {
        webdav_config.and_then(|v| serde_json::from_value(v).ok())
    } else { None };

    let is_onchange = matches!(schedule_parsed, Schedule::OnChange);

    // Resolve encryption fields: if a new password is provided, re-encrypt; otherwise preserve existing
    let resolved_encrypt = encrypt.unwrap_or(existing_encrypt != 0);
    let (resolved_enc_password, resolved_enc_salt) = if resolved_encrypt {
        match encrypt_password.as_deref() {
            Some(pwd) if !pwd.is_empty() => {
                // New password provided — re-encrypt
                let (enc, salt) = encrypt_password_for_storage(pwd).map_err(|e| e.to_string())?;
                (Some(enc), Some(salt))
            }
            _ => {
                // Keep existing password/salt
                (existing_encrypt_password_enc, existing_encrypt_salt)
            }
        }
    } else {
        (None, None)
    };

    let dest = Destination {
        id: id.clone(),
        source_id,
        path,
        schedule: schedule_parsed,
        retention: retention_parsed,
        exclusions: resolved_exclusions,
        enabled,
        incremental: resolved_incremental,
        last_run,
        last_status,
        next_run,
        destination_type: dest_type,
        cloud_config: cloud_cfg,
        sftp_config: sftp_cfg,
        oauth_config: oauth_cfg,
        webdav_config: webdav_cfg,
        encrypt: resolved_encrypt,
        encrypt_password_enc: resolved_enc_password,
        encrypt_salt: resolved_enc_salt,
    };

    // Cancel existing scheduled task; re-added on next reload
    {
        let mut scheduler = state.scheduler.lock().await;
        scheduler.cancel(&id);
    }

    queries::update_destination(&state.db, &dest)
        .await
        .map_err(|e| e.to_string())?;

    // Restart watcher if schedule involves OnChange
    if is_onchange {
        let db = state.db.clone();
        let running_jobs = state.running_jobs.clone();
        let mut watcher = state.watcher.lock().await;
        watcher.start(db, running_jobs, app_handle).await;
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_destination(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    id: String,
) -> Result<(), String> {
    // Check if it was OnChange before deleting
    let was_onchange = sqlx::query("SELECT schedule_json FROM destinations WHERE id = ?")
        .bind(&id)
        .fetch_optional(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .map(|row| {
            let json: String = row.try_get("schedule_json").unwrap_or_default();
            serde_json::from_str::<Schedule>(&json)
                .map(|s| matches!(s, Schedule::OnChange))
                .unwrap_or(false)
        })
        .unwrap_or(false);

    // Cancel scheduled task
    {
        let mut scheduler = state.scheduler.lock().await;
        scheduler.cancel(&id);
        state.running_jobs.remove(&id);
    }

    queries::delete_destination(&state.db, &id)
        .await
        .map_err(|e| e.to_string())?;

    // Restart watcher if it was an OnChange destination
    if was_onchange {
        let db = state.db.clone();
        let running_jobs = state.running_jobs.clone();
        let mut watcher = state.watcher.lock().await;
        watcher.start(db, running_jobs, app_handle).await;
    }

    Ok(())
}
