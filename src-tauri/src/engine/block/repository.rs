use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::snapshot::{
    BackupLevel, EncryptionConfig, FileBlockMap, PruneStats, RepoConfig, Snapshot, SnapshotSummary,
};
use super::splitter::{BlockDescriptor, BlockSplitter};
use super::store::BlockStore;

/// Callback for reporting backup/restore progress.
pub trait ProgressReporter: Send + Sync {
    fn on_file_start(&self, path: &str, file_index: u32, total_files: u32);
    fn on_file_done(&self, path: &str, file_index: u32, total_files: u32, bytes: u64);
    fn on_block_stored(&self, block_index: u32, size: u32, is_changed: bool);
}

/// No-op progress reporter for tests and non-interactive use.
pub struct NoopProgress;
impl ProgressReporter for NoopProgress {
    fn on_file_start(&self, _: &str, _: u32, _: u32) {}
    fn on_file_done(&self, _: &str, _: u32, _: u32, _: u64) {}
    fn on_block_stored(&self, _: u32, _: u32, _: bool) {}
}

/// Encryption helpers for block-level encryption.
mod encryption {
    use aes_gcm::{
        aead::{Aead, AeadCore, KeyInit, OsRng},
        Aes256Gcm, Key,
    };

    /// Encrypts block data with AES-256-GCM. Returns nonce || ciphertext.
    pub fn encrypt_block(key: &[u8; 32], plaintext: &[u8]) -> anyhow::Result<Vec<u8>> {
        let cipher_key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(cipher_key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|_| anyhow::anyhow!("Block encryption failed"))?;

        let mut out = Vec::with_capacity(12 + ciphertext.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// Decrypts block data (nonce || ciphertext) with AES-256-GCM.
    pub fn decrypt_block(key: &[u8; 32], data: &[u8]) -> anyhow::Result<Vec<u8>> {
        if data.len() < 12 {
            anyhow::bail!("Encrypted block too short");
        }

        let cipher_key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(cipher_key);
        let nonce = aes_gcm::Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| anyhow::anyhow!("Block decryption failed"))?;

        Ok(plaintext)
    }
}

/// The main block-level backup repository.
pub struct Repository {
    store: Box<dyn BlockStore>,
    config: RepoConfig,
    encryption_key: Option<[u8; 32]>,
}

impl Repository {
    /// Opens an existing repository or initializes a new one.
    pub async fn open_or_init(
        store: Box<dyn BlockStore>,
        encryption_key: Option<[u8; 32]>,
        encryption_config: Option<EncryptionConfig>,
    ) -> anyhow::Result<Self> {
        let config = match store.load_config().await? {
            Some(config) => config,
            None => {
                let config = RepoConfig {
                    version: 1,
                    chunking: super::snapshot::ChunkingConfig {
                        algorithm: "fastcdc-2020".into(),
                        min_size: 65_536,
                        avg_size: 262_144,
                        max_size: 1_048_576,
                    },
                    encryption: encryption_config,
                    created_at: Utc::now(),
                };
                store.init_repo(&config).await?;
                config
            }
        };

        Ok(Self {
            store,
            config,
            encryption_key,
        })
    }

