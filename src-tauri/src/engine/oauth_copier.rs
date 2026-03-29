use chrono::Utc;
use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::Emitter;

use crate::db::queries;
use crate::engine::oauth_token;
use crate::engine::retry;
use crate::models::{Destination, LogEntry, OAuthConfig, Source, SourceType};

pub struct OAuthCopyJob {
    pub source: Source,
    pub destination: Destination,
    pub trigger: String,
    pub app: Option<tauri::AppHandle>,
}

// ── opendal operator builder ─────────────────────────────────────────────────

fn build_operator(config: &OAuthConfig) -> anyhow::Result<opendal::Operator> {
    match config.provider.as_str() {
        "onedrive" => {
            let builder = opendal::services::Onedrive::default()
                .access_token(&config.access_token)
                .root(&config.folder_path);
            Ok(opendal::Operator::new(builder)?.finish())
        }
        "gdrive" => {
            let builder = opendal::services::Gdrive::default()
                .access_token(&config.access_token)
                .root(&config.folder_path);
            Ok(opendal::Operator::new(builder)?.finish())
        }
        "dropbox" => {
            let builder = opendal::services::Dropbox::default()
                .access_token(&config.access_token)
                .root(&config.folder_path);
            Ok(opendal::Operator::new(builder)?.finish())
        }
        p => anyhow::bail!("Bilinmeyen OAuth sağlayıcısı: {}", p),
    }
}

// ── OAuthCopyJob ─────────────────────────────────────────────────────────────

impl OAuthCopyJob {
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
            .oauth_config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("OAuth config eksik"))?
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

        // Refresh token if needed and persist updated config
        let fresh_config = oauth_token::ensure_fresh_token(&config).await?;
        if fresh_config.access_token != config.access_token {
            if let Err(e) =
                queries::update_oauth_token(&db, &self.destination.id, &fresh_config).await
            {
                log::warn!("OAuth token güncelleme başarısız: {}", e);
            }
        }

        let version_key = format!(
            "{}_{}",
            self.source.name,
            started_at.format("%Y-%m-%dT%H-%M-%SZ")
        );
        let display_path = format!(
            "{}://{}/{}",
            fresh_config.provider,
            fresh_config.folder_path.trim_end_matches('/'),
            version_key,
        );

        // Incremental cutoff
        let since: Option<std::time::SystemTime> = if self.destination.incremental {
            self.destination.last_run.map(|dt| {
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(dt.timestamp() as u64)
            })
        } else {
            None
        };

        // Collect files
        let file_entries = self.collect_files(since).await?;
        let total_files = file_entries.len() as i32;
        self.emit_progress(0, total_files, 0);

        // Build operator and upload
        let op = build_operator(&fresh_config)?;
        let upload_result = self
            .do_upload_with_retry(&op, &version_key, &file_entries)
            .await;

        let ended_at = Utc::now();

        match upload_result {
            Ok((bytes_copied, files_copied)) => {
                let checksum = Some(format!("{} dosya OAuth bulutuna yüklendi", files_copied));
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

    async fn do_upload(
        &self,
        op: &opendal::Operator,
        version_key: &str,
        file_entries: &[(std::path::PathBuf, String)],
    ) -> anyhow::Result<(i64, i32)> {
        let mut total_bytes: i64 = 0;
        let mut files_done: i32 = 0;
        let mut bytes_done: i64 = 0;
        let files_total = file_entries.len() as i32;

        for (local_path, rel_key) in file_entries {
            let remote_path = format!("{}/{}", version_key, rel_key.replace('\\', "/"));
            let data = std::fs::read(local_path)?;
            let len = data.len() as i64;

            op.write(&remote_path, data)
                .await
                .map_err(|e| anyhow::anyhow!("Yükleme hatası {}: {}", remote_path, e))?;

            total_bytes += len;
            files_done += 1;
            bytes_done += len;
            self.emit_progress(files_done, files_total, bytes_done);
        }

        Ok((total_bytes, files_done))
    }

    async fn do_upload_with_retry(
        &self,
        op: &opendal::Operator,
        version_key: &str,
        file_entries: &[(std::path::PathBuf, String)],
    ) -> anyhow::Result<(i64, i32)> {
        retry::run_remote_with_retry("OAuth upload", || {
            self.do_upload(op, version_key, file_entries)
        })
        .await
    }

    async fn collect_files(
        &self,
        since: Option<std::time::SystemTime>,
    ) -> anyhow::Result<Vec<(std::path::PathBuf, String)>> {
        let source_path = std::path::Path::new(&self.source.path);
        let mut entries: Vec<(std::path::PathBuf, String)> = Vec::new();

        match &self.source.source_type {
            SourceType::File => {
                let name = source_path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Dosya adı alınamadı"))?
                    .to_string_lossy()
                    .to_string();
                entries.push((source_path.to_path_buf(), name));
            }
            SourceType::Directory => {
                for entry in walkdir::WalkDir::new(source_path) {
                    let entry = entry?;
                    if !entry.file_type().is_file() {
                        continue;
                    }

                    if let Some(since_time) = since {
                        if let Ok(meta) = entry.metadata() {
                            if let Ok(modified) = meta.modified() {
                                if modified <= since_time {
                                    continue;
                                }
                            }
                        }
                    }

                    let rel = entry
                        .path()
                        .strip_prefix(source_path)
                        .unwrap_or(entry.path())
                        .to_string_lossy()
                        .to_string();
                    entries.push((entry.path().to_path_buf(), rel));
                }
            }
        }

        Ok(entries)
    }
}

/// Test connection by stat'ing the root.
pub async fn test_connection(config: &OAuthConfig) -> anyhow::Result<()> {
    let fresh = oauth_token::ensure_fresh_token(config).await?;
    let op = build_operator(&fresh)?;
    op.stat("/")
        .await
        .map_err(|e| anyhow::anyhow!("Bağlantı testi başarısız: {}", e))?;
    Ok(())
}
