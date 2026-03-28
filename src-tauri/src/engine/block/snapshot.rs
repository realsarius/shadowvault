use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::splitter::BlockDescriptor;

/// Backup level — determines which blocks are stored.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackupLevel {
    /// Full backup — stores ALL blocks for ALL files.
    Level0,
    /// Differential — stores blocks changed since the LAST backup (any level).
    Level1Differential,
    /// Cumulative — stores blocks changed since the LAST Level 0.
    Level1Cumulative,
}

impl std::fmt::Display for BackupLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupLevel::Level0 => write!(f, "Level 0 (Full)"),
            BackupLevel::Level1Differential => write!(f, "Level 1 (Differential)"),
            BackupLevel::Level1Cumulative => write!(f, "Level 1 (Cumulative)"),
        }
    }
}

/// Repository configuration, stored as `config.json` at the repo root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    /// Format version (currently 1).
    pub version: u32,
    /// Chunking algorithm parameters.
    pub chunking: ChunkingConfig,
    /// Encryption parameters (None = unencrypted).
    pub encryption: Option<EncryptionConfig>,
    /// When this repository was first created.
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// Algorithm identifier (e.g. "fastcdc-2020").
    pub algorithm: String,
    pub min_size: u32,
    pub avg_size: u32,
    pub max_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Encryption algorithm (e.g. "AES-256-GCM").
    pub algorithm: String,
    /// Base64-encoded Argon2 salt.
    pub argon2_salt: String,
    pub argon2_m_cost: u32,
    pub argon2_t_cost: u32,
    pub argon2_p_cost: u32,
}

/// A complete backup snapshot manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Unique snapshot ID (UUID v4).
    pub id: String,
    /// When this snapshot was created.
    pub created_at: DateTime<Utc>,
    /// Name of the source being backed up.
    pub source_name: String,
    /// Absolute path of the source.
    pub source_path: String,
    /// Backup level for this snapshot.
    pub level: BackupLevel,
    /// ID of the parent Level 0 snapshot (None for Level 0 itself).
    pub parent_level0_id: Option<String>,
    /// ID of the immediate parent snapshot (for differential chaining).
    pub parent_id: Option<String>,
    /// All files in this snapshot with their block mappings.
    pub files: Vec<FileBlockMap>,
    /// Total original size in bytes (sum of all file sizes).
    pub total_size: u64,
    /// Total number of blocks across all files.
    pub total_blocks: u32,
    /// Number of blocks actually stored in this snapshot (changed blocks).
    pub changed_blocks: u32,
    /// Bytes actually written (changed_blocks * block_size, roughly).
    pub changed_bytes: u64,
}

/// Block mapping for a single file within a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBlockMap {
    /// Relative path from the source root.
    pub path: String,
    /// File size in bytes.
    pub size: u64,
    /// Last modification time.
    pub modified: DateTime<Utc>,
    /// Whole-file SHA-256 hash for integrity verification.
    pub file_hash: String,
    /// Complete block hash map for this file (ALL block hashes, even unchanged ones).
    /// This is always the FULL map so we can compare against it for future backups.
    pub block_map: Vec<BlockDescriptor>,
    /// Which block indices were actually stored in this snapshot.
    /// For Level 0, this equals all indices. For Level 1, only changed ones.
    pub stored_block_indices: Vec<u32>,
}

/// Lightweight snapshot summary for listing (no file/block details).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub source_name: String,
    pub source_path: String,
    pub level: BackupLevel,
    pub parent_level0_id: Option<String>,
    pub total_size: u64,
    pub total_blocks: u32,
    pub changed_blocks: u32,
    pub changed_bytes: u64,
    pub file_count: u32,
}

