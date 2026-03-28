use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct LogEntry {
    pub id: i64,
    pub source_id: String,
    pub destination_id: String,
    pub source_path: String,
    pub destination_path: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub status: String,
    pub bytes_copied: Option<i64>,
    pub files_copied: Option<i32>,
    pub error_message: Option<String>,
    pub trigger: String,
    /// SHA-256 hash (file) or "N files verified" (directory) after integrity check
    pub checksum: Option<String>,
    /// Block backup level when available (Level0, Level1Cumulative, Level1Differential)
    pub backup_level: Option<String>,
    /// Block snapshot UUID when available
    pub snapshot_id: Option<String>,
}
