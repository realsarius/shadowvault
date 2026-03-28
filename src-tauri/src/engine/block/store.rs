use std::path::{Path, PathBuf};

use async_trait::async_trait;

use super::snapshot::{RepoConfig, Snapshot, SnapshotSummary};

/// Abstraction over block storage.
///
/// Unlike CDC's content-addressed ChunkStore, blocks are stored per-snapshot
/// and identified by (snapshot_id, file_hash, block_index).
#[async_trait]
pub trait BlockStore: Send + Sync {
    /// Store a block for a specific snapshot/file/index.
    async fn put_block(
        &self,
        snapshot_id: &str,
        file_hash: &str,
        block_index: u32,
        data: &[u8],
    ) -> anyhow::Result<()>;

    /// Retrieve a block.
    async fn get_block(
        &self,
        snapshot_id: &str,
        file_hash: &str,
        block_index: u32,
    ) -> anyhow::Result<Vec<u8>>;

    /// Check if a block exists.
    async fn has_block(
        &self,
        snapshot_id: &str,
        file_hash: &str,
        block_index: u32,
    ) -> anyhow::Result<bool>;

    /// Delete all blocks for a snapshot.
    async fn delete_snapshot_blocks(&self, snapshot_id: &str) -> anyhow::Result<()>;

    /// Store a snapshot manifest.
    async fn put_snapshot(&self, snapshot: &Snapshot) -> anyhow::Result<()>;

    /// List all available snapshots (lightweight summaries).
    async fn list_snapshots(&self) -> anyhow::Result<Vec<SnapshotSummary>>;

    /// Retrieve a full snapshot by ID.
    async fn get_snapshot(&self, id: &str) -> anyhow::Result<Snapshot>;

    /// Delete a snapshot manifest by ID.
    async fn delete_snapshot(&self, id: &str) -> anyhow::Result<()>;

    /// Initialize the repository directory structure.
    async fn init_repo(&self, config: &RepoConfig) -> anyhow::Result<()>;

    /// Load the repository config. Returns None if not yet initialized.
    async fn load_config(&self) -> anyhow::Result<Option<RepoConfig>>;

    /// Calculate total size of blocks for a snapshot.
    async fn snapshot_blocks_size(&self, snapshot_id: &str) -> anyhow::Result<u64>;
}

// ── LocalBlockStore ──────────────────────────────────────────────────────────

/// Stores blocks as files on the local filesystem.
///
/// Layout:
/// ```text
/// <destination>/
///   .shadowvault/
///     config.json
///     blocks/
///       <snapshot_id>/
///         <file_hash>/
///           block_0000
///           block_0001
///           ...
///     snapshots/
///       <uuid>.json
/// ```
pub struct LocalBlockStore {
    /// Root path of the block store (the `.shadowvault` directory).
    root: PathBuf,
}

impl LocalBlockStore {
    pub fn new(destination_path: &str) -> Self {
        Self {
            root: Path::new(destination_path).join(".shadowvault"),
        }
    }

    fn blocks_dir(&self) -> PathBuf {
        self.root.join("blocks")
    }

    fn snapshots_dir(&self) -> PathBuf {
        self.root.join("snapshots")
    }

    fn block_path(&self, snapshot_id: &str, file_hash: &str, block_index: u32) -> PathBuf {
        self.blocks_dir()
            .join(snapshot_id)
            .join(file_hash)
            .join(format!("block_{:04}", block_index))
    }

    fn snapshot_blocks_dir(&self, snapshot_id: &str) -> PathBuf {
        self.blocks_dir().join(snapshot_id)
    }

    fn snapshot_path(&self, id: &str) -> PathBuf {
        self.snapshots_dir().join(format!("{}.json", id))
    }

    fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }
}

#[async_trait]
impl BlockStore for LocalBlockStore {
    async fn put_block(
        &self,
        snapshot_id: &str,
        file_hash: &str,
        block_index: u32,
        data: &[u8],
    ) -> anyhow::Result<()> {
        let path = self.block_path(snapshot_id, file_hash, block_index);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write to temp file + rename for atomicity
        let tmp_path = path.with_extension("tmp");
        std::fs::write(&tmp_path, data)?;
        std::fs::rename(&tmp_path, &path)?;

        Ok(())
    }