impl Snapshot {
    /// Creates a summary (without file details) from this snapshot.
    pub fn to_summary(&self) -> SnapshotSummary {
        SnapshotSummary {
            id: self.id.clone(),
            created_at: self.created_at,
            source_name: self.source_name.clone(),
            source_path: self.source_path.clone(),
            level: self.level,
            parent_level0_id: self.parent_level0_id.clone(),
            total_size: self.total_size,
            total_blocks: self.total_blocks,
            changed_blocks: self.changed_blocks,
            changed_bytes: self.changed_bytes,
            file_count: self.files.len() as u32,
        }
    }

    /// Computes the savings ratio (0.0 = no savings, 1.0 = 100% savings).
    pub fn savings_ratio(&self) -> f64 {
        if self.total_size == 0 {
            return 0.0;
        }
        1.0 - (self.changed_bytes as f64 / self.total_size as f64)
    }
}

/// Statistics returned by a prune (garbage collection) operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneStats {
    /// Number of snapshots removed.
    pub snapshots_removed: u32,
    /// Number of block directories cleaned up.
    pub block_dirs_removed: u32,
    /// Bytes freed.
    pub bytes_freed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_savings_ratio_empty() {
        let snap = Snapshot {
            id: "test".into(),
            created_at: Utc::now(),
            source_name: "src".into(),
            source_path: "/tmp".into(),
            level: BackupLevel::Level0,
            parent_level0_id: None,
            parent_id: None,
            files: vec![],
            total_size: 0,
            total_blocks: 0,
            changed_blocks: 0,
            changed_bytes: 0,
        };
        assert_eq!(snap.savings_ratio(), 0.0);
    }

    #[test]
    fn test_savings_ratio_no_savings() {
        let snap = Snapshot {
            id: "test".into(),
            created_at: Utc::now(),
            source_name: "src".into(),
            source_path: "/tmp".into(),
            level: BackupLevel::Level0,
            parent_level0_id: None,
            parent_id: None,
            files: vec![],
            total_size: 1000,
            total_blocks: 10,
            changed_blocks: 10,
            changed_bytes: 1000,
        };
        assert!((snap.savings_ratio() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_savings_ratio_half() {
        let snap = Snapshot {
            id: "test".into(),
            created_at: Utc::now(),
            source_name: "src".into(),
            source_path: "/tmp".into(),
            level: BackupLevel::Level1Cumulative,
            parent_level0_id: Some("base".into()),
            parent_id: Some("base".into()),
            files: vec![],
            total_size: 1000,
            total_blocks: 10,
            changed_blocks: 5,
            changed_bytes: 500,
        };
        assert!((snap.savings_ratio() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_snapshot_summary() {
        let snap = Snapshot {
            id: "snap-1".into(),
            created_at: Utc::now(),
            source_name: "MySource".into(),
            source_path: "/data".into(),
            level: BackupLevel::Level0,
            parent_level0_id: None,
            parent_id: None,
            files: vec![
                FileBlockMap {
                    path: "a.txt".into(),
                    size: 100,
                    modified: Utc::now(),
                    file_hash: "abc".into(),
                    block_map: vec![],
                    stored_block_indices: vec![],
                },
                FileBlockMap {
                    path: "b.txt".into(),
                    size: 200,
                    modified: Utc::now(),
                    file_hash: "def".into(),
                    block_map: vec![],
                    stored_block_indices: vec![],
                },
            ],
            total_size: 300,
            total_blocks: 3,
            changed_blocks: 3,
            changed_bytes: 300,
        };

        let summary = snap.to_summary();
        assert_eq!(summary.id, "snap-1");
        assert_eq!(summary.file_count, 2);
        assert_eq!(summary.total_size, 300);
        assert_eq!(summary.level, BackupLevel::Level0);
    }

    #[test]
    fn test_backup_level_display() {
        assert_eq!(format!("{}", BackupLevel::Level0), "Level 0 (Full)");
        assert_eq!(
            format!("{}", BackupLevel::Level1Differential),
            "Level 1 (Differential)"
        );
        assert_eq!(
            format!("{}", BackupLevel::Level1Cumulative),
            "Level 1 (Cumulative)"
        );
    }
}