    /// Creates a backup snapshot of the given source path.
    pub async fn backup(
        &mut self,
        source_path: &Path,
        source_name: &str,
        source_type: &crate::models::SourceType,
        exclusions: &globset::GlobSet,
        level: BackupLevel,
        progress: &dyn ProgressReporter,
    ) -> anyhow::Result<Snapshot> {
        let snapshot_id = Uuid::new_v4().to_string();

        // Find parent snapshot based on level
        let (parent_level0_id, parent_id, parent_block_maps) =
            self.resolve_parent(source_name, level).await?;

        let mut files: Vec<FileBlockMap> = Vec::new();
        let mut total_size: u64 = 0;
        let mut total_blocks: u32 = 0;
        let mut changed_blocks: u32 = 0;
        let mut changed_bytes: u64 = 0;

        // Collect files to backup
        let file_list = self.collect_files(source_path, source_type, exclusions)?;
        let total_files = file_list.len() as u32;

        for (file_index, (abs_path, rel_path)) in file_list.iter().enumerate() {
            progress.on_file_start(rel_path, file_index as u32, total_files);

            let file_meta = std::fs::metadata(abs_path)?;
            let file_size = file_meta.len();
            let modified = file_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let modified_dt: DateTime<Utc> = modified.into();

            // Compute whole-file hash
            let file_hash = BlockSplitter::hash_file(abs_path)?;

            // Get parent's block map for this file (if any)
            let parent_blocks = parent_block_maps
                .as_ref()
                .and_then(|maps| maps.get(rel_path));

            // Split file into blocks and determine which are changed
            let mut file_block_map: Vec<BlockDescriptor> = Vec::new();
            let mut stored_indices: Vec<u32> = Vec::new();
            let mut pending_blocks: Vec<(BlockDescriptor, Vec<u8>)> = Vec::new();

            BlockSplitter::split_file(abs_path, |desc, block_data| {
                let is_changed = match level {
                    BackupLevel::Level0 => true, // Full: store all blocks
                    _ => {
                        // Check if block hash differs from parent
                        match parent_blocks {
                            Some(parent_file) => {
                                let parent_desc = parent_file
                                    .iter()
                                    .find(|b| b.index == desc.index);
                                match parent_desc {
                                    Some(pd) => pd.hash != desc.hash,
                                    None => true, // New block (file grew)
                                }
                            }
                            None => true, // New file (not in parent)
                        }
                    }
                };

                if is_changed {
                    let store_data = if let Some(ref key) = self.encryption_key {
                        encryption::encrypt_block(key, block_data)?
                    } else {
                        block_data.to_vec()
                    };
                    pending_blocks.push((desc.clone(), store_data));
                    stored_indices.push(desc.index);
                }

                file_block_map.push(desc.clone());
                total_blocks += 1;
                Ok(())
            })?;

            // Store changed blocks
            for (desc, store_data) in &pending_blocks {
                self.store
                    .put_block(&snapshot_id, &file_hash, desc.index, store_data)
                    .await?;
                changed_blocks += 1;
                changed_bytes += store_data.len() as u64;
                progress.on_block_stored(desc.index, desc.size, true);
            }

            total_size += file_size;

            files.push(FileBlockMap {
                path: rel_path.clone(),
                size: file_size,
                modified: modified_dt,
                file_hash,
                block_map: file_block_map,
                stored_block_indices: stored_indices,
            });

            progress.on_file_done(rel_path, file_index as u32, total_files, file_size);
        }

        let snapshot = Snapshot {
            id: snapshot_id,
            created_at: Utc::now(),
            source_name: source_name.to_string(),
            source_path: source_path.to_string_lossy().to_string(),
            level,
            parent_level0_id,
            parent_id,
            files,
            total_size,
            total_blocks,
            changed_blocks,
            changed_bytes,
        };

        self.store.put_snapshot(&snapshot).await?;

        Ok(snapshot)
    }

    /// Restores a snapshot to the given target path.
    pub async fn restore(
        &self,
        snapshot_id: &str,
        target_path: &Path,
    ) -> anyhow::Result<()> {
        let snapshot = self.store.get_snapshot(snapshot_id).await?;

        // Build restore chain based on level
        let chain = self.build_restore_chain(&snapshot).await?;

        for file_map in &snapshot.files {
            let dest_file = target_path.join(&file_map.path);

            if let Some(parent) = dest_file.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let chunk_count = file_map.block_map.len();

            // Build offset table from block_map (variable-size chunks)
            let mut chunk_offsets = Vec::with_capacity(chunk_count);
            let mut offset: usize = 0;
            for desc in &file_map.block_map {
                chunk_offsets.push(offset);
                offset += desc.size as usize;
            }

            // Allocate buffer for the full file
            let mut file_data = vec![0u8; file_map.size as usize];

            // Track which chunks have been filled
            let mut filled = vec![false; chunk_count];

            // Read chunks from chain (newest first — most recent wins)
            for chain_snapshot in &chain {
                let chain_file = chain_snapshot
                    .files
                    .iter()
                    .find(|f| f.path == file_map.path);

                let chain_file = match chain_file {
                    Some(f) => f,
                    None => continue,
                };

                for &chunk_idx in &chain_file.stored_block_indices {
                    let idx = chunk_idx as usize;
                    if idx >= chunk_count || filled[idx] {
                        continue; // Already filled or out of range
                    }

                    let chunk_data = self
                        .store
                        .get_block(&chain_snapshot.id, &chain_file.file_hash, chunk_idx)
                        .await?;

                    // Decrypt if needed
                    let plaintext = if let Some(ref key) = self.encryption_key {
                        encryption::decrypt_block(key, &chunk_data)?
                    } else {
                        chunk_data
                    };

                    // Write chunk into file buffer at its computed offset
                    let chunk_offset = chunk_offsets[idx];
                    let end = std::cmp::min(chunk_offset + plaintext.len(), file_map.size as usize);
                    let write_len = end - chunk_offset;
                    file_data[chunk_offset..chunk_offset + write_len]
                        .copy_from_slice(&plaintext[..write_len]);

                    filled[idx] = true;
                }
            }

            // Verify all chunks were filled
            if filled.iter().any(|&f| !f) {
                let missing: Vec<usize> = filled
                    .iter()
                    .enumerate()
                    .filter(|(_, &f)| !f)
                    .map(|(i, _)| i)
                    .collect();
                anyhow::bail!(
                    "Missing chunks for {}: {:?}",
                    file_map.path,
                    missing
                );
            }

            // Verify integrity
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&file_data);
            let restored_hash = format!("{:x}", hasher.finalize());

            if restored_hash != file_map.file_hash {
                anyhow::bail!(
                    "Integrity check failed for {}: expected {}, got {}",
                    file_map.path,
                    file_map.file_hash,
                    restored_hash
                );
            }

            std::fs::write(&dest_file, &file_data)?;
        }

