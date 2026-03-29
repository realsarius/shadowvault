use chrono::Utc;
use sqlx::SqlitePool;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tauri::Emitter;

use crate::db::queries;
use crate::engine::retry;
use crate::models::{Destination, LogEntry, SftpConfig, Source, SourceType};

pub struct SftpCopyJob {
    pub source: Source,
    pub destination: Destination,
    pub trigger: String,
    pub app: Option<tauri::AppHandle>,
}

// ── SSH helpers (run in spawn_blocking) ──────────────────────────────────────

fn connect_sftp(config: &SftpConfig) -> anyhow::Result<(ssh2::Session, ssh2::Sftp)> {
    let addr = format!("{}:{}", config.host, config.port);
    let tcp = std::net::TcpStream::connect(&addr)
        .map_err(|e| anyhow::anyhow!("TCP bağlantı hatası ({}): {}", addr, e))?;

    let mut session = ssh2::Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;

    match config.auth_type.as_str() {
        "key" => {
            let key_path = config
                .private_key
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Özel anahtar yolu belirtilmemiş"))?;
            session
                .userauth_pubkey_file(&config.username, None, Path::new(key_path), None)
                .map_err(|e| anyhow::anyhow!("SSH anahtar doğrulama hatası: {}", e))?;
        }
        _ => {
            let password = config
                .password
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Şifre belirtilmemiş"))?;
            session
                .userauth_password(&config.username, password)
                .map_err(|e| anyhow::anyhow!("SSH şifre doğrulama hatası: {}", e))?;
        }
    }

    if !session.authenticated() {
        anyhow::bail!("SSH kimlik doğrulama başarısız");
    }

    let sftp = session.sftp()?;
    Ok((session, sftp))
}

fn ensure_remote_dir(sftp: &ssh2::Sftp, path: &str) -> anyhow::Result<()> {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut current = String::new();
    for part in parts {
        current = format!("{}/{}", current, part);
        let p = Path::new(&current);
        match sftp.lstat(p) {
            Ok(_) => continue,
            Err(_) => {
                // Create dir; ignore error if it was already created by a concurrent upload
                let _ = sftp.mkdir(p, 0o755);
            }
        }
    }
    Ok(())
}

fn do_sftp_upload(
    config: &SftpConfig,
    version_prefix: &str,
    file_entries: Vec<(std::path::PathBuf, String)>,
    app: Option<tauri::AppHandle>,
    dest_id: String,
) -> anyhow::Result<(i64, i32)> {
    let (_session, sftp) = connect_sftp(config)?;
    let base_remote = format!(
        "{}/{}",
        config.remote_path.trim_end_matches('/'),
        version_prefix
    );
    ensure_remote_dir(&sftp, &base_remote)?;

    let files_total = file_entries.len() as i32;
    let mut total_bytes: i64 = 0;
    let mut files_done: i32 = 0;
    let mut bytes_done: i64 = 0;

    for (local_path, rel_key) in &file_entries {
        let remote_full = format!("{}/{}", base_remote, rel_key.replace('\\', "/"));

        // Ensure parent directory
        if let Some(parent) = Path::new(&remote_full).parent() {
            let parent_str = parent.to_string_lossy().to_string();
            if !parent_str.is_empty() {
                ensure_remote_dir(&sftp, &parent_str)?;
            }
        }

        let data = std::fs::read(local_path)?;
        let len = data.len() as i64;

        let mut remote_file = sftp
            .create(Path::new(&remote_full))
            .map_err(|e| anyhow::anyhow!("Uzak dosya oluşturulamadı {}: {}", remote_full, e))?;
        remote_file.write_all(&data)?;

        total_bytes += len;
        files_done += 1;
        bytes_done += len;

        if let Some(ref app) = app {
            let _ = app.emit(
                "copy-progress",
                serde_json::json!({
                    "destination_id": dest_id,
                    "files_done": files_done,
                    "files_total": files_total,
                    "bytes_done": bytes_done,
                }),
            );
        }
    }

    Ok((total_bytes, files_done))
}

// ── SftpCopyJob ──────────────────────────────────────────────────────────────

impl SftpCopyJob {
    fn emit_progress(&self, files_done: i32, files_total: i32, bytes_done: i64) {
        if let Some(app) = &self.app {
            let _ = app.emit(
                "copy-progress",
                serde_json::json!({
                    "destination_id": &self.destination.id,
                    "files_done": files_done,
                    "files_total": files_total,
                    "bytes_done": bytes_done,
                }),
            );
        }
    }

