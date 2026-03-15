use std::sync::Arc;
use std::io::Read;
use chrono::Utc;
use sqlx::SqlitePool;
use sha2::{Sha256, Digest};
use globset::{Glob, GlobSetBuilder};

use tauri::Emitter;

use crate::models::{Source, Destination, SourceType, LogEntry};
use crate::db::queries;
use crate::engine::versioning;

pub struct CopyJob {
    pub source: Source,
    pub destination: Destination,
    pub trigger: String,
    pub app: Option<tauri::AppHandle>,
}

/// System-owned directories that should never be used as backup destinations.
#[cfg(unix)]
const BLOCKED_DEST_PREFIXES: &[&str] = &[
    "/System", "/usr", "/bin", "/sbin", "/proc", "/sys", "/dev", "/boot",
];
#[cfg(windows)]
const BLOCKED_DEST_PREFIXES: &[&str] = &[
    "C:\\Windows", "C:\\Program Files", "C:\\System Volume Information",
];

// ── Helpers ──────────────────────────────────────────────────────────────────

fn compute_file_hash(path: &std::path::Path) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn build_exclusion_set(patterns: &[String]) -> globset::GlobSet {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        if let Ok(glob) = Glob::new(p) {
            builder.add(glob);
        } else {
            log::warn!("Invalid exclusion pattern: {}", p);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSetBuilder::new().build().unwrap())
}

// ── CopyJob ──────────────────────────────────────────────────────────────────

impl CopyJob {
    /// Validates source and destination paths before attempting a copy.
    fn validate_paths(&self) -> anyhow::Result<()> {
        let src = std::path::Path::new(&self.source.path);
        let dst = std::path::Path::new(&self.destination.path);

        if !src.exists() {
            anyhow::bail!("Kaynak yol bulunamadı: {}", self.source.path);
        }

        let src_canonical = src.canonicalize()?;

        // Canonicalize the nearest existing ancestor of destination
        let dst_canonical = {
            let mut check = dst;
            loop {
                if check.exists() {
                    break check.canonicalize()?
                        .join(dst.strip_prefix(check).unwrap_or(std::path::Path::new("")));
                }
                match check.parent() {
                    Some(p) => check = p,
                    None => break dst.to_path_buf(),
                }
            }
        };

        // Circular copy guards
        if dst_canonical.starts_with(&src_canonical) {
            anyhow::bail!("Hedef yol kaynak klasörün içinde olamaz: {}", self.destination.path);
        }
        if src_canonical.starts_with(&dst_canonical) {
            anyhow::bail!("Kaynak yol hedef klasörün içinde olamaz: {}", self.source.path);
        }

        // Block protected system directories
        for prefix in BLOCKED_DEST_PREFIXES {
            if dst_canonical.starts_with(prefix) {
                anyhow::bail!("Hedef yol korumalı bir sistem dizini içinde: {}", prefix);
            }
        }

        Ok(())
    }

    pub async fn execute(&self, db: Arc<SqlitePool>) -> anyhow::Result<LogEntry> {
        self.validate_paths()?;

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
        )
        .await?;

        let version_path = versioning::compute_version_path(
            &self.destination.path,
            &self.source.name,
            &self.destination.retention.naming,
            started_at,
        );

        let destination_path_str = version_path.to_string_lossy().to_string();

