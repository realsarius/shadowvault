use tauri::State;
use uuid::Uuid;
use sysinfo::System;
use serde::Serialize;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::AppState;
use crate::db::queries;

const LICENSE_API: &str = "https://license.berkansozer.com";

#[derive(Serialize)]
struct LicenseApiRequest {
    key: String,
    hardware_id: String,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn hardware_id_raw() -> String {
    let mut sys = System::new();
    sys.refresh_memory();
    let hostname = System::host_name().unwrap_or_else(|| "unknown-host".to_string());
    let total_memory = sys.total_memory();
    let cpu_count = sys.cpus().len();
    format!("shadowvault:{}:{}:{}", hostname, total_memory, cpu_count)
}

fn hardware_id_formatted(raw: &str) -> String {
    let uuid = Uuid::new_v5(&Uuid::NAMESPACE_DNS, raw.as_bytes());
    format!("HW-{}", uuid.to_string().to_uppercase())
}

fn derive_aes_key(hw_raw: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(hw_raw.as_bytes());
    hasher.finalize().into()
}

fn encrypt_value(plaintext: &str, hw_raw: &str) -> Result<String, String> {
    let key_bytes = derive_aes_key(hw_raw);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut combined = nonce.to_vec();
    combined.extend(ciphertext);
    Ok(BASE64.encode(combined))
}

fn decrypt_value(encoded: &str, hw_raw: &str) -> Result<String, String> {
    let combined = BASE64.decode(encoded).map_err(|e| e.to_string())?;
    if combined.len() < 13 {
        return Err("Ciphertext too short".to_string());
    }
    let key_bytes = derive_aes_key(hw_raw);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&combined[..12]);
    let plaintext_bytes = cipher
        .decrypt(nonce, &combined[12..])
        .map_err(|_| "Decryption failed".to_string())?;
    String::from_utf8(plaintext_bytes).map_err(|e| e.to_string())
}

/// Plaintext key stored by an older version → try to decrypt, fall back to raw
fn resolve_stored_key(stored: &str, hw_raw: &str) -> Option<String> {
    match decrypt_value(stored, hw_raw) {
        Ok(k) => Some(k),
        Err(_) => {
            // Legacy plaintext (pre-encryption)
            if stored.starts_with("SV-") {
                Some(stored.to_string())
            } else {
                None
            }
        }
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Returns the machine's unique hardware ID (for display).
#[tauri::command]
pub async fn get_hardware_id() -> Result<String, String> {
    let raw = hardware_id_raw();
    Ok(hardware_id_formatted(&raw))
}

/// Activates a license key via the backend API.
/// On success the key is encrypted with AES-256-GCM and stored in SQLite.
#[tauri::command]
pub async fn activate_license(
    state: State<'_, AppState>,
    key: String,
) -> Result<serde_json::Value, String> {
    let hw_raw = hardware_id_raw();
    let hw_id = hardware_id_formatted(&hw_raw);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .post(format!("{}/licenses/activate", LICENSE_API))
        .json(&LicenseApiRequest { key: key.clone(), hardware_id: hw_id })
        .send()
        .await
        .map_err(|e| format!("Sunucuya bağlanılamadı: {}", e))?;

    let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    let is_activated = data.get("valid").and_then(|v| v.as_bool()).unwrap_or(false)
        || data.get("activated_at").and_then(|v| v.as_str()).is_some();

    if is_activated {
        let encrypted = encrypt_value(&key, &hw_raw)?;
        queries::upsert_setting(&state.db, "license_key", &encrypted)
            .await
            .map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "success": true }))
    } else {
        let msg = data
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Geçersiz lisans anahtarı.")
            .to_string();
        Ok(serde_json::json!({ "success": false, "error": msg }))
    }
}

