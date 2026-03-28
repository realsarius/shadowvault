use globset::{Glob, GlobSetBuilder};
use serde::Serialize;
use tauri::State;

use crate::db::queries;
use crate::models::SourceType;
use crate::AppState;

const MAX_LISTED_FILES: usize = 200;

#[derive(Serialize, specta::Type)]
pub struct PreviewFile {
    pub rel_path: String,
    pub size_bytes: u64,
    /// `false` when incremental mode would skip this file (unchanged since last run)
    pub will_copy: bool,
}

#[derive(Serialize, specta::Type)]
pub struct BackupPreview {
    pub files: Vec<PreviewFile>, // capped at MAX_LISTED_FILES
    pub copy_count: usize,
    pub copy_bytes: u64,
    pub skip_count: usize,
    pub total_count: usize,
    pub source_name: String,
    pub dest_path: String,
    pub incremental: bool,
}

/// Dry-runs the copy logic and returns the list of files that would be copied.
/// Respects exclusion rules and incremental mode — does not write anything to disk.
#[tauri::command]
#[specta::specta]
pub async fn preview_backup(
    destination_id: String,
    state: State<'_, AppState>,
) -> Result<BackupPreview, String> {
    // Fetch destination (works for both enabled and disabled)
    let dest = queries::get_destination_by_id(&state.db, &destination_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Destination {} not found", destination_id))?;

    let source = queries::get_source_by_id(&state.db, &dest.source_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Source {} not found", dest.source_id))?;

    // Build exclusion glob set
    let mut builder = GlobSetBuilder::new();
    for p in &dest.exclusions {
        if let Ok(glob) = Glob::new(p) {
            builder.add(glob);
        }
    }
    let exclusion_set = builder
        .build()
        .unwrap_or_else(|_| GlobSetBuilder::new().build().unwrap());

    // Incremental: only copy files modified after last_run
    let since: Option<std::time::SystemTime> = if dest.incremental {
        dest.last_run
            .map(|dt| std::time::UNIX_EPOCH + std::time::Duration::from_secs(dt.timestamp() as u64))
    } else {
        None
    };

    let mut files: Vec<PreviewFile> = Vec::new();
    let mut copy_count: usize = 0;
    let mut copy_bytes: u64 = 0;
    let mut skip_count: usize = 0;

    match source.source_type {
        SourceType::File => {
            let size = std::fs::metadata(&source.path)
                .map(|m| m.len())
                .unwrap_or(0);
            let file_name = std::path::Path::new(&source.path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            copy_count = 1;
            copy_bytes = size;
            files.push(PreviewFile {
                rel_path: file_name,
                size_bytes: size,
                will_copy: true,
            });
        }
        SourceType::Directory => {
            let source_path = std::path::Path::new(&source.path);
            for entry in walkdir::WalkDir::new(source_path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }

                let rel_path = entry
                    .path()
                    .strip_prefix(source_path)
                    .unwrap_or(entry.path());
                if rel_path == std::path::Path::new("") {
                    continue;
                }
                if exclusion_set.is_match(rel_path) {
                    continue;
                }

                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

                let will_copy = match since {
                    Some(since_time) => entry
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|modified| modified > since_time)
                        .unwrap_or(true),
                    None => true,
                };

                if will_copy {
                    copy_count += 1;
                    copy_bytes += size;
                } else {
                    skip_count += 1;
                }

                if files.len() < MAX_LISTED_FILES {
                    files.push(PreviewFile {
                        rel_path: rel_path.to_string_lossy().to_string(),
                        size_bytes: size,
                        will_copy,
                    });
                }
            }
        }
    }

    Ok(BackupPreview {
        files,
        copy_count,
        copy_bytes,
        skip_count,
        total_count: copy_count + skip_count,
        source_name: source.name,
        dest_path: dest.path,
        incremental: dest.incremental,
    })
}