        Ok(())
    }

    /// Lists all snapshots.
    pub async fn list_snapshots(&self) -> anyhow::Result<Vec<SnapshotSummary>> {
        self.store.list_snapshots().await
    }

    /// Removes old backup sets beyond `keep_count` and deletes their blocks.
    ///
    /// A "backup set" = one Level 0 + all its dependent Level 1s.
    /// We keep the newest `keep_count` sets.
    pub async fn prune(&mut self, keep_count: u32) -> anyhow::Result<PruneStats> {
        let mut summaries = self.store.list_snapshots().await?;
        summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Group snapshots into backup sets (Level 0 + its Level 1 children)
        let mut sets: Vec<Vec<String>> = Vec::new();
        let mut current_set: Vec<String> = Vec::new();

        // Walk newest-first, start a new set at each Level 0
        for summary in &summaries {
            match summary.level {
                BackupLevel::Level0 => {
                    // Push previous set if non-empty
                    if !current_set.is_empty() {
                        sets.push(current_set);
                    }
                    current_set = vec![summary.id.clone()];
                }
                _ => {
                    current_set.push(summary.id.clone());
                }
            }
        }
        if !current_set.is_empty() {
            sets.push(current_set);
        }

        let mut snapshots_removed: u32 = 0;
        let mut block_dirs_removed: u32 = 0;
        let mut bytes_freed: u64 = 0;

        // Remove sets beyond keep_count
        let to_remove: Vec<Vec<String>> = sets
            .into_iter()
            .skip(keep_count as usize)
            .collect();

        for set in &to_remove {
            for id in set {
                // Calculate size before deleting
                if let Ok(size) = self.store.snapshot_blocks_size(id).await {
                    bytes_freed += size;
                }

                self.store.delete_snapshot_blocks(id).await?;
                self.store.delete_snapshot(id).await?;
                snapshots_removed += 1;
                block_dirs_removed += 1;
            }
        }

        Ok(PruneStats {
            snapshots_removed,
            block_dirs_removed,
            bytes_freed,
        })
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    /// Resolves the parent snapshot for the given level.
    /// Returns (parent_level0_id, parent_id, parent_block_maps).
    async fn resolve_parent(
        &self,
        source_name: &str,
        level: BackupLevel,
    ) -> anyhow::Result<(
        Option<String>,
        Option<String>,
        Option<HashMap<String, Vec<BlockDescriptor>>>,
    )> {
        match level {
            BackupLevel::Level0 => Ok((None, None, None)),
            BackupLevel::Level1Cumulative => {
                // Find the most recent Level 0 for this source
                let snapshots = self.store.list_snapshots().await?;
                let level0 = snapshots
                    .iter()
                    .find(|s| {
                        s.source_name == source_name && s.level == BackupLevel::Level0
                    })
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "No Level 0 backup found for source '{}'. \
                             Cannot create Level 1 Cumulative without a base.",
                            source_name
                        )
                    })?;

                let parent = self.store.get_snapshot(&level0.id).await?;
                let maps = Self::extract_block_maps(&parent);

                Ok((Some(parent.id.clone()), Some(parent.id.clone()), Some(maps)))
            }
            BackupLevel::Level1Differential => {
                // Find the most recent snapshot of any level for this source
                let snapshots = self.store.list_snapshots().await?;
                let latest = snapshots
                    .iter()
                    .find(|s| s.source_name == source_name)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "No previous backup found for source '{}'. \
                             Cannot create Level 1 Differential without a base.",
                            source_name
                        )
                    })?;

                let parent = self.store.get_snapshot(&latest.id).await?;

                // Level 0 reference: either the parent is Level 0, or its parent_level0_id
                let level0_id = match parent.level {
                    BackupLevel::Level0 => parent.id.clone(),
                    _ => parent
                        .parent_level0_id
                        .clone()
                        .unwrap_or_else(|| parent.id.clone()),
                };

                let maps = Self::extract_block_maps(&parent);

                Ok((Some(level0_id), Some(parent.id.clone()), Some(maps)))
            }
        }
    }

    /// Extracts block maps from a snapshot into a HashMap keyed by file path.
    fn extract_block_maps(snapshot: &Snapshot) -> HashMap<String, Vec<BlockDescriptor>> {
        snapshot
            .files
            .iter()
            .map(|f| (f.path.clone(), f.block_map.clone()))
            .collect()
    }

    /// Builds the restore chain for a snapshot (newest first).
    async fn build_restore_chain(&self, snapshot: &Snapshot) -> anyhow::Result<Vec<Snapshot>> {
        match snapshot.level {
            BackupLevel::Level0 => {
                // Just this snapshot — all blocks are present
                Ok(vec![snapshot.clone()])
            }
            BackupLevel::Level1Cumulative => {
                // Need: this snapshot (newest) + Level 0
                let level0_id = snapshot.parent_level0_id.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("Cumulative snapshot missing parent_level0_id")
                })?;
                let level0 = self.store.get_snapshot(level0_id).await?;
                Ok(vec![snapshot.clone(), level0])
            }
            BackupLevel::Level1Differential => {
                // Need: this snapshot + all diffs back to Level 0 (newest first)
                let mut chain = vec![snapshot.clone()];

                let mut current = snapshot.clone();
                loop {
                    let parent_id = match current.parent_id {
                        Some(ref id) => id.clone(),
                        None => break,
                    };

                    let parent = self.store.get_snapshot(&parent_id).await?;
                    chain.push(parent.clone());

                    if parent.level == BackupLevel::Level0 {
                        break;
                    }
                    current = parent;
                }

                Ok(chain)
            }
        }
    }

    /// Collects all files to be backed up as (absolute_path, relative_path) tuples.
    fn collect_files(
        &self,
        source_path: &Path,
        source_type: &crate::models::SourceType,
        exclusions: &globset::GlobSet,
    ) -> anyhow::Result<Vec<(std::path::PathBuf, String)>> {
        let mut files = Vec::new();

        match source_type {
            crate::models::SourceType::File => {
                let file_name = source_path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Cannot determine file name"))?
                    .to_string_lossy()
                    .to_string();
                files.push((source_path.to_path_buf(), file_name));
            }
            crate::models::SourceType::Directory => {
                for entry in walkdir::WalkDir::new(source_path) {
                    let entry = entry?;
                    if !entry.file_type().is_file() {
                        continue;
                    }

                    let rel_path = entry
                        .path()
                        .strip_prefix(source_path)
                        .unwrap_or(entry.path());

                    // Apply exclusions
                    if exclusions.is_match(rel_path) {
                        continue;
                    }

                    let rel_str = rel_path.to_string_lossy().to_string();
                    let rel_str = rel_str.replace('\\', "/");
                    files.push((entry.path().to_path_buf(), rel_str));
                }
            }
        }

        Ok(files)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::store::LocalBlockStore;
    use tempfile::TempDir;

    async fn setup_repo(dir: &TempDir) -> Repository {
        let store = Box::new(LocalBlockStore::new(dir.path().to_str().unwrap()));
        Repository::open_or_init(store, None, None).await.unwrap()
    }

    fn create_test_source(dir: &TempDir) {
        std::fs::write(dir.path().join("file1.txt"), b"Hello, World!").unwrap();
        std::fs::write(dir.path().join("file2.txt"), b"Goodbye, World!").unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("nested.txt"), b"Nested content here").unwrap();
    }

    #[tokio::test]
    async fn test_level0_backup_and_restore() {
        let src_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();
        let restore_dir = TempDir::new().unwrap();

        create_test_source(&src_dir);

        let mut repo = setup_repo(&repo_dir).await;
        let exclusions = globset::GlobSetBuilder::new().build().unwrap();

        let snapshot = repo
            .backup(
                src_dir.path(),
                "test-source",
                &crate::models::SourceType::Directory,
                &exclusions,
                BackupLevel::Level0,
                &NoopProgress,
            )
            .await
            .unwrap();

        assert_eq!(snapshot.files.len(), 3);
        assert_eq!(snapshot.level, BackupLevel::Level0);
        assert!(snapshot.total_size > 0);
        assert_eq!(snapshot.savings_ratio(), 0.0); // Full backup

        // Restore
        repo.restore(&snapshot.id, restore_dir.path()).await.unwrap();

        assert_eq!(
            std::fs::read_to_string(restore_dir.path().join("file1.txt")).unwrap(),
            "Hello, World!"
        );
        assert_eq!(
            std::fs::read_to_string(restore_dir.path().join("file2.txt")).unwrap(),
            "Goodbye, World!"
        );
        assert_eq!(
            std::fs::read_to_string(restore_dir.path().join("subdir/nested.txt")).unwrap(),
            "Nested content here"
        );
    }

    #[tokio::test]
    async fn test_level1_cumulative() {
        let src_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();
        let restore_dir = TempDir::new().unwrap();

        create_test_source(&src_dir);

        let mut repo = setup_repo(&repo_dir).await;
        let exclusions = globset::GlobSetBuilder::new().build().unwrap();

        // Level 0
        let _snap0 = repo
            .backup(
                src_dir.path(),
                "test",
                &crate::models::SourceType::Directory,
                &exclusions,
                BackupLevel::Level0,
                &NoopProgress,
            )
            .await
            .unwrap();

        // Modify one file
        std::fs::write(src_dir.path().join("file1.txt"), b"Modified content!").unwrap();

        // Level 1 Cumulative
        let snap1 = repo
            .backup(
                src_dir.path(),
                "test",
                &crate::models::SourceType::Directory,
                &exclusions,
                BackupLevel::Level1Cumulative,
                &NoopProgress,
            )
            .await
            .unwrap();

        assert_eq!(snap1.level, BackupLevel::Level1Cumulative);
        // Only the modified file's block should be changed
        assert!(snap1.changed_blocks < snap1.total_blocks);

        // Restore from cumulative
        repo.restore(&snap1.id, restore_dir.path()).await.unwrap();

        assert_eq!(
            std::fs::read_to_string(restore_dir.path().join("file1.txt")).unwrap(),
            "Modified content!"
        );
        assert_eq!(
            std::fs::read_to_string(restore_dir.path().join("file2.txt")).unwrap(),
            "Goodbye, World!"
        );
    }

    #[tokio::test]
    async fn test_level1_differential_chain() {
        let src_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();
        let restore_dir = TempDir::new().unwrap();

        create_test_source(&src_dir);

        let mut repo = setup_repo(&repo_dir).await;
        let exclusions = globset::GlobSetBuilder::new().build().unwrap();

        // Level 0
        repo.backup(
            src_dir.path(),
            "test",
            &crate::models::SourceType::Directory,
            &exclusions,
            BackupLevel::Level0,
            &NoopProgress,
        )
        .await
        .unwrap();

        // Day 2: modify file1
        std::fs::write(src_dir.path().join("file1.txt"), b"Day 2 content").unwrap();
        repo.backup(
            src_dir.path(),
            "test",
            &crate::models::SourceType::Directory,
            &exclusions,
            BackupLevel::Level1Differential,
            &NoopProgress,
        )
        .await
        .unwrap();

        // Day 3: modify file2
        std::fs::write(src_dir.path().join("file2.txt"), b"Day 3 content").unwrap();
        let snap_day3 = repo
            .backup(
                src_dir.path(),
                "test",
                &crate::models::SourceType::Directory,
                &exclusions,
                BackupLevel::Level1Differential,
                &NoopProgress,
            )
            .await
            .unwrap();

        // Restore from day 3 differential
        repo.restore(&snap_day3.id, restore_dir.path())
            .await
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(restore_dir.path().join("file1.txt")).unwrap(),
            "Day 2 content"
        );
        assert_eq!(
            std::fs::read_to_string(restore_dir.path().join("file2.txt")).unwrap(),
            "Day 3 content"
        );
        assert_eq!(
            std::fs::read_to_string(restore_dir.path().join("subdir/nested.txt")).unwrap(),
            "Nested content here"
        );
    }

    #[tokio::test]
    async fn test_unchanged_file_no_blocks_stored() {
        let src_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();

        create_test_source(&src_dir);

        let mut repo = setup_repo(&repo_dir).await;
        let exclusions = globset::GlobSetBuilder::new().build().unwrap();

        // Level 0
        repo.backup(
            src_dir.path(),
            "test",
            &crate::models::SourceType::Directory,
            &exclusions,
            BackupLevel::Level0,
            &NoopProgress,
        )
        .await
        .unwrap();

        // Level 1 without any changes
        let snap1 = repo
            .backup(
                src_dir.path(),
                "test",
                &crate::models::SourceType::Directory,
                &exclusions,
                BackupLevel::Level1Cumulative,
                &NoopProgress,
            )
            .await
            .unwrap();

        assert_eq!(snap1.changed_blocks, 0, "No blocks should be stored when nothing changed");
        assert_eq!(snap1.changed_bytes, 0);
        assert!((snap1.savings_ratio() - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_single_file_backup() {
        let src_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();
        let restore_dir = TempDir::new().unwrap();

        let src_file = src_dir.path().join("document.txt");
        std::fs::write(&src_file, b"Important document content").unwrap();

        let mut repo = setup_repo(&repo_dir).await;
        let exclusions = globset::GlobSetBuilder::new().build().unwrap();

        let snapshot = repo
            .backup(
                &src_file,
                "doc",
                &crate::models::SourceType::File,
                &exclusions,
                BackupLevel::Level0,
                &NoopProgress,
            )
            .await
            .unwrap();

        assert_eq!(snapshot.files.len(), 1);

        repo.restore(&snapshot.id, restore_dir.path()).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(restore_dir.path().join("document.txt")).unwrap(),
            "Important document content"
        );
    }

    #[tokio::test]
    async fn test_prune_backup_sets() {
        let src_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();

        std::fs::write(src_dir.path().join("a.txt"), b"aaa").unwrap();

        let mut repo = setup_repo(&repo_dir).await;
        let exclusions = globset::GlobSetBuilder::new().build().unwrap();

        // Create 3 Level 0 snapshots (3 backup sets)
        for _ in 0..3 {
            repo.backup(
                src_dir.path(),
                "test",
                &crate::models::SourceType::Directory,
                &exclusions,
                BackupLevel::Level0,
                &NoopProgress,
            )
            .await
            .unwrap();
        }

        let before = repo.list_snapshots().await.unwrap();
        assert_eq!(before.len(), 3);

        let stats = repo.prune(1).await.unwrap();
        assert_eq!(stats.snapshots_removed, 2);

        let after = repo.list_snapshots().await.unwrap();
        assert_eq!(after.len(), 1);
    }

    #[tokio::test]
    async fn test_level1_without_level0_fails() {
        let src_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();

        std::fs::write(src_dir.path().join("a.txt"), b"data").unwrap();

        let mut repo = setup_repo(&repo_dir).await;
        let exclusions = globset::GlobSetBuilder::new().build().unwrap();

        let result = repo
            .backup(
                src_dir.path(),
                "test",
                &crate::models::SourceType::Directory,
                &exclusions,
                BackupLevel::Level1Cumulative,
                &NoopProgress,
            )
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No Level 0 backup found"));
    }

    #[tokio::test]
    async fn test_encrypted_backup_and_restore() {
        let src_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();
        let restore_dir = TempDir::new().unwrap();

        std::fs::write(src_dir.path().join("secret.txt"), b"Top secret data!").unwrap();

        let key = [0x42u8; 32];
        let store = Box::new(LocalBlockStore::new(repo_dir.path().to_str().unwrap()));
        let enc_config = EncryptionConfig {
            algorithm: "AES-256-GCM".into(),
            argon2_salt: "dGVzdHNhbHQ=".into(),
            argon2_m_cost: 65536,
            argon2_t_cost: 3,
            argon2_p_cost: 4,
        };

        let mut repo = Repository::open_or_init(store, Some(key), Some(enc_config))
            .await
            .unwrap();
        let exclusions = globset::GlobSetBuilder::new().build().unwrap();

        let snapshot = repo
            .backup(
                src_dir.path(),
                "encrypted-test",
                &crate::models::SourceType::Directory,
                &exclusions,
                BackupLevel::Level0,
                &NoopProgress,
            )
            .await
            .unwrap();

        repo.restore(&snapshot.id, restore_dir.path()).await.unwrap();

        assert_eq!(
            std::fs::read_to_string(restore_dir.path().join("secret.txt")).unwrap(),
            "Top secret data!"
        );
    }
}
