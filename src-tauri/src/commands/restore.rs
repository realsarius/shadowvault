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
    "C:\\Windows", "C:\\Program Files", "C:\\System Volume Information",
];

#[tauri::command]
#[specta::specta]
pub async fn restore_backup(
    backup_path: String,
    restore_to: String,
) -> Result<(), String> {
    let src = std::path::Path::new(&backup_path);
    let dst = std::path::Path::new(&restore_to);

    if !src.exists() {
        return Err(format!("Yedek bulunamadı: {}", backup_path));
    }

    for prefix in BLOCKED_RESTORE_PREFIXES {
        if restore_to.starts_with(prefix) {
            return Err(format!(
                "Güvenlik ihlali: '{}' sistem dizinine geri yükleme yapılamaz.",
                restore_to
            ));
        }
    }

    // Create destination parent directories
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    if src.is_file() {
        std::fs::copy(src, dst)
            .map(|_| ())
            .map_err(|e| e.to_string())?;
    } else {
        // Directory: copy contents into restore_to
        std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
        let options = fs_extra::dir::CopyOptions {
            overwrite: true,
            skip_exist: false,
            copy_inside: true,
            content_only: true,
            ..Default::default()
        };
        fs_extra::dir::copy(src, dst, &options)
            .map(|_| ())
            .map_err(|e| e.to_string())?;
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
            return Err(format!(
                "Güvenlik ihlali: '{}' sistem dizinine geri yükleme yapılamaz.",
                restore_to
            ));
        }
    }

    let store = Box::new(
        crate::engine::block::store::LocalBlockStore::new(&destination_path),
    );

    // Derive encryption key from password if provided
    let encryption_key = if let Some(ref pwd) = password {
        // Load repo config to get the salt
        let config = store
            .load_config()
            .await
            .map_err(|e| format!("Repo config yüklenemedi: {}", e))?
            .ok_or_else(|| "Repo config bulunamadı".to_string())?;

        if let Some(ref enc) = config.encryption {
            let key = crate::engine::copier::derive_backup_key_from_password(pwd, &enc.argon2_salt)
                .map_err(|e| format!("Şifre çözme anahtarı türetilemedi: {}", e))?;
            Some(key)
        } else {
            None
        }
    } else {
        None
    };

    use crate::engine::block::store::BlockStore;
    let repo = crate::engine::block::repository::Repository::open_or_init(
        store,
        encryption_key,
        None,
    )
    .await
    .map_err(|e| format!("Repo açılamadı: {}", e))?;

    let target_path = std::path::Path::new(&restore_to);
    std::fs::create_dir_all(target_path).map_err(|e| e.to_string())?;

    repo.restore(&snapshot_id, target_path)
        .await
        .map_err(|e| format!("Geri yükleme başarısız: {}", e))?;

    log::info!(
        "Block restore completed: snapshot {} → {}",
        snapshot_id,
        restore_to
    );
    Ok(())
}
