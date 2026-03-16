use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "value")]
pub enum Schedule {
    Interval { minutes: u32 },
    Cron { expression: String },
    OnChange,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct RetentionPolicy {
    pub max_versions: u32,
    pub naming: VersionNaming,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        RetentionPolicy {
            max_versions: 5,
            naming: VersionNaming::Timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub enum VersionNaming {
    Timestamp,
    Index,
    Overwrite,
}
