use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use globset::{Glob, GlobSetBuilder};
use sqlx::SqlitePool;
use std::sync::Arc;

use tauri::Emitter;

use crate::crypto_utils::hw_decrypt;
use crate::db::queries;
use crate::engine::block::repository::Repository;
use crate::engine::block::snapshot::{BackupLevel, EncryptionConfig};
use crate::engine::block::store::LocalBlockStore;
use crate::models::{Destination, LogEntry, Source, SourceType};

pub struct CopyJob {
    pub source: Source,
    pub destination: Destination,
    pub trigger: String,
    pub app: Option<tauri::AppHandle>,
    /// Explicitly set backup level (from scheduler). None = auto-detect.
    pub backup_level: Option<BackupLevel>,
}

/// System-owned directories that should never be used as backup destinations.
#[cfg(unix)]
const BLOCKED_DEST_PREFIXES: &[&str] = &[
    "/System", "/usr", "/bin", "/sbin", "/proc", "/sys", "/dev", "/boot",
];
#[cfg(windows)]
const BLOCKED_DEST_PREFIXES: &[&str] = &[
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\System Volume Information",
];

// ── Backup encryption helpers ─────────────────────────────────────────────────

/// Decrypts the hardware-ID-protected stored password, derives Argon2id key.
fn derive_backup_key(encrypt_password_enc: &str, encrypt_salt: &str) -> anyhow::Result<[u8; 32]> {
    use argon2::{Argon2, Params, Version};

    // 1. Decrypt the stored password using HW key
    let password_bytes = hw_decrypt(encrypt_password_enc)
        .ok_or_else(|| anyhow::anyhow!("Failed to decrypt backup password"))?;
    let password = String::from_utf8(password_bytes)?;

    // 2. Derive Argon2id key from password + salt
    let salt_bytes = BASE64.decode(encrypt_salt)?;
    let params =
        Params::new(65536, 3, 4, Some(32)).map_err(|e| anyhow::anyhow!("Argon2 params: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);
    let mut master_key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), &salt_bytes, &mut master_key)
        .map_err(|e| anyhow::anyhow!("Argon2 hash: {e}"))?;
    Ok(master_key)
}

