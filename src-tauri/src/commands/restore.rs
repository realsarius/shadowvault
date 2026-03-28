use std::path::Path;

use chrono::Utc;
use dashmap::mapref::entry::Entry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::State;

use crate::db::queries;
use crate::engine::block::snapshot::{BackupLevel, Snapshot};
use crate::engine::block::store::{BlockStore, LocalBlockStore};
use crate::AppState;

/// Restores a versioned backup to its original (or specified) location.
///
/// `backup_path`  — path to the versioned backup directory/file (from copy_logs.destination_path)
/// `restore_to`   — target path to restore into (typically the original source_path)

#[cfg(unix)]
const BLOCKED_RESTORE_PREFIXES: &[&str] = &[
    "/System", "/usr", "/bin", "/sbin", "/proc", "/sys", "/dev", "/boot",
];
#[cfg(windows)]
const BLOCKED_RESTORE_PREFIXES: &[&str] = &[
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\System Volume Information",
];

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RestoreDryRunResult {
    pub mode: String,
    pub backup_path: String,
    pub restore_to: String,
    pub files_to_restore: u64,
    pub bytes_to_restore: u64,
    pub blocked: bool,
    pub notes: Vec<String>,
    pub snapshot_id: Option<String>,
    pub backup_level: Option<String>,
    pub error_code: Option<RestoreErrorCode>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct VerifyBackupResult {
    pub destination_id: String,
    pub source_id: String,
    pub snapshot_id: String,
    pub source_name: String,
    pub backup_level: String,
    pub chain_depth: u32,
    pub files_checked: u32,
    pub blocks_checked: u32,
    pub total_bytes: u64,
    pub snapshot_digest: String,
    pub verified_at: String,
    pub error_code: Option<RestoreErrorCode>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, specta::Type, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RestoreErrorCode {
    BlockedPath,
    MissingSnapshot,
    WrongPassword,
    ChainIncomplete,
    IoFailure,
}

impl RestoreErrorCode {
    fn as_str(&self) -> &'static str {
        match self {
            RestoreErrorCode::BlockedPath => "blocked_path",
            RestoreErrorCode::MissingSnapshot => "missing_snapshot",
            RestoreErrorCode::WrongPassword => "wrong_password",
            RestoreErrorCode::ChainIncomplete => "chain_incomplete",
            RestoreErrorCode::IoFailure => "io_failure",
        }
    }
}

#[derive(Debug, Serialize)]
struct CommandErrorPayload {
    error_code: RestoreErrorCode,
    message: String,
}

fn command_error(error_code: RestoreErrorCode, message: impl Into<String>) -> String {
    let payload = CommandErrorPayload {
        error_code,
        message: message.into(),
    };

    serde_json::to_string(&payload)
        .unwrap_or_else(|_| format!("{}: {}", error_code.as_str(), payload.message))
}

fn classify_restore_error(message: &str) -> RestoreErrorCode {
    let msg = message.to_lowercase();
    if msg.contains("sistem dizin") || msg.contains("güvenlik ihlali") || msg.contains("blocked") {
        return RestoreErrorCode::BlockedPath;
    }
    if msg.contains("snapshot") && (msg.contains("bulunamadı") || msg.contains("not found")) {
        return RestoreErrorCode::MissingSnapshot;
    }
    if msg.contains("zincir") || msg.contains("parent level 0") || msg.contains("chain") {
        return RestoreErrorCode::ChainIncomplete;
    }
    if msg.contains("şifre")
        || msg.contains("password")
        || msg.contains("argon2")
        || msg.contains("decrypt")
    {
        return RestoreErrorCode::WrongPassword;
    }
    RestoreErrorCode::IoFailure
}

fn is_restore_blocked(restore_to: &str) -> bool {
    BLOCKED_RESTORE_PREFIXES
        .iter()
        .any(|prefix| restore_to.starts_with(prefix))
}

fn estimate_legacy_restore(backup_path: &Path) -> Result<(u64, u64), String> {
    if backup_path.is_file() {
        let bytes = std::fs::metadata(backup_path)
            .map(|m| m.len())
            .map_err(|e| e.to_string())?;
        return Ok((1, bytes));
    }

    let mut files = 0u64;
    let mut bytes = 0u64;
    for entry in walkdir::WalkDir::new(backup_path) {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().is_file() {
            files += 1;
            bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }

    Ok((files, bytes))
}

async fn compute_chain_depth(store: &LocalBlockStore, snapshot: &Snapshot) -> Result<u32, String> {
    let mut depth = 1u32;

    match snapshot.level {
        BackupLevel::Level0 => return Ok(depth),
        BackupLevel::Level1Cumulative => {
            if let Some(level0_id) = &snapshot.parent_level0_id {
                store
                    .get_snapshot(level0_id)
                    .await
                    .map_err(|e| format!("Parent Level 0 yüklenemedi: {}", e))?;
                depth += 1;
            }
            return Ok(depth);
        }
        BackupLevel::Level1Differential => {}
    }

    let mut current = snapshot.clone();
    loop {
        let parent_id = match current.parent_id.clone() {
            Some(id) => id,
            None => break,
        };
        let parent = store
            .get_snapshot(&parent_id)
            .await
            .map_err(|e| format!("Snapshot zinciri yüklenemedi: {}", e))?;
        depth += 1;
        if parent.level == BackupLevel::Level0 {
            break;
        }
        current = parent;
    }

    Ok(depth)
}

fn compute_snapshot_digest(snapshot: &Snapshot) -> String {
    let mut lines: Vec<String> = snapshot
        .files
        .iter()
        .map(|f| format!("{}:{}", f.path, f.file_hash))
        .collect();
    lines.sort();

    let mut hasher = Sha256::new();
    for line in lines {
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

#[tauri::command]
#[specta::specta]
pub async fn restore_dry_run(
    backup_path: String,
    restore_to: String,
) -> Result<RestoreDryRunResult, String> {
    let src = Path::new(&backup_path);
    if !src.exists() {
        return Err(command_error(
            RestoreErrorCode::MissingSnapshot,
            format!("Yedek bulunamadı: {}", backup_path),
        ));
    }

    let (files_to_restore, bytes_to_restore) =
        estimate_legacy_restore(src).map_err(|e| command_error(RestoreErrorCode::IoFailure, e))?;
    let blocked = is_restore_blocked(&restore_to);
    let mut notes = Vec::new();
    if blocked {
        notes.push("Hedef yol sistem dizini kısıtına takılıyor.".to_string());
    }
    if src.is_file() {
        notes.push("Tek dosya geri yüklenecek.".to_string());
    } else {
        notes.push("Klasör içeriği hedef klasöre birleştirilerek geri yüklenecek.".to_string());
    }

    Ok(RestoreDryRunResult {
        mode: "VersionedCopy".to_string(),
        backup_path,
        restore_to,
        files_to_restore,
        bytes_to_restore,
        blocked,
        notes,
        snapshot_id: None,
        backup_level: None,
        error_code: if blocked {
            Some(RestoreErrorCode::BlockedPath)
        } else {
            None
        },
    })
}

#[tauri::command]
#[specta::specta]
pub async fn restore_block_dry_run(
    destination_path: String,
    snapshot_id: String,
    restore_to: String,
) -> Result<RestoreDryRunResult, String> {
    let store = LocalBlockStore::new(&destination_path);
    let snapshot = store.get_snapshot(&snapshot_id).await.map_err(|e| {
        command_error(
            RestoreErrorCode::MissingSnapshot,
            format!("Snapshot bulunamadı: {}", e),
        )
    })?;
    let blocked = is_restore_blocked(&restore_to);

    let mut notes = vec![
        format!("Zincir seviyesi: {:?}", snapshot.level),
        format!(
            "{} bloktan {} tanesi bu snapshot içinde saklanıyor.",
            snapshot.total_blocks, snapshot.changed_blocks
        ),
    ];
    if blocked {
        notes.push("Hedef yol sistem dizini kısıtına takılıyor.".to_string());
    }

    Ok(RestoreDryRunResult {
        mode: "BlockSnapshot".to_string(),
        backup_path: destination_path,
        restore_to,
        files_to_restore: snapshot.files.len() as u64,
        bytes_to_restore: snapshot.total_size,
        blocked,
        notes,
        snapshot_id: Some(snapshot_id),
        backup_level: Some(format!("{:?}", snapshot.level)),
        error_code: if blocked {
            Some(RestoreErrorCode::BlockedPath)
        } else {
            None
        },
    })
}

#[tauri::command]
#[specta::specta]
pub async fn restore_backup(backup_path: String, restore_to: String) -> Result<(), String> {
    let src = std::path::Path::new(&backup_path);
    let dst = std::path::Path::new(&restore_to);

    if !src.exists() {
        return Err(command_error(
            RestoreErrorCode::MissingSnapshot,
            format!("Yedek bulunamadı: {}", backup_path),
        ));
    }

    for prefix in BLOCKED_RESTORE_PREFIXES {
        if restore_to.starts_with(prefix) {
            return Err(command_error(
                RestoreErrorCode::BlockedPath,
                format!(
                    "Güvenlik ihlali: '{}' sistem dizinine geri yükleme yapılamaz.",
                    restore_to
                ),
            ));
        }
    }

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| command_error(RestoreErrorCode::IoFailure, e.to_string()))?;
    }

    if src.is_file() {
        std::fs::copy(src, dst)
            .map(|_| ())
            .map_err(|e| command_error(RestoreErrorCode::IoFailure, e.to_string()))?;
    } else {
        std::fs::create_dir_all(dst)
            .map_err(|e| command_error(RestoreErrorCode::IoFailure, e.to_string()))?;
        let options = fs_extra::dir::CopyOptions {
            overwrite: true,
            skip_exist: false,
            copy_inside: true,
            content_only: true,
            ..Default::default()
        };
        fs_extra::dir::copy(src, dst, &options)
            .map(|_| ())
            .map_err(|e| command_error(RestoreErrorCode::IoFailure, e.to_string()))?;
    }

    log::info!("Restored {} → {}", backup_path, restore_to);
    Ok(())
}

/// Restores a block-level snapshot to the specified location.
///
/// `destination_path` — path to the destination (contains `.shadowvault/` directory)
/// `snapshot_id`      — UUID of the snapshot to restore
/// `restore_to`       — target path to restore into
/// `password`         — optional plaintext password for encrypted backups
#[tauri::command]
#[specta::specta]
pub async fn restore_block_backup(
    destination_path: String,
    snapshot_id: String,
    restore_to: String,
    password: Option<String>,
) -> Result<(), String> {
    for prefix in BLOCKED_RESTORE_PREFIXES {
        if restore_to.starts_with(prefix) {
            return Err(command_error(
                RestoreErrorCode::BlockedPath,
                format!(
                    "Güvenlik ihlali: '{}' sistem dizinine geri yükleme yapılamaz.",
                    restore_to
                ),
            ));
        }
    }

    let store = Box::new(crate::engine::block::store::LocalBlockStore::new(
        &destination_path,
    ));

    let encryption_key = if let Some(ref pwd) = password {
        let config = store
            .load_config()
            .await
            .map_err(|e| {
                command_error(
                    RestoreErrorCode::IoFailure,
                    format!("Repo config yüklenemedi: {}", e),
                )
            })?
            .ok_or_else(|| command_error(RestoreErrorCode::IoFailure, "Repo config bulunamadı"))?;

        if let Some(ref enc) = config.encryption {
            let key = crate::engine::copier::derive_backup_key_from_password(pwd, &enc.argon2_salt)
                .map_err(|e| {
                    command_error(
                        RestoreErrorCode::WrongPassword,
                        format!("Şifre çözme anahtarı türetilemedi: {}", e),
                    )
                })?;
            Some(key)
        } else {
            None
        }
    } else {
        None
    };

    let repo =
        crate::engine::block::repository::Repository::open_or_init(store, encryption_key, None)
            .await
            .map_err(|e| {
                command_error(
                    RestoreErrorCode::IoFailure,
                    format!("Repo açılamadı: {}", e),
                )
            })?;

    let target_path = std::path::Path::new(&restore_to);
    std::fs::create_dir_all(target_path)
        .map_err(|e| command_error(RestoreErrorCode::IoFailure, e.to_string()))?;

    repo.restore(&snapshot_id, target_path).await.map_err(|e| {
        command_error(
            classify_restore_error(&e.to_string()),
            format!("Geri yükleme başarısız: {}", e),
        )
    })?;

    log::info!(
        "Block restore completed: snapshot {} → {}",
        snapshot_id,
        restore_to
    );
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn verify_backup(
    destination_id: Option<String>,
    snapshot_id: Option<String>,
    password: Option<String>,
    state: State<'_, AppState>,
) -> Result<VerifyBackupResult, String> {
    let canary_enabled = queries::get_setting(&state.db, "beta_canary_verify_enabled")
        .await
        .map_err(|e| command_error(RestoreErrorCode::IoFailure, e.to_string()))?
        .unwrap_or_else(|| "true".to_string());
    if canary_enabled.trim().eq_ignore_ascii_case("false") {
        return Err(command_error(
            RestoreErrorCode::IoFailure,
            "verify_backup canary cohort dışında devre dışı.",
        ));
    }

    let requested_snapshot = snapshot_id.clone();
    if destination_id.is_none() && requested_snapshot.is_none() {
        return Err(command_error(
            RestoreErrorCode::IoFailure,
            "destination_id veya snapshot_id verilmelidir.",
        ));
    }

    let (source, destination) = if let Some(dest_id) = destination_id.clone() {
        let dest = queries::get_destination_by_id(&state.db, &dest_id)
            .await
            .map_err(|e| command_error(RestoreErrorCode::IoFailure, e.to_string()))?
            .ok_or_else(|| {
                command_error(
                    RestoreErrorCode::IoFailure,
                    format!("Destination {} not found", dest_id),
                )
            })?;
        let src = queries::get_source_by_id(&state.db, &dest.source_id)
            .await
            .map_err(|e| command_error(RestoreErrorCode::IoFailure, e.to_string()))?
            .ok_or_else(|| {
                command_error(
                    RestoreErrorCode::IoFailure,
                    format!("Source {} not found", dest.source_id),
                )
            })?;
        (src, dest)
    } else {
        let wanted = requested_snapshot.as_ref().ok_or_else(|| {
            command_error(
                RestoreErrorCode::MissingSnapshot,
                "snapshot_id verilmelidir.",
            )
        })?;
        let sources = queries::get_all_sources(&state.db)
            .await
            .map_err(|e| command_error(RestoreErrorCode::IoFailure, e.to_string()))?;

        let mut found = None;
        for src in sources {
            for dest in &src.destinations {
                let manifest = Path::new(&dest.path)
                    .join(".shadowvault")
                    .join("snapshots")
                    .join(format!("{}.json", wanted));
                if manifest.exists() {
                    found = Some((src.clone(), dest.clone()));
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        found.ok_or_else(|| {
            command_error(
                RestoreErrorCode::MissingSnapshot,
                format!("Snapshot {} için destination bulunamadı.", wanted),
            )
        })?
    };

    if !matches!(
        destination.destination_type,
        crate::models::DestinationType::Local
    ) {
        return Err(command_error(
            RestoreErrorCode::IoFailure,
            "verify_backup yalnızca Local destination için desteklenir.",
        ));
    }

    let repo_config_path = Path::new(&destination.path)
        .join(".shadowvault")
        .join("config.json");
    if !repo_config_path.exists() {
        return Err(command_error(
            RestoreErrorCode::ChainIncomplete,
            "Block repository bulunamadı (.shadowvault/config.json).",
        ));
    }

    let config_store = LocalBlockStore::new(&destination.path);
    let config = config_store
        .load_config()
        .await
        .map_err(|e| {
            command_error(
                RestoreErrorCode::IoFailure,
                format!("Repo config yüklenemedi: {}", e),
            )
        })?
        .ok_or_else(|| command_error(RestoreErrorCode::IoFailure, "Repo config bulunamadı."))?;

    let encryption_key = if let Some(enc) = config.encryption {
        let pwd = password.as_deref().ok_or_else(|| {
            command_error(
                RestoreErrorCode::WrongPassword,
                "Bu yedek şifreli. verify_backup için password gerekli.",
            )
        })?;
        Some(
            crate::engine::copier::derive_backup_key_from_password(pwd, &enc.argon2_salt).map_err(
                |e| {
                    command_error(
                        RestoreErrorCode::WrongPassword,
                        format!("Şifre çözme anahtarı türetilemedi: {}", e),
                    )
                },
            )?,
        )
    } else {
        None
    };

    let repo = crate::engine::block::repository::Repository::open_or_init(
        Box::new(LocalBlockStore::new(&destination.path)),
        encryption_key,
        None,
    )
    .await
    .map_err(|e| {
        command_error(
            RestoreErrorCode::IoFailure,
            format!("Repo açılamadı: {}", e),
        )
    })?;

    let resolved_snapshot_id = if let Some(id) = requested_snapshot {
        id
    } else {
        let summaries = repo.list_snapshots().await.map_err(|e| {
            command_error(
                RestoreErrorCode::IoFailure,
                format!("Snapshot listesi alınamadı: {}", e),
            )
        })?;
        let selected = summaries
            .iter()
            .find(|s| s.source_name == source.name)
            .or_else(|| summaries.first())
            .ok_or_else(|| {
                command_error(
                    RestoreErrorCode::MissingSnapshot,
                    "Doğrulanacak snapshot bulunamadı.",
                )
            })?;
        selected.id.clone()
    };

    let meta_store = LocalBlockStore::new(&destination.path);
    let snapshot = meta_store
        .get_snapshot(&resolved_snapshot_id)
        .await
        .map_err(|e| {
            command_error(
                RestoreErrorCode::MissingSnapshot,
                format!("Snapshot yüklenemedi: {}", e),
            )
        })?;
    let backup_level = format!("{:?}", snapshot.level);

    let verify_key = format!("{}::{}", destination.id, resolved_snapshot_id);
    match state.verifying_jobs.entry(verify_key.clone()) {
        Entry::Occupied(_) => {
            let _ = queries::insert_skipped_log_entry(
                &state.db,
                &source.id,
                &destination.id,
                &source.path,
                &destination.path,
                "Verification",
                "Skipped: verification already running for destination+snapshot",
                Some(backup_level.as_str()),
                Some(resolved_snapshot_id.as_str()),
            )
            .await;
            return Err(command_error(
                RestoreErrorCode::IoFailure,
                "Aynı destination + snapshot için doğrulama zaten çalışıyor.",
            ));
        }
        Entry::Vacant(v) => {
            v.insert(std::time::Instant::now());
        }
    }

    let started_at = Utc::now();
    let log_id = match queries::insert_log_entry(
        &state.db,
        &source.id,
        &destination.id,
        &source.path,
        &destination.path,
        started_at,
        "Running",
        "Verification",
        Some(backup_level.as_str()),
        Some(resolved_snapshot_id.as_str()),
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            state.verifying_jobs.remove(&verify_key);
            return Err(command_error(
                RestoreErrorCode::IoFailure,
                format!("Doğrulama logu yazılamadı: {}", e),
            ));
        }
    };

    let verify_tmp_dir =
        std::env::temp_dir().join(format!("shadowvault-verify-{}", uuid::Uuid::new_v4()));
    if let Err(e) = std::fs::create_dir_all(&verify_tmp_dir) {
        let msg = e.to_string();
        let _ = queries::update_log_entry_completed(
            &state.db,
            log_id,
            Utc::now(),
            "Failed",
            None,
            None,
            Some(msg.as_str()),
            None,
            Some(backup_level.as_str()),
            Some(resolved_snapshot_id.as_str()),
        )
        .await;
        state.verifying_jobs.remove(&verify_key);
        return Err(command_error(RestoreErrorCode::IoFailure, msg));
    }

    let verify_result: Result<VerifyBackupResult, String> = async {
        repo.restore(&resolved_snapshot_id, &verify_tmp_dir)
            .await
            .map_err(|e| format!("Snapshot restore doğrulaması başarısız: {}", e))?;

        let mut files_checked = 0u32;
        let mut blocks_checked = 0u32;
        let mut total_bytes = 0u64;

        for file in &snapshot.files {
            let restored_path = verify_tmp_dir.join(&file.path);
            let bytes = std::fs::read(&restored_path)
                .map_err(|e| format!("Doğrulama dosyası okunamadı ({}): {}", file.path, e))?;

            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let actual_hash = format!("{:x}", hasher.finalize());
            if actual_hash != file.file_hash {
                return Err(format!(
                    "Checksum uyuşmazlığı: {} (beklenen {}, gelen {})",
                    file.path, file.file_hash, actual_hash
                ));
            }

            files_checked += 1;
            blocks_checked += file.block_map.len() as u32;
            total_bytes += bytes.len() as u64;
        }

        let chain_depth = compute_chain_depth(&meta_store, &snapshot).await?;
        let snapshot_digest = compute_snapshot_digest(&snapshot);

        Ok(VerifyBackupResult {
            destination_id: destination.id.clone(),
            source_id: source.id.clone(),
            snapshot_id: resolved_snapshot_id.clone(),
            source_name: snapshot.source_name.clone(),
            backup_level: backup_level.clone(),
            chain_depth,
            files_checked,
            blocks_checked,
            total_bytes,
            snapshot_digest,
            verified_at: Utc::now().to_rfc3339(),
            error_code: None,
        })
    }
    .await;

    let _ = std::fs::remove_dir_all(&verify_tmp_dir);
    state.verifying_jobs.remove(&verify_key);

    let ended_at = Utc::now();
    match verify_result {
        Ok(report) => {
            let checksum = format!(
                "Snapshot doğrulandı: {} dosya, {} blok, zincir derinliği {}",
                report.files_checked, report.blocks_checked, report.chain_depth
            );

            queries::update_log_entry_completed(
                &state.db,
                log_id,
                ended_at,
                "Verified",
                Some(report.total_bytes as i64),
                Some(report.files_checked as i32),
                None,
                Some(checksum.as_str()),
                Some(report.backup_level.as_str()),
                Some(report.snapshot_id.as_str()),
            )
            .await
            .map_err(|e| {
                command_error(
                    RestoreErrorCode::IoFailure,
                    format!("Doğrulama logu güncellenemedi: {}", e),
                )
            })?;

            Ok(report)
        }
        Err(err) => {
            let _ = queries::update_log_entry_completed(
                &state.db,
                log_id,
                ended_at,
                "Failed",
                None,
                None,
                Some(err.as_str()),
                None,
                Some(backup_level.as_str()),
                Some(resolved_snapshot_id.as_str()),
            )
            .await;
            Err(command_error(classify_restore_error(&err), err))
        }
    }
}
