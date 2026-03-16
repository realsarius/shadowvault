use sqlx::SqlitePool;
use chrono::{DateTime, Utc};
use crate::models::{Source, Destination, SourceType, JobStatus, DestinationType, S3Config, SftpConfig, OAuthConfig, WebDavConfig};
use crate::models::schedule::{Schedule, RetentionPolicy};
use std::str::FromStr;

fn encrypt_cloud_config(plaintext: &str) -> anyhow::Result<String> {
    use sysinfo::System;
    use sha2::{Sha256, Digest};
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use aes_gcm::{aead::{Aead, AeadCore, KeyInit, OsRng}, Aes256Gcm, Key};

    let hw_raw = {
        let mut sys = System::new();
        sys.refresh_memory();
        let hostname = System::host_name().unwrap_or_else(|| "unknown-host".to_string());
        let total_memory = sys.total_memory();
        let cpu_count = sys.cpus().len();
        format!("shadowvault:{}:{}:{}", hostname, total_memory, cpu_count)
    };
    let key_bytes: [u8; 32] = {
        let mut hasher = Sha256::new();
        hasher.update(hw_raw.as_bytes());
        hasher.finalize().into()
    };
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let mut combined = nonce.to_vec();
    combined.extend(ciphertext);
    Ok(BASE64.encode(combined))
}

fn decrypt_cloud_config(enc: &str) -> Option<S3Config> {
    use sysinfo::System;
    use sha2::{Sha256, Digest};
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};

    let hw_raw = {
        let mut sys = System::new();
        sys.refresh_memory();
        let hostname = System::host_name().unwrap_or_else(|| "unknown-host".to_string());
        let total_memory = sys.total_memory();
        let cpu_count = sys.cpus().len();
        format!("shadowvault:{}:{}:{}", hostname, total_memory, cpu_count)
    };

    let key_bytes: [u8; 32] = {
        let mut hasher = Sha256::new();
        hasher.update(hw_raw.as_bytes());
        hasher.finalize().into()
    };

    let combined = BASE64.decode(enc).ok()?;
    if combined.len() < 13 { return None; }
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&combined[..12]);
    let plaintext = cipher.decrypt(nonce, &combined[12..]).ok()?;
    let json = String::from_utf8(plaintext).ok()?;
    serde_json::from_str::<S3Config>(&json).ok()
}

fn decrypt_sftp_config(enc: &str) -> Option<SftpConfig> {
    use sysinfo::System;
    use sha2::{Sha256, Digest};
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};

    let hw_raw = {
        let mut sys = System::new();
        sys.refresh_memory();
        let hostname = System::host_name().unwrap_or_else(|| "unknown-host".to_string());
        let total_memory = sys.total_memory();
        let cpu_count = sys.cpus().len();
        format!("shadowvault:{}:{}:{}", hostname, total_memory, cpu_count)
    };
    let key_bytes: [u8; 32] = {
        let mut hasher = Sha256::new();
        hasher.update(hw_raw.as_bytes());
        hasher.finalize().into()
    };
    let combined = BASE64.decode(enc).ok()?;
    if combined.len() < 13 { return None; }
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&combined[..12]);
    let plaintext = cipher.decrypt(nonce, &combined[12..]).ok()?;
    let json = String::from_utf8(plaintext).ok()?;
    serde_json::from_str::<SftpConfig>(&json).ok()
}

fn decrypt_oauth_config(enc: &str) -> Option<OAuthConfig> {
    use sysinfo::System;
    use sha2::{Sha256, Digest};
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};

    let hw_raw = {
        let mut sys = System::new();
        sys.refresh_memory();
        let hostname = System::host_name().unwrap_or_else(|| "unknown-host".to_string());
        let total_memory = sys.total_memory();
        let cpu_count = sys.cpus().len();
        format!("shadowvault:{}:{}:{}", hostname, total_memory, cpu_count)
    };
    let key_bytes: [u8; 32] = {
        let mut hasher = Sha256::new();
        hasher.update(hw_raw.as_bytes());
        hasher.finalize().into()
    };
    let combined = BASE64.decode(enc).ok()?;
    if combined.len() < 13 { return None; }
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&combined[..12]);
    let plaintext = cipher.decrypt(nonce, &combined[12..]).ok()?;
    let json = String::from_utf8(plaintext).ok()?;
    serde_json::from_str::<OAuthConfig>(&json).ok()
}