    pub async fn execute(&self, db: Arc<SqlitePool>) -> anyhow::Result<LogEntry> {
        let config = self
            .destination
            .sftp_config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SFTP config eksik"))?
            .clone();

        let started_at = Utc::now();

        let log_id = queries::insert_log_entry(
            &db,
            &self.source.id,
            &self.destination.id,
            &self.source.path,
            &self.destination.path,
            started_at,
            "Running",
            &self.trigger,
            None,
            None,
        )
        .await?;

        let version_key = format!(
            "{}_{}",
            self.source.name,
            started_at.format("%Y-%m-%dT%H-%M-%SZ")
        );
        let display_path = format!(
            "sftp://{}{}{}",
            config.host,
            config.remote_path.trim_end_matches('/'),
            format!("/{}", version_key)
        );

        // For incremental: compute cutoff
        let since: Option<std::time::SystemTime> = if self.destination.incremental {
            self.destination.last_run.map(|dt| {
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(dt.timestamp() as u64)
            })
        } else {
            None
        };

        // Collect files to upload (async-friendly scan)
        let file_entries = self.collect_files(since).await?;

        let total_files = file_entries.len() as i32;
        self.emit_progress(0, total_files, 0);

        let copy_result = self
            .do_upload_with_retry(&config, &version_key, &file_entries)
            .await;

        let ended_at = Utc::now();

        match copy_result {
            Ok((bytes_copied, files_copied)) => {
                let checksum = Some(format!("{} dosya SFTP ile yüklendi", files_copied));

                queries::update_log_entry_completed(
                    &db,
                    log_id,
                    ended_at,
                    "Success",
                    Some(bytes_copied),
                    Some(files_copied),
                    None,
                    checksum.as_deref(),
                    None,
                    None,
                )
                .await?;

                let next_run = crate::engine::copier::compute_next_run_pub(
                    &self.destination.schedule,
                    ended_at,
                );
                queries::update_destination_run_status(
                    &db,
                    &self.destination.id,
                    ended_at,
                    "Success",
                    next_run,
                )
                .await?;

                {
                    let email_db = db.clone();
                    let name = self.source.name.clone();
                    tokio::spawn(async move {
                        crate::notifications::send_backup_email(
                            &email_db,
                            &name,
                            Some(files_copied),
                            Some(bytes_copied),
                            None,
                        )
                        .await;
                    });
                }

                Ok(LogEntry {
                    id: log_id,
                    source_id: self.source.id.clone(),
                    destination_id: self.destination.id.clone(),
                    source_path: self.source.path.clone(),
                    destination_path: display_path,
                    started_at,
                    ended_at: Some(ended_at),
                    status: "Success".to_string(),
                    bytes_copied: Some(bytes_copied),
                    files_copied: Some(files_copied),
                    error_message: None,
                    trigger: self.trigger.clone(),
                    checksum,
                    backup_level: None,
                    snapshot_id: None,
                })
            }
            Err(e) => {
                let error_msg = e.to_string();
                queries::update_log_entry_completed(
                    &db,
                    log_id,
                    ended_at,
                    "Failed",
                    None,
                    None,
                    Some(&error_msg),
                    None,
                    None,
                    None,
                )
                .await?;

                let next_run = crate::engine::copier::compute_next_run_pub(
                    &self.destination.schedule,
                    ended_at,
                );
                queries::update_destination_run_status(
                    &db,
                    &self.destination.id,
                    ended_at,
                    "Failed",
                    next_run,
                )
                .await?;

                {
                    let email_db = db.clone();
                    let name = self.source.name.clone();
                    let err_clone = error_msg.clone();
                    tokio::spawn(async move {
                        crate::notifications::send_backup_email(
                            &email_db,
                            &name,
                            None,
                            None,
                            Some(&err_clone),
                        )
                        .await;
                    });
                }

                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    async fn collect_files(
        &self,
        since: Option<std::time::SystemTime>,
    ) -> anyhow::Result<Vec<(std::path::PathBuf, String)>> {
        let source_path = std::path::Path::new(&self.source.path);
        let mut file_entries: Vec<(std::path::PathBuf, String)> = Vec::new();

        match &self.source.source_type {
            SourceType::File => {
                let file_name = source_path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Dosya adı alınamadı"))?
                    .to_string_lossy()
                    .to_string();
                file_entries.push((source_path.to_path_buf(), file_name));
            }
            SourceType::Directory => {
                for entry in walkdir::WalkDir::new(source_path) {
                    let entry = entry?;
                    if !entry.file_type().is_file() {
                        continue;
                    }

                    let rel_path = entry
                        .path()
                        .strip_prefix(source_path)
                        .unwrap_or(entry.path());

                    if let Some(since_time) = since {
                        if let Ok(meta) = entry.metadata() {
                            if let Ok(modified) = meta.modified() {
                                if modified <= since_time {
                                    continue;
                                }
                            }
                        }
                    }

                    file_entries.push((
                        entry.path().to_path_buf(),
                        rel_path.to_string_lossy().to_string(),
                    ));
                }
            }
        }

        Ok(file_entries)
    }

    async fn do_upload_with_retry(
        &self,
        config: &SftpConfig,
        version_key: &str,
        file_entries: &[(std::path::PathBuf, String)],
    ) -> anyhow::Result<(i64, i32)> {
        retry::run_remote_with_retry("SFTP upload", || async {
            let config_clone = config.clone();
            let vk = version_key.to_string();
            let entries = file_entries.to_vec();
            let app_handle = self.app.clone();
            let dest_id = self.destination.id.clone();

            let copy_result = tokio::task::spawn_blocking(move || {
                do_sftp_upload(&config_clone, &vk, entries, app_handle, dest_id)
            })
            .await
            .map_err(|e| anyhow::anyhow!("SFTP worker failed: {}", e))?;

            copy_result
        })
        .await
    }
}

/// Test SFTP connection by connecting and listing the remote_path
pub fn test_connection_blocking(config: &SftpConfig) -> anyhow::Result<()> {
    let (_session, sftp) = connect_sftp(config)?;

    // List the remote_path (create if not exists, then stat it)
    let remote = config.remote_path.trim();
    if !remote.is_empty() && remote != "/" {
        match sftp.lstat(Path::new(remote)) {
            Ok(_) => {}
            Err(_) => {
                // Try to create it
                ensure_remote_dir(&sftp, remote)?;
            }
        }
    }

    Ok(())
}
