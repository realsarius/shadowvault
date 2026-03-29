use bytes::Bytes;
use chrono::Utc;
use object_store::aws::AmazonS3Builder;
use object_store::{path::Path as OsPath, ObjectStore};
use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::Emitter;

use crate::db::queries;
use crate::engine::retry;
use crate::models::{Destination, DestinationType, LogEntry, S3Config, Source, SourceType};

pub struct CloudCopyJob {
    pub source: Source,
    pub destination: Destination,
    pub trigger: String,
    pub app: Option<tauri::AppHandle>,
}

impl CloudCopyJob {
    fn build_store(config: &S3Config) -> anyhow::Result<Arc<dyn ObjectStore>> {
        let mut builder = AmazonS3Builder::new()
            .with_bucket_name(&config.bucket)
            .with_region(&config.region)
            .with_access_key_id(&config.access_key_id)
            .with_secret_access_key(&config.secret_access_key);

        if let Some(endpoint) = &config.endpoint_url {
            builder = builder
                .with_endpoint(endpoint)
                .with_virtual_hosted_style_request(false);
        }

        Ok(Arc::new(builder.build()?))
    }

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
            .cloud_config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cloud config eksik"))?;

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

        // Version prefix: e.g. "mybackup/2025-03-15T14-30-00Z/"
        let version_key = format!(
            "{}{}_{}",
            if config.prefix.is_empty() {
                String::new()
            } else {
                format!("{}/", config.prefix)
            },
            self.source.name,
            started_at.format("%Y-%m-%dT%H-%M-%SZ")
        );

        let display_path = format!("s3://{}/{}", config.bucket, version_key);

        let copy_result = self.do_upload_with_retry(config, &version_key).await;

        let ended_at = Utc::now();

        match copy_result {
            Ok((bytes_copied, files_copied)) => {
                let checksum = Some(format!(
                    "{} dosya buluta yüklendi ({})",
                    files_copied,
                    if self.destination.destination_type == DestinationType::R2 {
                        "R2"
                    } else {
                        "S3"
                    }
                ));

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
        config: &S3Config,
        version_prefix: &str,
    ) -> anyhow::Result<(i64, i32)> {
        let store = Self::build_store(config)?;
        let source_path = std::path::Path::new(&self.source.path);

        // For incremental: get last_run cutoff
        let since: Option<std::time::SystemTime> = if self.destination.incremental {
            self.destination.last_run.map(|dt| {
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(dt.timestamp() as u64)
            })
        } else {
            None
        };

        match &self.source.source_type {
            SourceType::File => {
                let file_name = source_path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Dosya adı alınamadı"))?
                    .to_string_lossy();
                let key = OsPath::from(format!("{}/{}", version_prefix, file_name));

                self.emit_progress(0, 1, 0);
                let data = tokio::fs::read(source_path).await?;
                let len = data.len() as i64;
                store.put(&key, Bytes::from(data).into()).await?;
                self.emit_progress(1, 1, len);

                Ok((len, 1))
            }
            SourceType::Directory => {
                // Collect all files
                let mut file_entries: Vec<(std::path::PathBuf, String)> = Vec::new();

                for entry in walkdir::WalkDir::new(source_path) {
                    let entry = entry?;
                    if !entry.file_type().is_file() {
                        continue;
                    }

                    let rel_path = entry
                        .path()
                        .strip_prefix(source_path)
                        .unwrap_or(entry.path());

                    // Incremental: skip unchanged files
                    if let Some(since_time) = since {
                        if let Ok(meta) = entry.metadata() {
                            if let Ok(modified) = meta.modified() {
                                if modified <= since_time {
                                    continue;
                                }
                            }
                        }
                    }

                    let s3_key = format!(
                        "{}/{}",
                        version_prefix,
                        rel_path.to_string_lossy().replace('\\', "/")
                    );
                    file_entries.push((entry.path().to_path_buf(), s3_key));
                }

                let files_total = file_entries.len() as i32;
                self.emit_progress(0, files_total, 0);

                let mut total_bytes: i64 = 0;
                let mut files_done: i32 = 0;
                let mut bytes_done: i64 = 0;

                for (local_path, s3_key) in &file_entries {
                    let data = tokio::fs::read(local_path).await?;
                    let len = data.len() as i64;
                    let key = OsPath::from(s3_key.as_str());
                    store.put(&key, Bytes::from(data).into()).await?;
                    total_bytes += len;
                    files_done += 1;
                    bytes_done += len;
                    self.emit_progress(files_done, files_total, bytes_done);
                }

                Ok((total_bytes, files_done))
            }
        }
    }

    async fn do_upload_with_retry(
        &self,
        config: &S3Config,
        version_prefix: &str,
    ) -> anyhow::Result<(i64, i32)> {
        retry::run_remote_with_retry("Cloud upload", || self.do_upload(config, version_prefix))
            .await
    }
}

/// Test a cloud connection by listing objects in the bucket
pub async fn test_connection(config: &S3Config) -> anyhow::Result<()> {
    let mut builder = AmazonS3Builder::new()
        .with_bucket_name(&config.bucket)
        .with_region(&config.region)
        .with_access_key_id(&config.access_key_id)
        .with_secret_access_key(&config.secret_access_key);

    if let Some(endpoint) = &config.endpoint_url {
        builder = builder
            .with_endpoint(endpoint)
            .with_virtual_hosted_style_request(false);
    }

    let store = builder.build()?;

    // ListObjectsV2 with limit=1 to verify credentials
    let prefix = if config.prefix.is_empty() {
        None
    } else {
        Some(OsPath::from(config.prefix.as_str()))
    };

    let _result = store
        .list_with_delimiter(prefix.as_ref())
        .await
        .map_err(|e| anyhow::anyhow!("Bağlantı hatası: {}", e))?;

    Ok(())
}
