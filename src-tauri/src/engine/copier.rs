use std::sync::Arc;
use chrono::Utc;
use sqlx::SqlitePool;
use crate::models::{Source, Destination, SourceType, LogEntry};
use crate::db::queries;
use crate::engine::versioning;

pub struct CopyJob {
    pub source: Source,
    pub destination: Destination,
    pub trigger: String,
}

impl CopyJob {
    pub async fn execute(&self, db: Arc<SqlitePool>) -> anyhow::Result<LogEntry> {
        let started_at = Utc::now();

        // Insert initial log row with status Running
        let log_id = queries::insert_log_entry(
            &db,
            &self.source.id,
            &self.destination.id,
            &self.source.path,
            &self.destination.path,
            started_at,
            "Running",
            &self.trigger,
        )
        .await?;

        // Compute the versioned destination path
        let version_path = versioning::compute_version_path(
            &self.destination.path,
            &self.source.name,
            &self.destination.retention.naming,
            started_at,
        );

        let destination_path_str = version_path.to_string_lossy().to_string();

        // Attempt the copy
        let copy_result = self.do_copy(&version_path).await;

        let ended_at = Utc::now();

        match copy_result {
            Ok((bytes_copied, files_copied)) => {
                // Update log to Success
                queries::update_log_entry_completed(
                    &db,
                    log_id,
                    ended_at,
                    "Success",
                    Some(bytes_copied),
                    Some(files_copied),
                    None,
                )
                .await?;

                // Apply retention policy
                if let Err(e) = versioning::apply_retention(
                    &self.destination.path,
                    &self.source.name,
                    self.destination.retention.max_versions,
                )
                .await
                {
                    log::warn!("Retention policy failed for destination {}: {}", self.destination.id, e);
                }

                // Update destination last_run, last_status, next_run
                let next_run = compute_next_run(&self.destination.schedule, ended_at);
                queries::update_destination_run_status(
                    &db,
                    &self.destination.id,
                    ended_at,
                    "Success",
                    next_run,
                )
                .await?;

                Ok(LogEntry {
                    id: log_id,
                    source_id: self.source.id.clone(),
                    destination_id: self.destination.id.clone(),
                    source_path: self.source.path.clone(),
                    destination_path: destination_path_str,
                    started_at,
                    ended_at: Some(ended_at),
                    status: "Success".to_string(),
                    bytes_copied: Some(bytes_copied),
                    files_copied: Some(files_copied),
                    error_message: None,
                    trigger: self.trigger.clone(),
                })
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Update log to Failed
                queries::update_log_entry_completed(
                    &db,
                    log_id,
                    ended_at,
                    "Failed",
                    None,
                    None,
                    Some(&error_msg),
                )
                .await?;

                // Update destination status
                let next_run = compute_next_run(&self.destination.schedule, ended_at);
                queries::update_destination_run_status(
                    &db,
                    &self.destination.id,
                    ended_at,
                    "Failed",
                    next_run,
                )
                .await?;

                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    async fn do_copy(&self, version_path: &std::path::Path) -> anyhow::Result<(i64, i32)> {
        // Create destination directory
        std::fs::create_dir_all(version_path)?;

        match &self.source.source_type {
            SourceType::File => {
                let source_path = std::path::Path::new(&self.source.path);
                let file_name = source_path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Cannot determine file name from source path"))?;
                let dest_file = version_path.join(file_name);

                let bytes = std::fs::copy(source_path, &dest_file)?;

                Ok((bytes as i64, 1))
            }
            SourceType::Directory => {
                let source_path = std::path::Path::new(&self.source.path);

                // Remove the directory we just created so fs_extra can copy into it cleanly
                // fs_extra::dir::copy copies the source dir INTO the destination
                // We want the contents copied, so we copy source into version_path's parent
                // and rename if needed, or use copy_contents
                let options = fs_extra::dir::CopyOptions {
                    overwrite: true,
                    skip_exist: false,
                    buffer_size: 64 * 1024,
                    copy_inside: true,
                    content_only: true,
                    depth: 0,
                };

                fs_extra::dir::copy(source_path, version_path, &options)
                    .map_err(|e| anyhow::anyhow!("Directory copy failed: {}", e))?;

                // Count bytes and files after copy
                let (bytes, files) = count_dir_stats(version_path)?;

                Ok((bytes, files))
            }
        }
    }
}

fn count_dir_stats(dir: &std::path::Path) -> anyhow::Result<(i64, i32)> {
    let mut total_bytes: i64 = 0;
    let mut total_files: i32 = 0;

    if dir.is_file() {
        let meta = std::fs::metadata(dir)?;
        return Ok((meta.len() as i64, 1));
    }

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let meta = entry.metadata()?;
            total_bytes += meta.len() as i64;
            total_files += 1;
        }
    }

    Ok((total_bytes, total_files))
}

fn compute_next_run(
    schedule: &crate::models::Schedule,
    after: chrono::DateTime<Utc>,
) -> Option<chrono::DateTime<Utc>> {
    use crate::models::Schedule;
    match schedule {
        Schedule::Interval { minutes } => {
            Some(after + chrono::Duration::minutes(*minutes as i64))
        }
        Schedule::Cron { expression } => {
            use std::str::FromStr;
            match cron::Schedule::from_str(expression) {
                Ok(sched) => sched.after(&after).next(),
                Err(_) => None,
            }
        }
        Schedule::OnChange | Schedule::Manual => None,
    }
}
