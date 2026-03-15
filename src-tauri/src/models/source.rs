use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::models::schedule::{Schedule, RetentionPolicy};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub path: String,
    pub source_type: SourceType,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub destinations: Vec<Destination>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum SourceType {
    File,
    Directory,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SourceType::File => write!(f, "File"),
            SourceType::Directory => write!(f, "Directory"),
        }
    }
}

impl std::str::FromStr for SourceType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "File" => Ok(SourceType::File),
            "Directory" => Ok(SourceType::Directory),
            _ => Err(anyhow::anyhow!("Invalid source type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Destination {
    pub id: String,
    pub source_id: String,
    pub path: String,
    pub schedule: Schedule,
    pub retention: RetentionPolicy,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub last_status: Option<JobStatus>,
    pub next_run: Option<DateTime<Utc>>,
    /// .gitignore-style exclusion patterns applied during copy
    #[serde(default)]
    pub exclusions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Running,
    Success,
    Failed,
    Skipped,
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            JobStatus::Running => "Running",
            JobStatus::Success => "Success",
            JobStatus::Failed => "Failed",
            JobStatus::Skipped => "Skipped",
            JobStatus::Cancelled => "Cancelled",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for JobStatus {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Running" => Ok(JobStatus::Running),
            "Success" => Ok(JobStatus::Success),
            "Failed" => Ok(JobStatus::Failed),
            "Skipped" => Ok(JobStatus::Skipped),
            "Cancelled" => Ok(JobStatus::Cancelled),
            _ => Err(anyhow::anyhow!("Invalid job status: {}", s)),
        }
    }
}