fn decrypt_webdav_config(enc: &str) -> Option<WebDavConfig> {
    use sysinfo::System;
    use sha2::{Sha256, Digest};
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};

    let hw_raw = {
        let mut sys = System::new();
        sys.refresh_memory();
        let hostname = System::host_name().unwrap_or_else(|| "unknown-host".to_string());
        let total_memory = sys.total_memory();
        let cpu_count = sys.cpus().len();
        format!("shadowvault:{}:{}:{}", hostname, total_memory, cpu_count)
    };
    let key_bytes: [u8; 32] = {
        let mut hasher = Sha256::new();
        hasher.update(hw_raw.as_bytes());
        hasher.finalize().into()
    };
    let combined = BASE64.decode(enc).ok()?;
    if combined.len() < 13 { return None; }
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&combined[..12]);
    let plaintext = cipher.decrypt(nonce, &combined[12..]).ok()?;
    let json = String::from_utf8(plaintext).ok()?;
    serde_json::from_str::<WebDavConfig>(&json).ok()
}

fn parse_destination_type(s: &str) -> DestinationType {
    match s {
        "S3" => DestinationType::S3,
        "R2" => DestinationType::R2,
        "Sftp" => DestinationType::Sftp,
        "OneDrive" => DestinationType::OneDrive,
        "GoogleDrive" => DestinationType::GoogleDrive,
        "Dropbox" => DestinationType::Dropbox,
        "WebDav" => DestinationType::WebDav,
        _ => DestinationType::Local,
    }
}

fn destination_type_str(dt: &DestinationType) -> &'static str {
    match dt {
        DestinationType::S3 => "S3",
        DestinationType::R2 => "R2",
        DestinationType::Sftp => "Sftp",
        DestinationType::OneDrive => "OneDrive",
        DestinationType::GoogleDrive => "GoogleDrive",
        DestinationType::Dropbox => "Dropbox",
        DestinationType::WebDav => "WebDav",
        DestinationType::Local => "Local",
    }
}

pub async fn get_all_sources(pool: &SqlitePool) -> anyhow::Result<Vec<Source>> {
    let rows = sqlx::query(
        "SELECT id, name, path, source_type, enabled, created_at FROM sources ORDER BY created_at ASC"
    )
    .fetch_all(pool)
    .await?;

    let mut sources = Vec::new();
    for row in rows {
        use sqlx::Row;
        let id: String = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let path: String = row.try_get("path")?;
        let source_type_str: String = row.try_get("source_type")?;
        let enabled_int: i64 = row.try_get("enabled")?;
        let created_at_str: String = row.try_get("created_at")?;

        let source_type = SourceType::from_str(&source_type_str)?;
        let enabled = enabled_int != 0;
        let created_at = created_at_str
            .parse::<DateTime<Utc>>()
            .unwrap_or_else(|_| Utc::now());

        let destinations = get_destinations_for_source(pool, &id).await?;

        sources.push(Source {
            id,
            name,
            path,
            source_type,
            enabled,
            created_at,
            destinations,
        });
    }

    Ok(sources)
}

pub async fn get_source_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Source>> {
    let row = sqlx::query(
        "SELECT id, name, path, source_type, enabled, created_at FROM sources WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        None => Ok(None),
        Some(row) => {
            use sqlx::Row;
            let id: String = row.try_get("id")?;
            let name: String = row.try_get("name")?;
            let path: String = row.try_get("path")?;
            let source_type_str: String = row.try_get("source_type")?;
            let enabled_int: i64 = row.try_get("enabled")?;
            let created_at_str: String = row.try_get("created_at")?;

            let source_type = SourceType::from_str(&source_type_str)?;
            let enabled = enabled_int != 0;
            let created_at = created_at_str
                .parse::<DateTime<Utc>>()
                .unwrap_or_else(|_| Utc::now());

            let destinations = get_destinations_for_source(pool, &id).await?;

            Ok(Some(Source {
                id,
                name,
                path,
                source_type,
                enabled,
                created_at,
                destinations,
            }))
        }
    }
}

