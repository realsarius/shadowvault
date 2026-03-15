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

/// Returns the total bytes and file count for a directory (or 0,0 if it doesn't exist yet).
/// Used in tests without the private `count_dir_stats` from copier.rs.
#[cfg(test)]
fn dir_entry_count(base: &std::path::Path, prefix: &str) -> usize {
    std::fs::read_dir(base)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with(prefix))
                .count()
        })
        .unwrap_or(0)
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap()
    }

    #[test]
    fn test_timestamp_naming() {
        let path = compute_version_path("/backups", "mydata", &VersionNaming::Timestamp, fixed_now());
        assert_eq!(
            path,
            std::path::PathBuf::from("/backups/mydata_2024-01-15T14-30-00Z")
        );
    }

    #[test]
    fn test_overwrite_naming() {
        let path = compute_version_path("/backups", "mydata", &VersionNaming::Overwrite, fixed_now());
        assert_eq!(path, std::path::PathBuf::from("/backups/mydata"));
    }

    #[test]
    fn test_index_naming_starts_at_001_when_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path().to_str().unwrap();
        let path = compute_version_path(base, "mydata", &VersionNaming::Index, fixed_now());
        assert!(
            path.to_string_lossy().ends_with("mydata_001"),
            "Expected mydata_001, got {:?}",
            path
        );
    }

    #[test]
    fn test_index_naming_increments_existing() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("mydata_003")).unwrap();
        let base = tmp.path().to_str().unwrap();
        let path = compute_version_path(base, "mydata", &VersionNaming::Index, fixed_now());
        assert!(
            path.to_string_lossy().ends_with("mydata_004"),
            "Expected mydata_004, got {:?}",
            path
        );
    }

    #[tokio::test]
    async fn test_retention_removes_oldest() {
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path().to_str().unwrap();

        for i in 1..=5 {
            std::fs::create_dir(tmp.path().join(format!("mydata_{:03}", i))).unwrap();
        }

        let deleted = apply_retention(base, "mydata", 3).await.unwrap();
        assert_eq!(deleted, 2);
        assert!(!tmp.path().join("mydata_001").exists());
        assert!(!tmp.path().join("mydata_002").exists());
        assert!(tmp.path().join("mydata_003").exists());
        assert!(tmp.path().join("mydata_004").exists());
        assert!(tmp.path().join("mydata_005").exists());
    }

    #[tokio::test]
    async fn test_retention_no_op_under_limit() {
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path().to_str().unwrap();

        for i in 1..=3 {
            std::fs::create_dir(tmp.path().join(format!("mydata_{:03}", i))).unwrap();
        }

        let deleted = apply_retention(base, "mydata", 5).await.unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(dir_entry_count(tmp.path(), "mydata_"), 3);
    }

    #[tokio::test]
    async fn test_retention_zero_max_is_no_op() {
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path().to_str().unwrap();

        for i in 1..=10 {
            std::fs::create_dir(tmp.path().join(format!("mydata_{:03}", i))).unwrap();
        }

        let deleted = apply_retention(base, "mydata", 0).await.unwrap();
        assert_eq!(deleted, 0);
    }
}
