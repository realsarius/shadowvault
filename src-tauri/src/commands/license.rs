use tauri::State;
use uuid::Uuid;
use sysinfo::System;

use crate::AppState;
use crate::db::queries;

/// Makineye özgü deterministik bir kimlik üretir.
/// sysinfo ile hostname + toplam bellek alınır, UUID v5 (SHA-1 tabanlı) ile hashlenir.
#[tauri::command]
pub async fn get_hardware_id() -> Result<String, String> {
    let mut sys = System::new();
    sys.refresh_memory();

    let hostname = System::host_name().unwrap_or_else(|| "unknown-host".to_string());
    let total_memory = sys.total_memory(); // bayt cinsinden
    let cpu_count = sys.cpus().len();

    let raw = format!("shadowvault:{}:{}:{}", hostname, total_memory, cpu_count);

    // UUID v5 — deterministik, SHA-1 bazlı
    let hw_uuid = Uuid::new_v5(&Uuid::NAMESPACE_DNS, raw.as_bytes());
    Ok(format!("HW-{}", hw_uuid.to_string().to_uppercase()))
}

/// Aktif lisans anahtarını SQLite ayarlar tablosuna kaydeder.
#[tauri::command]
pub async fn store_license(state: State<'_, AppState>, key: String) -> Result<(), String> {
    queries::upsert_setting(&state.db, "license_key", &key)
        .await
        .map_err(|e| e.to_string())
}

/// Kayıtlı lisans anahtarını döner (yoksa null).
#[tauri::command]
pub async fn get_stored_license(state: State<'_, AppState>) -> Result<Option<String>, String> {
    queries::get_setting(&state.db, "license_key")
        .await
        .map_err(|e| e.to_string())
}

/// Lisansı geçersiz kılar (aktivasyon ekranına döndürmek için).
#[tauri::command]
pub async fn clear_license(state: State<'_, AppState>) -> Result<(), String> {
    queries::upsert_setting(&state.db, "license_key", "")
        .await
        .map_err(|e| e.to_string())
}