/// Derives Argon2id key directly from a plaintext password + base64 salt.
pub fn derive_backup_key_from_password(
    password: &str,
    encrypt_salt: &str,
) -> anyhow::Result<[u8; 32]> {
    use argon2::{Argon2, Params, Version};
    let salt_bytes = BASE64.decode(encrypt_salt)?;
    let params =
        Params::new(65536, 3, 4, Some(32)).map_err(|e| anyhow::anyhow!("Argon2 params: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);
    let mut master_key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), &salt_bytes, &mut master_key)
        .map_err(|e| anyhow::anyhow!("Argon2 hash: {e}"))?;
    Ok(master_key)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn build_exclusion_set(patterns: &[String]) -> globset::GlobSet {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        if let Ok(glob) = Glob::new(p) {
            builder.add(glob);
        } else {
            log::warn!("Invalid exclusion pattern: {}", p);
        }
    }
    builder
        .build()
        .unwrap_or_else(|_| GlobSetBuilder::new().build().unwrap())
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

        // Reject destination if it is a symlink (symlink attack prevention)
        if dst.exists() {
            let meta = std::fs::symlink_metadata(dst)?;
            if meta.file_type().is_symlink() {
                anyhow::bail!(
                    "Hedef yol bir sembolik bağ — güvenlik ihlali: {}",
                    self.destination.path
                );
            }
        }

        let src_canonical = src.canonicalize()?;

        // Canonicalize the nearest existing ancestor of destination
        let dst_canonical = {
            let mut check = dst;
            loop {
                if check.exists() {
                    break check
                        .canonicalize()?
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
            anyhow::bail!(
                "Hedef yol kaynak klasörün içinde olamaz: {}",
                self.destination.path
            );
        }
        if src_canonical.starts_with(&dst_canonical) {
            anyhow::bail!(
                "Kaynak yol hedef klasörün içinde olamaz: {}",
                self.source.path
            );
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
        // Remote destinations: delegate to appropriate copier
        match self.destination.destination_type {
            crate::models::DestinationType::S3 | crate::models::DestinationType::R2 => {
                return crate::engine::cloud_copier::CloudCopyJob {
                    source: self.source.clone(),
                    destination: self.destination.clone(),
                    trigger: self.trigger.clone(),
                    app: self.app.clone(),
                }
                .execute(db)
                .await;
            }
            crate::models::DestinationType::Sftp => {
                return crate::engine::sftp_copier::SftpCopyJob {
                    source: self.source.clone(),
                    destination: self.destination.clone(),
                    trigger: self.trigger.clone(),
                    app: self.app.clone(),
                }
                .execute(db)
                .await;
            }
            crate::models::DestinationType::OneDrive
            | crate::models::DestinationType::GoogleDrive
            | crate::models::DestinationType::Dropbox => {
                return crate::engine::oauth_copier::OAuthCopyJob {
                    source: self.source.clone(),
                    destination: self.destination.clone(),
                    trigger: self.trigger.clone(),
                    app: self.app.clone(),
                }
                .execute(db)
                .await;
            }
            crate::models::DestinationType::WebDav => {
                return crate::engine::webdav_copier::WebDavCopyJob {
                    source: self.source.clone(),
                    destination: self.destination.clone(),
                    trigger: self.trigger.clone(),
                    app: self.app.clone(),
                }
                .execute(db)
                .await;
            }
            crate::models::DestinationType::Local => {}
        }

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
            None,
            None,
        )
        .await?;

        let destination_path_str = self.destination.path.clone();

        // ── Block-Level Delta Backup ─────────────────────────────────────
        let copy_result = match self.check_disk_space() {
            Err(e) => Err(e),
            Ok(()) => self.do_block_backup().await,
        };

        let ended_at = Utc::now();

        match copy_result {
            Ok(snapshot) => {
                let files_copied = snapshot.files.len() as i32;
                let bytes_copied = snapshot.total_size as i64;
                let savings_pct = (snapshot.savings_ratio() * 100.0) as u32;
                let checksum = Some(format!(
                    "{} dosya, {} blok ({} değişen), {} %{} tasarruf, {} byte kazanç",
                    files_copied,
                    snapshot.total_blocks,
                    snapshot.changed_blocks,
                    snapshot.level,
                    savings_pct,
                    snapshot.total_size as i64 - snapshot.changed_bytes as i64,
                ));

                let checksum_ref = checksum.as_deref();
                queries::update_log_entry_completed(
                    &db,
                    log_id,
                    ended_at,
                    "Success",
                    Some(bytes_copied),
                    Some(files_copied),
                    None,
                    checksum_ref,
                    Some(&format!("{:?}", snapshot.level)),
                    Some(&snapshot.id),
                )
                .await?;

                // Prune old backup sets based on retention policy
                {
                    let store = Box::new(LocalBlockStore::new(&self.destination.path));
                    let encryption_key = self.get_encryption_key();
                    if let Ok(mut repo) =
                        Repository::open_or_init(store, encryption_key, None).await
                    {
                        let keep = self.destination.retention.max_versions as u32;
                        if let Err(e) = repo.prune(keep).await {
                            log::warn!("Block prune failed for {}: {}", self.destination.id, e);
                        }
                    }
                }

                let next_run = compute_next_run(&self.destination.schedule, ended_at);
                queries::update_destination_run_status(
                    &db,
                    &self.destination.id,
                    ended_at,
                    "Success",
                    next_run,
                )
                .await?;

                // Send email notification (best-effort, non-blocking)
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

                log::info!(
                    "Block backup completed: {} files, {} blocks ({} changed), {} {}% savings",
                    files_copied,
                    snapshot.total_blocks,
                    snapshot.changed_blocks,
                    snapshot.level,
                    savings_pct
                );

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
                    backup_level: Some(format!("{:?}", snapshot.level)),
                    snapshot_id: Some(snapshot.id),
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

                let next_run = compute_next_run(&self.destination.schedule, ended_at);
                queries::update_destination_run_status(
                    &db,
                    &self.destination.id,
                    ended_at,
                    "Failed",
                    next_run,
                )
                .await?;

                // Send email notification (best-effort, non-blocking)
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

    fn check_disk_space(&self) -> anyhow::Result<()> {
        use sysinfo::Disks;

        let source_size: u64 = match &self.source.source_type {
            SourceType::File => std::fs::metadata(&self.source.path)
                .map(|m| m.len())
                .unwrap_or(0),
            SourceType::Directory => {
                let mut total: u64 = 0;
                for entry in walkdir::WalkDir::new(&self.source.path) {
                    if let Ok(e) = entry {
                        if e.file_type().is_file() {
                            total += e.metadata().map(|m| m.len()).unwrap_or(0);
                        }
                    }
                }
                total
            }
        };

        if source_size == 0 {
            return Ok(());
        }

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
                required,
                available
            );
        }

        Ok(())
    }

    /// Derives the encryption key from the destination's stored password, if encryption is enabled.
    fn get_encryption_key(&self) -> Option<[u8; 32]> {
        if !self.destination.encrypt {
            return None;
        }
        if let (Some(ref enc_pwd), Some(ref enc_salt)) = (
            &self.destination.encrypt_password_enc,
            &self.destination.encrypt_salt,
        ) {
            derive_backup_key(enc_pwd, enc_salt).ok()
        } else {
            None
        }
    }

    /// Performs a block-level delta backup. Returns the snapshot with savings stats.
    async fn do_block_backup(&self) -> anyhow::Result<crate::engine::block::snapshot::Snapshot> {
        let source_path = std::path::Path::new(&self.source.path);
        let exclusion_set = build_exclusion_set(&self.destination.exclusions);

        let encryption_key = self.get_encryption_key();
        let enc_config = if let (Some(_key), Some(ref salt)) =
            (encryption_key, &self.destination.encrypt_salt)
        {
            Some(EncryptionConfig {
                algorithm: "AES-256-GCM".into(),
                argon2_salt: salt.clone(),
                argon2_m_cost: 65536,
                argon2_t_cost: 3,
                argon2_p_cost: 4,
            })
        } else {
            None
        };

        let store = Box::new(LocalBlockStore::new(&self.destination.path));
        let mut repo = Repository::open_or_init(store, encryption_key, enc_config).await?;

        // Determine backup level:
        // 1. Explicitly set by scheduler → use it
        // 2. Manual trigger → Level 0 (full)
        // Safety: if Level 1 requested but no Level 0 exists, force Level 0
        let mut level = self.backup_level.unwrap_or_else(|| {
            if self.destination.incremental {
                BackupLevel::Level1Cumulative
            } else {
                BackupLevel::Level0
            }
        });

        if matches!(
            level,
            BackupLevel::Level1Cumulative | BackupLevel::Level1Differential
        ) {
            let snapshots = repo.list_snapshots().await?;
            let has_level0 = snapshots
                .iter()
                .any(|s| s.source_name == self.source.name && s.level == BackupLevel::Level0);
            if !has_level0 {
                log::info!(
                    "No Level 0 found for '{}', forcing Level 0",
                    self.source.name
                );
                level = BackupLevel::Level0;
            }
        }

        let progress = TauriProgressReporter {
            app: self.app.clone(),
            destination_id: self.destination.id.clone(),
        };

        let snapshot = repo
            .backup(
                source_path,
                &self.source.name,
                &self.source.source_type,
                &exclusion_set,
                level,
                &progress,
            )
            .await?;

        Ok(snapshot)
    }
}

/// Bridges block backup progress events to Tauri's event system.
struct TauriProgressReporter {
    app: Option<tauri::AppHandle>,
    destination_id: String,
}

impl crate::engine::block::repository::ProgressReporter for TauriProgressReporter {
    fn on_file_start(&self, _path: &str, file_index: u32, total_files: u32) {
        if let Some(app) = &self.app {
            let _ = app.emit(
                "copy-progress",
                serde_json::json!({
                    "destination_id": &self.destination_id,
                    "files_done": file_index,
                    "files_total": total_files,
                    "bytes_done": 0,
                    "bytes_total": 0,
                }),
            );
        }
    }

    fn on_file_done(&self, _path: &str, file_index: u32, total_files: u32, bytes: u64) {
        if let Some(app) = &self.app {
            let _ = app.emit(
                "copy-progress",
                serde_json::json!({
                    "destination_id": &self.destination_id,
                    "files_done": file_index + 1,
                    "files_total": total_files,
                    "bytes_done": bytes,
                    "bytes_total": 0,
                }),
            );
        }
    }

    fn on_block_stored(&self, _block_index: u32, _size: u32, _is_changed: bool) {}
}

pub(crate) fn compute_next_run(
    schedule: &crate::models::Schedule,
    after: chrono::DateTime<Utc>,
) -> Option<chrono::DateTime<Utc>> {
    use crate::models::Schedule;
    match schedule {
        Schedule::Interval { minutes } => Some(after + chrono::Duration::minutes(*minutes as i64)),
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

pub fn compute_next_run_pub(
    schedule: &crate::models::Schedule,
    after: chrono::DateTime<Utc>,
) -> Option<chrono::DateTime<Utc>> {
    compute_next_run(schedule, after)
}

// ── Integration tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::schedule::{RetentionPolicy, Schedule, VersionNaming};
    use crate::models::{Destination, DestinationType, Source, SourceType};
    use chrono::Utc;
    use tempfile::TempDir;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn make_source(path: &str, source_type: SourceType) -> Source {
        Source {
            id: "test-src-id".to_string(),
            name: "Test Source".to_string(),
            path: path.to_string(),
            source_type,
            enabled: true,
            created_at: Utc::now(),
            destinations: vec![],
        }
    }

    fn make_destination(path: &str) -> Destination {
        Destination {
            id: "test-dst-id".to_string(),
            source_id: "test-src-id".to_string(),
            path: path.to_string(),
            schedule: Schedule::Manual,
            retention: RetentionPolicy {
                max_versions: 5,
                naming: VersionNaming::Timestamp,
            },
            exclusions: vec![],
            enabled: true,
            incremental: false,
            last_run: None,
            last_status: None,
            next_run: None,
            destination_type: DestinationType::Local,
            cloud_config: None,
            sftp_config: None,
            oauth_config: None,
            webdav_config: None,
            level1_enabled: false,
            level1_schedule: None,
            level1_type: "Cumulative".to_string(),
            level1_last_run: None,
            level1_next_run: None,
            encrypt: false,
            encrypt_password_enc: None,
            encrypt_salt: None,
        }
    }

    fn make_job(source: Source, destination: Destination) -> CopyJob {
        CopyJob {
            source,
            destination,
            trigger: "Manual".to_string(),
            app: None,
            backup_level: None,
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_block_backup_single_file() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        let src_file = src_dir.path().join("hello.txt");
        std::fs::write(&src_file, b"hello world").unwrap();

        let src = make_source(src_file.to_str().unwrap(), SourceType::File);
        let dst = make_destination(dst_dir.path().to_str().unwrap());
        let job = make_job(src, dst);

        let snapshot = job.do_block_backup().await.unwrap();

        assert_eq!(snapshot.files.len(), 1);
        assert_eq!(snapshot.total_size, 11);
        assert_eq!(snapshot.changed_blocks, snapshot.total_blocks);
        assert_eq!(snapshot.level, BackupLevel::Level0);
    }

    #[tokio::test]
    async fn test_block_backup_directory() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        std::fs::write(src_dir.path().join("a.txt"), b"aaa").unwrap();
        std::fs::write(src_dir.path().join("b.txt"), b"bbbb").unwrap();
        let sub = src_dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("c.txt"), b"ccccc").unwrap();

        let src = make_source(src_dir.path().to_str().unwrap(), SourceType::Directory);
        let dst = make_destination(dst_dir.path().to_str().unwrap());
        let job = make_job(src, dst);

        let snapshot = job.do_block_backup().await.unwrap();

        assert_eq!(snapshot.files.len(), 3);
        assert_eq!(snapshot.total_size, 12); // 3 + 4 + 5
    }

    #[tokio::test]
    async fn test_block_incremental_second_backup_zero_changed() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        std::fs::write(src_dir.path().join("data.txt"), b"hello block world").unwrap();

        let src = make_source(src_dir.path().to_str().unwrap(), SourceType::Directory);
        let mut dst = make_destination(dst_dir.path().to_str().unwrap());
        dst.incremental = true;

        // First backup — auto Level 0 (no previous Level 0 exists)
        let job1 = make_job(src.clone(), dst.clone());
        let snap1 = job1.do_block_backup().await.unwrap();
        assert_eq!(snap1.level, BackupLevel::Level0);
        assert!(snap1.changed_blocks > 0, "First backup should store blocks");

        // Second backup — Level 1 Cumulative, same data, zero changed
        let job2 = make_job(src, dst);
        let snap2 = job2.do_block_backup().await.unwrap();
        assert_eq!(snap2.level, BackupLevel::Level1Cumulative);
        assert_eq!(
            snap2.changed_blocks, 0,
            "Second backup should have zero changed blocks"
        );
        assert_eq!(
            snap2.changed_bytes, 0,
            "Second backup should write zero new bytes"
        );
    }

    #[tokio::test]
    async fn test_block_backup_with_exclusion() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        std::fs::write(src_dir.path().join("keep.txt"), b"keep").unwrap();
        std::fs::write(src_dir.path().join("skip.log"), b"skip").unwrap();

        let src = make_source(src_dir.path().to_str().unwrap(), SourceType::Directory);
        let mut dst = make_destination(dst_dir.path().to_str().unwrap());
        dst.exclusions = vec!["*.log".to_string()];
        let job = make_job(src, dst);

        let snapshot = job.do_block_backup().await.unwrap();

        assert_eq!(snapshot.files.len(), 1);
        assert_eq!(snapshot.files[0].path, "keep.txt");
    }

    #[tokio::test]
    async fn test_validate_missing_source() {
        let dst_dir = TempDir::new().unwrap();
        let src = make_source("/nonexistent/path/that/does/not/exist", SourceType::File);
        let dst = make_destination(dst_dir.path().to_str().unwrap());
        let job = make_job(src, dst);

        let result = job.validate_paths();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("bulunamadı") || msg.contains("not found") || msg.contains("exist"));
    }

    #[tokio::test]
    async fn test_validate_destination_inside_source_rejected() {
        let src_dir = TempDir::new().unwrap();
        let dst_path = src_dir.path().join("backup");
        std::fs::create_dir_all(&dst_path).unwrap();

        let src = make_source(src_dir.path().to_str().unwrap(), SourceType::Directory);
        let dst = make_destination(dst_path.to_str().unwrap());
        let job = make_job(src, dst);

        let result = job.validate_paths();
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_next_run_interval() {
        use crate::models::Schedule;
        let after = Utc::now();
        let next = compute_next_run(&Schedule::Interval { minutes: 30 }, after).unwrap();
        let diff = (next - after).num_minutes();
        assert_eq!(diff, 30);
    }

    #[test]
    fn test_compute_next_run_manual_returns_none() {
        use crate::models::Schedule;
        let result = compute_next_run(&Schedule::Manual, Utc::now());
        assert!(result.is_none());
    }

    #[test]
    fn test_build_exclusion_set_matches() {
        let patterns = vec!["*.log".to_string(), ".git/**".to_string()];
        let set = build_exclusion_set(&patterns);
        assert!(set.is_match("error.log"));
        assert!(set.is_match(".git/config"));
        assert!(!set.is_match("main.rs"));
    }
}