        let copy_result = match self.check_disk_space() {
            Err(e) => Err(e),
            Ok(()) => {
                let mut result = Err(anyhow::anyhow!("Copy did not start"));
                for attempt in 1u32..=3 {
                    result = self.do_copy(&version_path).await;
                    if result.is_ok() { break; }
                    if attempt < 3 {
                        log::warn!(
                            "Copy attempt {}/3 failed for destination {}: {}, retrying in 5s...",
                            attempt, self.destination.id, result.as_ref().unwrap_err()
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
                result
            }
        };

        let ended_at = Utc::now();

        match copy_result {
            Ok((bytes_copied, files_copied, checksum)) => {
                let checksum_ref = checksum.as_deref();
                queries::update_log_entry_completed(
                    &db, log_id, ended_at, "Success",
                    Some(bytes_copied), Some(files_copied),
                    None, checksum_ref,
                )
                .await?;

                if let Err(e) = versioning::apply_retention(
                    &self.destination.path,
                    &self.source.name,
                    self.destination.retention.max_versions,
                )
                .await
                {
                    log::warn!("Retention policy failed for destination {}: {}", self.destination.id, e);
                }

                let next_run = compute_next_run(&self.destination.schedule, ended_at);
                queries::update_destination_run_status(
                    &db, &self.destination.id, ended_at, "Success", next_run,
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
                    checksum,
                })
            }
            Err(e) => {
                let error_msg = e.to_string();

                queries::update_log_entry_completed(
                    &db, log_id, ended_at, "Failed",
                    None, None, Some(&error_msg), None,
                )
                .await?;

                let next_run = compute_next_run(&self.destination.schedule, ended_at);
                queries::update_destination_run_status(
                    &db, &self.destination.id, ended_at, "Failed", next_run,
                )
                .await?;

                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    fn check_disk_space(&self) -> anyhow::Result<()> {
        use sysinfo::Disks;

        let source_size: u64 = match &self.source.source_type {
            SourceType::File => std::fs::metadata(&self.source.path)
                .map(|m| m.len())
                .unwrap_or(0),
            SourceType::Directory => count_dir_stats(std::path::Path::new(&self.source.path))
                .map(|(bytes, _)| bytes as u64)
                .unwrap_or(0),
        };

        if source_size == 0 { return Ok(()); }

        let disks = Disks::new_with_refreshed_list();
        let dest = std::path::Path::new(&self.destination.path);

        let available = disks
            .iter()
            .filter(|d| dest.starts_with(d.mount_point()))
            .max_by_key(|d| d.mount_point().components().count())
            .map(|d| d.available_space())
            .unwrap_or(u64::MAX);

        let required = source_size + source_size / 10;
        if available < required {
            anyhow::bail!(
                "Disk space insufficient: need {} bytes, only {} available at destination",
                required, available
            );
        }

        Ok(())
    }

    fn emit_progress(&self, files_done: i32, files_total: i32, bytes_done: i64) {
        if let Some(app) = &self.app {
            let _ = app.emit("copy-progress", serde_json::json!({
                "destination_id": &self.destination.id,
                "files_done": files_done,
                "files_total": files_total,
                "bytes_done": bytes_done,
            }));
        }
    }

    /// Returns (bytes_copied, files_copied, checksum_string)
    async fn do_copy(&self, version_path: &std::path::Path) -> anyhow::Result<(i64, i32, Option<String>)> {
        std::fs::create_dir_all(version_path)?;

        let exclusion_set = build_exclusion_set(&self.destination.exclusions);

        match &self.source.source_type {
            SourceType::File => {
                let source_path = std::path::Path::new(&self.source.path);
                let file_name = source_path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Cannot determine file name from source path"))?;
                let dest_file = version_path.join(file_name);

                self.emit_progress(0, 1, 0);
                let bytes = std::fs::copy(source_path, &dest_file)?;

                // SHA-256 integrity check
                let src_hash = compute_file_hash(source_path)?;
                let dst_hash = compute_file_hash(&dest_file)?;
                if src_hash != dst_hash {
                    std::fs::remove_file(&dest_file).ok();
                    anyhow::bail!(
                        "Bütünlük doğrulaması başarısız: kaynak ve hedef SHA-256 değerleri eşleşmiyor"
                    );
                }

                self.emit_progress(1, 1, bytes as i64);
                Ok((bytes as i64, 1, Some(src_hash)))
            }
            SourceType::Directory => {
                let source_path = std::path::Path::new(&self.source.path);

                // Collect all entries first so we know totals for progress
                let mut file_entries: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();
                let mut dir_entries: Vec<std::path::PathBuf> = Vec::new();

                for entry in walkdir::WalkDir::new(source_path) {
                    let entry = entry?;
                    let rel_path = entry.path()
                        .strip_prefix(source_path)
                        .unwrap_or(entry.path());

                    if rel_path == std::path::Path::new("") { continue; }
                    if exclusion_set.is_match(rel_path) {
                        log::debug!("Excluded: {}", rel_path.display());
                        continue;
                    }

                    let dest_entry = version_path.join(rel_path);
                    if entry.file_type().is_dir() {
                        dir_entries.push(dest_entry);
                    } else if entry.file_type().is_file() {
                        file_entries.push((entry.path().to_path_buf(), dest_entry));
                    }
                }

                let files_total = file_entries.len() as i32;
                self.emit_progress(0, files_total, 0);

                // Create directories
                for dir in &dir_entries {
                    std::fs::create_dir_all(dir)?;
                }

                // Copy files with progress
                let mut total_bytes: i64 = 0;
                let mut total_files: i32 = 0;
                let mut bytes_done: i64 = 0;

                for (src_path, dst_path) in &file_entries {
                    if let Some(parent) = dst_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    let bytes = std::fs::copy(src_path, dst_path)?;
                    total_bytes += bytes as i64;
                    total_files += 1;
                    bytes_done += bytes as i64;
                    self.emit_progress(total_files, files_total, bytes_done);
                }

                // Verify: destination file count must match source
                let (dst_bytes, dst_files) = count_dir_stats(version_path)?;
                if dst_files != total_files {
                    anyhow::bail!(
                        "Bütünlük doğrulaması başarısız: kopyalanan {} dosyadan {} hedefte bulunamadı",
                        total_files, dst_files
                    );
                }

                let checksum = Some(format!("{} dosya, {} bayt doğrulandı", dst_files, dst_bytes));
                Ok((total_bytes, total_files, checksum))
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