pub async fn insert_source(pool: &SqlitePool, source: &Source) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO sources (id, name, path, source_type, enabled, created_at) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&source.id)
    .bind(&source.name)
    .bind(&source.path)
    .bind(source.source_type.to_string())
    .bind(if source.enabled { 1i64 } else { 0i64 })
    .bind(source.created_at.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_source(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    path: &str,
    source_type: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE sources SET name = ?, path = ?, source_type = ?, enabled = ? WHERE id = ?")
        .bind(name)
        .bind(path)
        .bind(source_type)
        .bind(if enabled { 1i64 } else { 0i64 })
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_source(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM sources WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_destination_by_id(pool: &SqlitePool, dest_id: &str) -> anyhow::Result<Option<Destination>> {
    let row = sqlx::query(
        "SELECT id, source_id, path, schedule_json, retention_json, exclusions_json, enabled, incremental, last_run, last_status, next_run, destination_type, cloud_config_json, encrypt, encrypt_password_enc, encrypt_salt
         FROM destinations WHERE id = ?"
    )
    .bind(dest_id)
    .fetch_optional(pool)
    .await?;

    match row {
        None => Ok(None),
        Some(row) => {
            use sqlx::Row;
            let id: String = row.try_get("id")?;
            let source_id: String = row.try_get("source_id")?;
            let path: String = row.try_get("path")?;
            let schedule_json: String = row.try_get("schedule_json")?;
            let retention_json: String = row.try_get("retention_json")?;
            let exclusions_json: String = row.try_get("exclusions_json").unwrap_or_else(|_| "[]".to_string());
            let enabled_int: i64 = row.try_get("enabled")?;
            let incremental_int: i64 = row.try_get("incremental").unwrap_or(0);
            let last_run_str: Option<String> = row.try_get("last_run")?;
            let last_status_str: Option<String> = row.try_get("last_status")?;
            let next_run_str: Option<String> = row.try_get("next_run")?;
            let destination_type_str_val: String = row.try_get("destination_type").unwrap_or_else(|_| "Local".to_string());
            let cloud_config_json_enc: Option<String> = row.try_get("cloud_config_json").unwrap_or(None);

            let schedule: crate::models::schedule::Schedule = serde_json::from_str(&schedule_json)?;
            let retention: crate::models::schedule::RetentionPolicy = serde_json::from_str(&retention_json)?;
            let exclusions: Vec<String> = serde_json::from_str(&exclusions_json).unwrap_or_default();
            let last_run = last_run_str.and_then(|s| s.parse::<DateTime<Utc>>().ok());
            let last_status = last_status_str.and_then(|s| JobStatus::from_str(&s).ok());
            let next_run = next_run_str.and_then(|s| s.parse::<DateTime<Utc>>().ok());
            let destination_type = parse_destination_type(&destination_type_str_val);
            let cloud_config = if matches!(destination_type, DestinationType::S3 | DestinationType::R2) { cloud_config_json_enc.as_deref().and_then(decrypt_cloud_config) } else { None };
            let sftp_config = if destination_type == DestinationType::Sftp { cloud_config_json_enc.as_deref().and_then(decrypt_sftp_config) } else { None };
            let oauth_config = if matches!(destination_type, DestinationType::OneDrive | DestinationType::GoogleDrive | DestinationType::Dropbox) { cloud_config_json_enc.as_deref().and_then(decrypt_oauth_config) } else { None };
            let webdav_config = if destination_type == DestinationType::WebDav { cloud_config_json_enc.as_deref().and_then(decrypt_webdav_config) } else { None };
            let encrypt_int: i64 = row.try_get("encrypt").unwrap_or(0);
            let encrypt_password_enc: Option<String> = row.try_get("encrypt_password_enc").unwrap_or(None);
            let encrypt_salt: Option<String> = row.try_get("encrypt_salt").unwrap_or(None);

            Ok(Some(Destination {
                id,
                source_id,
                path,
                schedule,
                retention,
                exclusions,
                enabled: enabled_int != 0,
                incremental: incremental_int != 0,
                last_run,
                last_status,
                next_run,
                destination_type,
                cloud_config,
                sftp_config,
                oauth_config,
                webdav_config,
                encrypt: encrypt_int != 0,
                encrypt_password_enc,
                encrypt_salt,
            }))
        }
    }
}

pub async fn insert_destination(pool: &SqlitePool, dest: &Destination) -> anyhow::Result<()> {
    let schedule_json = serde_json::to_string(&dest.schedule)?;
    let retention_json = serde_json::to_string(&dest.retention)?;
    let exclusions_json = serde_json::to_string(&dest.exclusions)?;
    let last_run = dest.last_run.map(|dt| dt.to_rfc3339());
    let last_status = dest.last_status.as_ref().map(|s| s.to_string());
    let next_run = dest.next_run.map(|dt| dt.to_rfc3339());
    let dest_type_str = destination_type_str(&dest.destination_type);
    let encrypted_cloud_config = if let Some(ref config) = dest.cloud_config {
        let json = serde_json::to_string(config).map_err(|e| anyhow::anyhow!(e))?;
        Some(encrypt_cloud_config(&json)?)
    } else if let Some(ref config) = dest.sftp_config {
        let json = serde_json::to_string(config).map_err(|e| anyhow::anyhow!(e))?;
        Some(encrypt_cloud_config(&json)?)
    } else if let Some(ref config) = dest.oauth_config {
        let json = serde_json::to_string(config).map_err(|e| anyhow::anyhow!(e))?;
        Some(encrypt_cloud_config(&json)?)
    } else if let Some(ref config) = dest.webdav_config {
        let json = serde_json::to_string(config).map_err(|e| anyhow::anyhow!(e))?;
        Some(encrypt_cloud_config(&json)?)
    } else {
        None
    };

    sqlx::query(
        "INSERT INTO destinations (id, source_id, path, schedule_json, retention_json, exclusions_json, enabled, incremental, last_run, last_status, next_run, destination_type, cloud_config_json, encrypt, encrypt_password_enc, encrypt_salt)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&dest.id)
    .bind(&dest.source_id)
    .bind(&dest.path)
    .bind(&schedule_json)
    .bind(&retention_json)
    .bind(&exclusions_json)
    .bind(if dest.enabled { 1i64 } else { 0i64 })
    .bind(if dest.incremental { 1i64 } else { 0i64 })
    .bind(last_run)
    .bind(last_status)
    .bind(next_run)
    .bind(dest_type_str)
    .bind(encrypted_cloud_config)
    .bind(if dest.encrypt { 1i64 } else { 0i64 })
    .bind(&dest.encrypt_password_enc)
    .bind(&dest.encrypt_salt)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_destination(pool: &SqlitePool, dest: &Destination) -> anyhow::Result<()> {
    let schedule_json = serde_json::to_string(&dest.schedule)?;
    let retention_json = serde_json::to_string(&dest.retention)?;
    let exclusions_json = serde_json::to_string(&dest.exclusions)?;
    let last_run = dest.last_run.map(|dt| dt.to_rfc3339());
    let last_status = dest.last_status.as_ref().map(|s| s.to_string());
    let next_run = dest.next_run.map(|dt| dt.to_rfc3339());
    let dest_type_str = destination_type_str(&dest.destination_type);
    let encrypted_cloud_config = if let Some(ref config) = dest.cloud_config {
        let json = serde_json::to_string(config).map_err(|e| anyhow::anyhow!(e))?;
        Some(encrypt_cloud_config(&json)?)
    } else if let Some(ref config) = dest.sftp_config {
        let json = serde_json::to_string(config).map_err(|e| anyhow::anyhow!(e))?;
        Some(encrypt_cloud_config(&json)?)
    } else if let Some(ref config) = dest.oauth_config {
        let json = serde_json::to_string(config).map_err(|e| anyhow::anyhow!(e))?;
        Some(encrypt_cloud_config(&json)?)
    } else if let Some(ref config) = dest.webdav_config {
        let json = serde_json::to_string(config).map_err(|e| anyhow::anyhow!(e))?;
        Some(encrypt_cloud_config(&json)?)
    } else {
        None
    };

    sqlx::query(
        "UPDATE destinations SET path = ?, schedule_json = ?, retention_json = ?, exclusions_json = ?, enabled = ?, incremental = ?, last_run = ?, last_status = ?, next_run = ?, destination_type = ?, cloud_config_json = ?, encrypt = ?, encrypt_password_enc = ?, encrypt_salt = ?
         WHERE id = ?"
    )
    .bind(&dest.path)
    .bind(&schedule_json)
    .bind(&retention_json)
    .bind(&exclusions_json)
    .bind(if dest.enabled { 1i64 } else { 0i64 })
    .bind(if dest.incremental { 1i64 } else { 0i64 })
    .bind(last_run)
    .bind(last_status)
    .bind(next_run)
    .bind(dest_type_str)
    .bind(encrypted_cloud_config)
    .bind(if dest.encrypt { 1i64 } else { 0i64 })
    .bind(&dest.encrypt_password_enc)
    .bind(&dest.encrypt_salt)
    .bind(&dest.id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_destination(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM destinations WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_destinations_for_source(
    pool: &SqlitePool,
    source_id: &str,
) -> anyhow::Result<Vec<Destination>> {
    let rows = sqlx::query(
        "SELECT id, source_id, path, schedule_json, retention_json, exclusions_json, enabled, incremental, last_run, last_status, next_run, destination_type, cloud_config_json, encrypt, encrypt_password_enc, encrypt_salt
         FROM destinations WHERE source_id = ? ORDER BY id ASC"
    )
    .bind(source_id)
    .fetch_all(pool)
    .await?;

    let mut destinations = Vec::new();
    for row in rows {
        use sqlx::Row;
        let id: String = row.try_get("id")?;
        let source_id: String = row.try_get("source_id")?;
        let path: String = row.try_get("path")?;
        let schedule_json: String = row.try_get("schedule_json")?;
        let retention_json: String = row.try_get("retention_json")?;
        let exclusions_json: String = row.try_get("exclusions_json").unwrap_or_else(|_| "[]".to_string());
        let enabled_int: i64 = row.try_get("enabled")?;
        let incremental_int: i64 = row.try_get("incremental").unwrap_or(0);
        let last_run_str: Option<String> = row.try_get("last_run")?;
        let last_status_str: Option<String> = row.try_get("last_status")?;
        let next_run_str: Option<String> = row.try_get("next_run")?;
        let destination_type_str_val: String = row.try_get("destination_type").unwrap_or_else(|_| "Local".to_string());
        let cloud_config_json_enc: Option<String> = row.try_get("cloud_config_json").unwrap_or(None);

        let schedule: Schedule = serde_json::from_str(&schedule_json)?;
        let retention: RetentionPolicy = serde_json::from_str(&retention_json)?;
        let exclusions: Vec<String> = serde_json::from_str(&exclusions_json).unwrap_or_default();
        let enabled = enabled_int != 0;
        let incremental = incremental_int != 0;
        let last_run = last_run_str.and_then(|s| s.parse::<DateTime<Utc>>().ok());
        let last_status = last_status_str.and_then(|s| JobStatus::from_str(&s).ok());
        let next_run = next_run_str.and_then(|s| s.parse::<DateTime<Utc>>().ok());
        let destination_type = parse_destination_type(&destination_type_str_val);
        let cloud_config = if matches!(destination_type, DestinationType::S3 | DestinationType::R2) { cloud_config_json_enc.as_deref().and_then(decrypt_cloud_config) } else { None };
        let sftp_config = if destination_type == DestinationType::Sftp { cloud_config_json_enc.as_deref().and_then(decrypt_sftp_config) } else { None };
        let oauth_config = if matches!(destination_type, DestinationType::OneDrive | DestinationType::GoogleDrive | DestinationType::Dropbox) { cloud_config_json_enc.as_deref().and_then(decrypt_oauth_config) } else { None };
        let webdav_config = if destination_type == DestinationType::WebDav { cloud_config_json_enc.as_deref().and_then(decrypt_webdav_config) } else { None };
        let encrypt_int: i64 = row.try_get("encrypt").unwrap_or(0);
        let encrypt_password_enc: Option<String> = row.try_get("encrypt_password_enc").unwrap_or(None);
        let encrypt_salt: Option<String> = row.try_get("encrypt_salt").unwrap_or(None);

        destinations.push(Destination {
            id,
            source_id,
            path,
            schedule,
            retention,
            exclusions,
            enabled,
            incremental,
            last_run,
            last_status,
            next_run,
            destination_type,
            cloud_config,
            sftp_config,
            oauth_config,
            webdav_config,
            encrypt: encrypt_int != 0,
            encrypt_password_enc,
            encrypt_salt,
        });
    }

    Ok(destinations)
}

pub async fn get_all_active_destinations(
    pool: &SqlitePool,
) -> anyhow::Result<Vec<(Source, Destination)>> {
    let rows = sqlx::query(
        "SELECT
            s.id as s_id, s.name as s_name, s.path as s_path, s.source_type, s.enabled as s_enabled, s.created_at,
            d.id as d_id, d.source_id, d.path as d_path, d.schedule_json, d.retention_json, d.exclusions_json,
            d.enabled as d_enabled, d.incremental, d.last_run, d.last_status, d.next_run,
            d.destination_type, d.cloud_config_json, d.encrypt, d.encrypt_password_enc, d.encrypt_salt
         FROM sources s
         JOIN destinations d ON d.source_id = s.id
         WHERE s.enabled = 1 AND d.enabled = 1"
    )
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for row in rows {
        use sqlx::Row;
        let s_id: String = row.try_get("s_id")?;
        let s_name: String = row.try_get("s_name")?;
        let s_path: String = row.try_get("s_path")?;
        let source_type_str: String = row.try_get("source_type")?;
        let s_enabled_int: i64 = row.try_get("s_enabled")?;
        let created_at_str: String = row.try_get("created_at")?;

        let d_id: String = row.try_get("d_id")?;
        let source_id: String = row.try_get("source_id")?;
        let d_path: String = row.try_get("d_path")?;
        let schedule_json: String = row.try_get("schedule_json")?;
        let retention_json: String = row.try_get("retention_json")?;
        let exclusions_json: String = row.try_get("exclusions_json").unwrap_or_else(|_| "[]".to_string());
        let d_enabled_int: i64 = row.try_get("d_enabled")?;
        let incremental_int: i64 = row.try_get("incremental").unwrap_or(0);
        let last_run_str: Option<String> = row.try_get("last_run")?;
        let last_status_str: Option<String> = row.try_get("last_status")?;
        let next_run_str: Option<String> = row.try_get("next_run")?;
        let destination_type_str_val: String = row.try_get("destination_type").unwrap_or_else(|_| "Local".to_string());
        let cloud_config_json_enc: Option<String> = row.try_get("cloud_config_json").unwrap_or(None);

        let source_type = SourceType::from_str(&source_type_str)?;
        let created_at = created_at_str
            .parse::<DateTime<Utc>>()
            .unwrap_or_else(|_| Utc::now());
        let schedule: Schedule = serde_json::from_str(&schedule_json)?;
        let retention: RetentionPolicy = serde_json::from_str(&retention_json)?;
        let exclusions: Vec<String> = serde_json::from_str(&exclusions_json).unwrap_or_default();
        let last_run = last_run_str.and_then(|s| s.parse::<DateTime<Utc>>().ok());
        let last_status = last_status_str.and_then(|s| JobStatus::from_str(&s).ok());
        let next_run = next_run_str.and_then(|s| s.parse::<DateTime<Utc>>().ok());
        let destination_type = parse_destination_type(&destination_type_str_val);
        let cloud_config = if matches!(destination_type, DestinationType::S3 | DestinationType::R2) { cloud_config_json_enc.as_deref().and_then(decrypt_cloud_config) } else { None };
        let sftp_config = if destination_type == DestinationType::Sftp { cloud_config_json_enc.as_deref().and_then(decrypt_sftp_config) } else { None };
        let oauth_config = if matches!(destination_type, DestinationType::OneDrive | DestinationType::GoogleDrive | DestinationType::Dropbox) { cloud_config_json_enc.as_deref().and_then(decrypt_oauth_config) } else { None };
        let webdav_config = if destination_type == DestinationType::WebDav { cloud_config_json_enc.as_deref().and_then(decrypt_webdav_config) } else { None };

        let source = Source {
            id: s_id.clone(),
            name: s_name,
            path: s_path,
            source_type,
            enabled: s_enabled_int != 0,
            created_at,
            destinations: vec![],
        };

        let d_encrypt_int: i64 = row.try_get("encrypt").unwrap_or(0);
        let d_encrypt_password_enc: Option<String> = row.try_get("encrypt_password_enc").unwrap_or(None);
        let d_encrypt_salt: Option<String> = row.try_get("encrypt_salt").unwrap_or(None);

        let destination = Destination {
            id: d_id,
            source_id,
            path: d_path,
            schedule,
            retention,
            exclusions,
            enabled: d_enabled_int != 0,
            incremental: incremental_int != 0,
            last_run,
            last_status,
            next_run,
            destination_type,
            cloud_config,
            sftp_config,
            oauth_config,
            webdav_config,
            encrypt: d_encrypt_int != 0,
            encrypt_password_enc: d_encrypt_password_enc,
            encrypt_salt: d_encrypt_salt,
        };

        result.push((source, destination));
    }

    Ok(result)
}

pub async fn update_oauth_token(
    pool: &SqlitePool,
    dest_id: &str,
    new_oauth: &OAuthConfig,
) -> anyhow::Result<()> {
    let json = serde_json::to_string(new_oauth)?;
    let encrypted = encrypt_cloud_config(&json)?;
    sqlx::query("UPDATE destinations SET cloud_config_json = ? WHERE id = ?")
        .bind(encrypted)
        .bind(dest_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_destination_run_status(
    pool: &SqlitePool,
    dest_id: &str,
    last_run: DateTime<Utc>,
    status: &str,
    next_run: Option<DateTime<Utc>>,
) -> anyhow::Result<()> {
    let next_run_str = next_run.map(|dt| dt.to_rfc3339());
    sqlx::query(
        "UPDATE destinations SET last_run = ?, last_status = ?, next_run = ? WHERE id = ?"
    )
    .bind(last_run.to_rfc3339())
    .bind(status)
    .bind(next_run_str)
    .bind(dest_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_log_entry(
    pool: &SqlitePool,
    source_id: &str,
    destination_id: &str,
    source_path: &str,
    destination_path: &str,
    started_at: DateTime<Utc>,
    status: &str,
    trigger: &str,
) -> anyhow::Result<i64> {
    let result = sqlx::query(
        "INSERT INTO copy_logs (source_id, destination_id, source_path, destination_path, started_at, status, trigger)
         VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(source_id)
    .bind(destination_id)
    .bind(source_path)
    .bind(destination_path)
    .bind(started_at.to_rfc3339())
    .bind(status)
    .bind(trigger)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

pub async fn update_log_entry_completed(
    pool: &SqlitePool,
    log_id: i64,
    ended_at: DateTime<Utc>,
    status: &str,
    bytes_copied: Option<i64>,
    files_copied: Option<i32>,
    error_message: Option<&str>,
    checksum: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE copy_logs SET ended_at = ?, status = ?, bytes_copied = ?, files_copied = ?, error_message = ?, checksum = ? WHERE id = ?"
    )
    .bind(ended_at.to_rfc3339())
    .bind(status)
    .bind(bytes_copied)
    .bind(files_copied)
    .bind(error_message)
    .bind(checksum)
    .bind(log_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn cancel_running_logs(
    pool: &SqlitePool,
    ended_at: DateTime<Utc>,
) -> anyhow::Result<u64> {
    let result = sqlx::query(
        "UPDATE copy_logs SET status = 'Cancelled', ended_at = ?, error_message = 'Application shutdown'
         WHERE status = 'Running'"
    )
    .bind(ended_at.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn get_logs(
    pool: &SqlitePool,
    source_id: Option<&str>,
    destination_id: Option<&str>,
    status: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> anyhow::Result<Vec<crate::models::LogEntry>> {
    let mut query_str = String::from(
        "SELECT id, source_id, destination_id, source_path, destination_path, started_at, ended_at, status, bytes_copied, files_copied, error_message, trigger, checksum
         FROM copy_logs WHERE 1=1"
    );

    if source_id.is_some() {
        query_str.push_str(" AND source_id = ?");
    }
    if destination_id.is_some() {
        query_str.push_str(" AND destination_id = ?");
    }
    if status.is_some() {
        query_str.push_str(" AND status = ?");
    }

    query_str.push_str(" ORDER BY started_at DESC");

    if limit.is_some() {
        query_str.push_str(" LIMIT ?");
    }
    if offset.is_some() {
        query_str.push_str(" OFFSET ?");
    }

    let mut q = sqlx::query(&query_str);

    if let Some(sid) = source_id {
        q = q.bind(sid);
    }
    if let Some(did) = destination_id {
        q = q.bind(did);
    }
    if let Some(st) = status {
        q = q.bind(st);
    }
    if let Some(l) = limit {
        q = q.bind(l);
    }
    if let Some(o) = offset {
        q = q.bind(o);
    }

    let rows = q.fetch_all(pool).await?;

    let mut logs = Vec::new();
    for row in rows {
        use sqlx::Row;
        let id: i64 = row.try_get("id")?;
        let source_id: String = row.try_get("source_id")?;
        let destination_id: String = row.try_get("destination_id")?;
        let source_path: String = row.try_get("source_path")?;
        let destination_path: String = row.try_get("destination_path")?;
        let started_at_str: String = row.try_get("started_at")?;
        let ended_at_str: Option<String> = row.try_get("ended_at")?;
        let status: String = row.try_get("status")?;
        let bytes_copied: Option<i64> = row.try_get("bytes_copied")?;
        let files_copied: Option<i32> = row.try_get("files_copied")?;
        let error_message: Option<String> = row.try_get("error_message")?;
        let trigger: String = row.try_get("trigger")?;
        let checksum: Option<String> = row.try_get("checksum").unwrap_or(None);

        let started_at = started_at_str
            .parse::<DateTime<Utc>>()
            .unwrap_or_else(|_| Utc::now());
        let ended_at = ended_at_str.and_then(|s| s.parse::<DateTime<Utc>>().ok());

        logs.push(crate::models::LogEntry {
            id,
            source_id,
            destination_id,
            source_path,
            destination_path,
            started_at,
            ended_at,
            status,
            bytes_copied,
            files_copied,
            error_message,
            trigger,
            checksum,
        });
    }

    Ok(logs)
}

pub async fn get_log_count(pool: &SqlitePool, source_id: Option<&str>) -> anyhow::Result<i64> {
    let count: i64 = if let Some(sid) = source_id {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM copy_logs WHERE source_id = ?")
            .bind(sid)
            .fetch_one(pool)
            .await?;
        use sqlx::Row;
        row.try_get("cnt")?
    } else {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM copy_logs")
            .fetch_one(pool)
            .await?;
        use sqlx::Row;
        row.try_get("cnt")?
    };

    Ok(count)
}

pub async fn clear_old_logs(pool: &SqlitePool, older_than_days: u32) -> anyhow::Result<u64> {
    let cutoff = Utc::now() - chrono::Duration::days(older_than_days as i64);
    let result = sqlx::query("DELETE FROM copy_logs WHERE started_at < ?")
        .bind(cutoff.to_rfc3339())
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

pub async fn get_setting(pool: &SqlitePool, key: &str) -> anyhow::Result<Option<String>> {
    let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    match row {
        None => Ok(None),
        Some(row) => {
            use sqlx::Row;
            let value: String = row.try_get("value")?;
            Ok(Some(value))
        }
    }
}

pub async fn upsert_setting(pool: &SqlitePool, key: &str, value: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_schema_version(pool: &SqlitePool) -> anyhow::Result<i64> {
    let row = sqlx::query("SELECT COALESCE(MAX(version), 0) AS v FROM schema_versions")
        .fetch_one(pool)
        .await?;
    use sqlx::Row;
    Ok(row.try_get::<i64, _>("v").unwrap_or(0))
}

pub async fn get_onchange_destinations(
    pool: &SqlitePool,
) -> anyhow::Result<Vec<(Source, Destination)>> {
    let all = get_all_active_destinations(pool).await?;
    Ok(all
        .into_iter()
        .filter(|(_, d)| matches!(d.schedule, Schedule::OnChange))
        .collect())
}