    async fn get_block(
        &self,
        snapshot_id: &str,
        file_hash: &str,
        block_index: u32,
    ) -> anyhow::Result<Vec<u8>> {
        let path = self.block_path(snapshot_id, file_hash, block_index);
        std::fs::read(&path).map_err(|e| {
            anyhow::anyhow!(
                "Block not found [{}/{}/#{}]: {}",
                snapshot_id,
                file_hash,
                block_index,
                e
            )
        })
    }

    async fn has_block(
        &self,
        snapshot_id: &str,
        file_hash: &str,
        block_index: u32,
    ) -> anyhow::Result<bool> {
        Ok(self
            .block_path(snapshot_id, file_hash, block_index)
            .exists())
    }

    async fn delete_snapshot_blocks(&self, snapshot_id: &str) -> anyhow::Result<()> {
        let dir = self.snapshot_blocks_dir(snapshot_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    async fn put_snapshot(&self, snapshot: &Snapshot) -> anyhow::Result<()> {
        let path = self.snapshot_path(&snapshot.id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(snapshot)?;
        let tmp_path = path.with_extension("tmp");
        std::fs::write(&tmp_path, json.as_bytes())?;
        std::fs::rename(&tmp_path, &path)?;

        Ok(())
    }

    async fn list_snapshots(&self) -> anyhow::Result<Vec<SnapshotSummary>> {
        let dir = self.snapshots_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut summaries = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let data = std::fs::read_to_string(&path)?;
            let snapshot: Snapshot = serde_json::from_str(&data)?;
            summaries.push(snapshot.to_summary());
        }

        summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(summaries)
    }

    async fn get_snapshot(&self, id: &str) -> anyhow::Result<Snapshot> {
        let path = self.snapshot_path(id);
        let data = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Snapshot not found {}: {}", id, e))?;
        let snapshot: Snapshot = serde_json::from_str(&data)?;
        Ok(snapshot)
    }

    async fn delete_snapshot(&self, id: &str) -> anyhow::Result<()> {
        let path = self.snapshot_path(id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    async fn init_repo(&self, config: &RepoConfig) -> anyhow::Result<()> {
        std::fs::create_dir_all(self.blocks_dir())?;
        std::fs::create_dir_all(self.snapshots_dir())?;

        let config_json = serde_json::to_string_pretty(config)?;
        std::fs::write(self.config_path(), config_json.as_bytes())?;

        Ok(())
    }

    async fn load_config(&self) -> anyhow::Result<Option<RepoConfig>> {
        let path = self.config_path();
        if !path.exists() {
            return Ok(None);
        }

        let data = std::fs::read_to_string(&path)?;
        let config: RepoConfig = serde_json::from_str(&data)?;
        Ok(Some(config))
    }

    async fn snapshot_blocks_size(&self, snapshot_id: &str) -> anyhow::Result<u64> {
        let dir = self.snapshot_blocks_dir(snapshot_id);
        if !dir.exists() {
            return Ok(0);
        }

        let mut total: u64 = 0;
        for file_entry in std::fs::read_dir(&dir)? {
            let file_entry = file_entry?;
            if !file_entry.file_type()?.is_dir() {
                continue;
            }
            for block_entry in std::fs::read_dir(file_entry.path())? {
                let block_entry = block_entry?;
                total += block_entry.metadata()?.len();
            }
        }

        Ok(total)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    use super::super::snapshot::{BackupLevel, FileBlockMap};
    use super::super::splitter::BlockDescriptor;

    fn make_config() -> RepoConfig {
        RepoConfig {
            version: 1,
            chunking: super::super::snapshot::ChunkingConfig {
                algorithm: "fastcdc-2020".into(),
                min_size: 65536,
                avg_size: 262144,
                max_size: 1048576,
            },
            encryption: None,
            created_at: Utc::now(),
        }
    }

    fn make_snapshot(id: &str) -> Snapshot {
        Snapshot {
            id: id.into(),
            created_at: Utc::now(),
            source_name: "test".into(),
            source_path: "/tmp/src".into(),
            level: BackupLevel::Level0,
            parent_level0_id: None,
            parent_id: None,
            files: vec![FileBlockMap {
                path: "file.txt".into(),
                size: 100,
                modified: Utc::now(),
                file_hash: "aabbccdd".into(),
                block_map: vec![BlockDescriptor {
                    index: 0,
                    hash: "aabbccdd".into(),
                    size: 100,
                }],
                stored_block_indices: vec![0],
            }],
            total_size: 100,
            total_blocks: 1,
            changed_blocks: 1,
            changed_bytes: 100,
        }
    }

    #[tokio::test]
    async fn test_init_repo_creates_structure() {
        let dir = TempDir::new().unwrap();
        let store = LocalBlockStore::new(dir.path().to_str().unwrap());
        let config = make_config();

        store.init_repo(&config).await.unwrap();

        assert!(store.blocks_dir().exists());
        assert!(store.snapshots_dir().exists());
        assert!(store.config_path().exists());
    }

    #[tokio::test]
    async fn test_load_config() {
        let dir = TempDir::new().unwrap();
        let store = LocalBlockStore::new(dir.path().to_str().unwrap());

        assert!(store.load_config().await.unwrap().is_none());

        let config = make_config();
        store.init_repo(&config).await.unwrap();

        let loaded = store.load_config().await.unwrap().unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.chunking.algorithm, "fastcdc-2020");
    }

    #[tokio::test]
    async fn test_put_get_block() {
        let dir = TempDir::new().unwrap();
        let store = LocalBlockStore::new(dir.path().to_str().unwrap());
        store.init_repo(&make_config()).await.unwrap();

        let data = b"hello block data";
        store
            .put_block("snap-1", "filehash123", 0, data)
            .await
            .unwrap();

        assert!(store.has_block("snap-1", "filehash123", 0).await.unwrap());

        let retrieved = store.get_block("snap-1", "filehash123", 0).await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn test_delete_snapshot_blocks() {
        let dir = TempDir::new().unwrap();
        let store = LocalBlockStore::new(dir.path().to_str().unwrap());
        store.init_repo(&make_config()).await.unwrap();

        store
            .put_block("snap-1", "file1", 0, b"data0")
            .await
            .unwrap();
        store
            .put_block("snap-1", "file1", 1, b"data1")
            .await
            .unwrap();
        store
            .put_block("snap-1", "file2", 0, b"data2")
            .await
            .unwrap();

        assert!(store.has_block("snap-1", "file1", 0).await.unwrap());

        store.delete_snapshot_blocks("snap-1").await.unwrap();

        assert!(!store.has_block("snap-1", "file1", 0).await.unwrap());
        assert!(!store.has_block("snap-1", "file2", 0).await.unwrap());
    }

    #[tokio::test]
    async fn test_snapshot_crud() {
        let dir = TempDir::new().unwrap();
        let store = LocalBlockStore::new(dir.path().to_str().unwrap());
        store.init_repo(&make_config()).await.unwrap();

        // Create
        let snap = make_snapshot("snap-001");
        store.put_snapshot(&snap).await.unwrap();

        // List
        let summaries = store.list_snapshots().await.unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, "snap-001");
        assert_eq!(summaries[0].file_count, 1);
        assert_eq!(summaries[0].level, BackupLevel::Level0);

        // Get
        let loaded = store.get_snapshot("snap-001").await.unwrap();
        assert_eq!(loaded.id, "snap-001");
        assert_eq!(loaded.files.len(), 1);

        // Delete
        store.delete_snapshot("snap-001").await.unwrap();
        let summaries = store.list_snapshots().await.unwrap();
        assert!(summaries.is_empty());
    }

    #[tokio::test]
    async fn test_snapshot_blocks_size() {
        let dir = TempDir::new().unwrap();
        let store = LocalBlockStore::new(dir.path().to_str().unwrap());
        store.init_repo(&make_config()).await.unwrap();

        store
            .put_block("snap-1", "file1", 0, &[0u8; 4096])
            .await
            .unwrap();
        store
            .put_block("snap-1", "file1", 1, &[0u8; 4096])
            .await
            .unwrap();

        let size = store.snapshot_blocks_size("snap-1").await.unwrap();
        assert_eq!(size, 8192);
    }
}
