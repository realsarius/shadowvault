use std::sync::Arc;
use serde::{Serialize, Deserialize};
use chrono::Utc;
use sqlx::SqlitePool;
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::AppState;
use crate::db::queries;
use crate::models::{Source, Destination, SourceType};
use crate::models::schedule::{Schedule, RetentionPolicy};

// ── Config format ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct ConfigExport {
    pub version: String,
    pub exported_at: String,
    pub settings: std::collections::HashMap<String, String>,
    pub sources: Vec<SourceExport>,
}

#[derive(Serialize, Deserialize)]
pub struct SourceExport {
    pub name: String,
    pub path: String,
    pub source_type: String,
    pub enabled: bool,
    pub destinations: Vec<DestinationExport>,
}

#[derive(Serialize, Deserialize)]
pub struct DestinationExport {
    pub path: String,
    pub schedule: Schedule,
    pub retention: RetentionPolicy,
    pub exclusions: Vec<String>,
    pub enabled: bool,
    #[serde(default)]
    pub incremental: bool,
}

// ── Settings keys that are safe to export/import ─────────────────────────────

const SAFE_SETTINGS: &[&str] = &[
    "run_on_startup",
    "minimize_to_tray",
    "theme",
    "log_retention_days",
    "language",
];

// ── Commands ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn export_config(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    // Collect sources
    let sources = queries::get_all_sources(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let source_exports: Vec<SourceExport> = sources
        .into_iter()
        .map(|s| SourceExport {
            name: s.name,
            path: s.path,
            source_type: s.source_type.to_string(),
            enabled: s.enabled,
            destinations: s
                .destinations
                .into_iter()
                .map(|d| DestinationExport {
                    path: d.path,
                    schedule: d.schedule,
                    retention: d.retention,
                    exclusions: d.exclusions,
                    enabled: d.enabled,
                    incremental: d.incremental,
                })
                .collect(),
        })
        .collect();

    // Collect safe settings
    let mut settings_map = std::collections::HashMap::new();
    for key in SAFE_SETTINGS {
        if let Ok(Some(val)) = queries::get_setting(&state.db, key).await {
            settings_map.insert(key.to_string(), val);
        }
    }

    let export = ConfigExport {
        version: "1".to_string(),
        exported_at: Utc::now().to_rfc3339(),
        settings: settings_map,
        sources: source_exports,
    };

    let json = serde_json::to_string_pretty(&export).map_err(|e| e.to_string())?;

    // Ask user where to save
    let file_path = app
        .dialog()
        .file()
        .set_file_name("shadowvault-config.json")
        .add_filter("JSON", &["json"])
        .blocking_save_file();

    match file_path {
        Some(path) => {
            let path_str = path
                .as_path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string());
            std::fs::write(&path_str, &json).map_err(|e| e.to_string())?;
            Ok(path_str)
        }
        None => Err("cancelled".to_string()),
    }
}

#[tauri::command]
pub async fn import_config(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<ImportResult, String> {
    // Ask user to pick file
    let file_path = app
        .dialog()
        .file()
        .add_filter("JSON", &["json"])
        .blocking_pick_file();

    let path = match file_path {
        Some(p) => p,
        None => return Err("cancelled".to_string()),
    };

    let path_str = path
        .as_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());

    let json = std::fs::read_to_string(&path_str).map_err(|e| e.to_string())?;
    let export: ConfigExport = serde_json::from_str(&json)
        .map_err(|e| format!("Geçersiz yapılandırma dosyası: {}", e))?;

    if export.version != "1" {
        return Err(format!("Desteklenmeyen yapılandırma sürümü: {}", export.version));
    }

    apply_import(&state.db, export).await.map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct ImportResult {
    pub sources_imported: usize,
    pub destinations_imported: usize,
    pub settings_applied: usize,
}

async fn apply_import(
    db: &Arc<SqlitePool>,
    export: ConfigExport,
) -> anyhow::Result<ImportResult> {
    let mut sources_imported = 0usize;
    let mut destinations_imported = 0usize;

    for src_export in export.sources {
        let source_type = src_export
            .source_type
            .parse::<SourceType>()
            .unwrap_or(SourceType::Directory);

        let source = Source {
            id: Uuid::new_v4().to_string(),
            name: src_export.name,
            path: src_export.path,
            source_type,
            enabled: src_export.enabled,
            created_at: Utc::now(),
            destinations: vec![],
        };

        queries::insert_source(db, &source).await?;
        sources_imported += 1;

        for dest_export in src_export.destinations {
            let dest = Destination {
                id: Uuid::new_v4().to_string(),
                source_id: source.id.clone(),
                path: dest_export.path,
                schedule: dest_export.schedule,
                retention: dest_export.retention,
                exclusions: dest_export.exclusions,
                enabled: dest_export.enabled,
                incremental: dest_export.incremental,
                last_run: None,
                last_status: None,
                next_run: None,
                destination_type: crate::models::DestinationType::Local,
                cloud_config: None,
                sftp_config: None,
                oauth_config: None,
                encrypt: false,
                encrypt_password_enc: None,
                encrypt_salt: None,
            };
            queries::insert_destination(db, &dest).await?;
            destinations_imported += 1;
        }
    }

    // Apply safe settings
    let mut settings_applied = 0usize;
    for (key, value) in &export.settings {
        if SAFE_SETTINGS.contains(&key.as_str()) {
            queries::upsert_setting(db, key, value).await?;
            settings_applied += 1;
        }
    }

    Ok(ImportResult {
        sources_imported,
        destinations_imported,
        settings_applied,
    })
}
