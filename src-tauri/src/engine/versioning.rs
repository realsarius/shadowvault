use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use crate::models::VersionNaming;

pub fn compute_version_path(
    destination_base: &str,
    source_name: &str,
    naming: &VersionNaming,
    now: DateTime<Utc>,
) -> PathBuf {
    let base = Path::new(destination_base);
    match naming {
        VersionNaming::Timestamp => {
            let ts = now.format("%Y-%m-%dT%H-%M-%SZ").to_string();
            base.join(format!("{}_{}", source_name, ts))
        }
        VersionNaming::Index => {
            let next_index = find_next_index(destination_base, source_name);
            base.join(format!("{}_{:03}", source_name, next_index))
        }
        VersionNaming::Overwrite => base.join(source_name),
    }
}

fn find_next_index(destination_base: &str, source_name: &str) -> u32 {
    let base = Path::new(destination_base);
    let prefix = format!("{}_", source_name);

    let max_index = match std::fs::read_dir(base) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with(&prefix) {
                    let suffix = &name[prefix.len()..];
                    suffix.parse::<u32>().ok()
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0),
        Err(_) => 0,
    };

    max_index + 1
}

pub async fn apply_retention(
    destination_base: &str,
    source_name: &str,
    max_versions: u32,
) -> anyhow::Result<u32> {
    if max_versions == 0 {
        return Ok(0);
    }

    let base = Path::new(destination_base);
    let prefix = format!("{}_", source_name);

    let mut versioned_dirs: Vec<PathBuf> = match std::fs::read_dir(base) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                // Match both timestamped and indexed versions, but NOT overwrite (exact name)
                name.starts_with(&prefix)
            })
            .map(|e| e.path())
            .collect(),
        Err(_) => return Ok(0),
    };

    // Sort by name (lexicographic); timestamps and zero-padded indexes both sort correctly
    versioned_dirs.sort();

    let count = versioned_dirs.len() as u32;
    if count <= max_versions {
        return Ok(0);
    }

    let to_delete = count - max_versions;
    let mut deleted = 0u32;

    for path in versioned_dirs.iter().take(to_delete as usize) {
        if path.is_dir() {
            if let Err(e) = std::fs::remove_dir_all(path) {
                log::warn!("Failed to delete old version {:?}: {}", path, e);
            } else {
                deleted += 1;
            }
        } else if path.is_file() {
            if let Err(e) = std::fs::remove_file(path) {
                log::warn!("Failed to delete old version {:?}: {}", path, e);
            } else {
                deleted += 1;
            }
        }
    }

    Ok(deleted)
}