/// Validates the stored license key against the backend API.
/// Falls back to "valid" (offline grace) when the network is unreachable.
#[tauri::command]
pub async fn validate_license(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let hw_raw = hardware_id_raw();
    let hw_id = hardware_id_formatted(&hw_raw);

    let stored = match queries::get_setting(&state.db, "license_key")
        .await
        .map_err(|e| e.to_string())?
    {
        Some(k) if !k.is_empty() => k,
        _ => return Ok(serde_json::json!({ "status": "invalid" })),
    };

    let key = match resolve_stored_key(&stored, &hw_raw) {
        Some(k) => k,
        None => return Ok(serde_json::json!({ "status": "invalid" })),
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    match client
        .post(format!("{}/licenses/validate", LICENSE_API))
        .json(&LicenseApiRequest { key, hardware_id: hw_id })
        .send()
        .await
    {
        Ok(response) => {
            let data: serde_json::Value = response.json().await.unwrap_or_default();
            let valid = data.get("valid").and_then(|v| v.as_bool()).unwrap_or(false);
            Ok(serde_json::json!({ "status": if valid { "valid" } else { "invalid" } }))
        }
        Err(_) => {
            // Offline: grant access if a key is stored
            Ok(serde_json::json!({ "status": "valid", "offline": true }))
        }
    }
}

/// Returns the decrypted stored key (for display / compatibility).
#[tauri::command]
pub async fn get_stored_license(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let hw_raw = hardware_id_raw();
    let stored = queries::get_setting(&state.db, "license_key")
        .await
        .map_err(|e| e.to_string())?;

    Ok(stored.and_then(|k| {
        if k.is_empty() {
            None
        } else {
            resolve_stored_key(&k, &hw_raw)
        }
    }))
}

/// Stores a license key encrypted. Kept for backward compatibility.
#[tauri::command]
pub async fn store_license(state: State<'_, AppState>, key: String) -> Result<(), String> {
    let hw_raw = hardware_id_raw();
    let encrypted = encrypt_value(&key, &hw_raw)?;
    queries::upsert_setting(&state.db, "license_key", &encrypted)
        .await
        .map_err(|e| e.to_string())
}

/// Core activation logic shared between the Tauri command and the deep link handler.
/// Returns Ok(true) if activated, Ok(false) if invalid key.
pub async fn activate_license_with_key(db: &std::sync::Arc<sqlx::SqlitePool>, key: &str) -> anyhow::Result<bool> {
    let hw_raw = hardware_id_raw();
    let hw_id = hardware_id_formatted(&hw_raw);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client
        .post(format!("{}/licenses/activate", LICENSE_API))
        .json(&LicenseApiRequest { key: key.to_string(), hardware_id: hw_id })
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Sunucuya bağlanılamadı: {}", e))?;

    let data: serde_json::Value = response.json().await?;

    let is_activated = data.get("valid").and_then(|v| v.as_bool()).unwrap_or(false)
        || data.get("activated_at").and_then(|v| v.as_str()).is_some();

    if is_activated {
        let encrypted = encrypt_value(key, &hw_raw).map_err(|e| anyhow::anyhow!(e))?;
        queries::upsert_setting(db, "license_key", &encrypted).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Clears the stored license key (local only, no server call).
#[tauri::command]
pub async fn clear_license(state: State<'_, AppState>) -> Result<(), String> {
    queries::upsert_setting(&state.db, "license_key", "")
        .await
        .map_err(|e| e.to_string())
}

/// Deactivates the license on the server and clears it locally.
/// Allows the user to activate on a different device afterwards.
#[tauri::command]
pub async fn deactivate_license(state: State<'_, AppState>) -> Result<(), String> {
    let hw_raw = hardware_id_raw();
    let hw_id = hardware_id_formatted(&hw_raw);

    let stored = queries::get_setting(&state.db, "license_key")
        .await
        .map_err(|e| e.to_string())?;

    if let Some(s) = stored.filter(|k| !k.is_empty()) {
        if let Some(key) = resolve_stored_key(&s, &hw_raw) {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| e.to_string())?;

            let resp = client
                .post(format!("{}/licenses/deactivate", LICENSE_API))
                .json(&LicenseApiRequest { key, hardware_id: hw_id })
                .send()
                .await;

            // If server is unreachable we still clear locally so the user isn't stuck.
            if let Ok(r) = resp {
                let data: serde_json::Value = r.json().await.unwrap_or_default();
                let ok = data.get("deactivated").and_then(|v| v.as_bool()).unwrap_or(true);
                if !ok {
                    let msg = data.get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Deaktivasyon başarısız.")
                        .to_string();
                    return Err(msg);
                }
            }
        }
    }

    queries::upsert_setting(&state.db, "license_key", "")
        .await
        .map_err(|e| e.to_string())
}
