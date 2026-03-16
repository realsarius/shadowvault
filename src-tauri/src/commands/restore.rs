/// Restores a versioned backup to its original (or specified) location.
///
/// `backup_path`  — path to the versioned backup directory/file (from copy_logs.destination_path)
/// `restore_to`   — target path to restore into (typically the original source_path)
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
