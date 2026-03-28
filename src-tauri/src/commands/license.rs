use serde::Serialize;
use tauri::State;
use uuid::Uuid;

use crate::crypto_utils::{hw_decrypt_string, hw_encrypt, hw_id_raw};
use crate::db::queries;
use crate::AppState;

const LICENSE_API: &str = "https://license.berkansozer.com";

#[derive(Debug, Serialize, specta::Type)]
pub struct ActivateResult {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, specta::Type)]
pub struct ValidateResult {
    pub status: String,
    pub offline: Option<bool>,
    pub cached: Option<bool>,
}

#[derive(Serialize)]
struct LicenseApiRequest {
    key: String,
    hardware_id: String,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn hardware_id_formatted(raw: &str) -> String {
    let uuid = Uuid::new_v5(&Uuid::NAMESPACE_DNS, raw.as_bytes());
    format!("HW-{}", uuid.to_string().to_uppercase())
}

/// Plaintext key stored by an older version → try to decrypt, fall back to raw
fn resolve_stored_key(stored: &str) -> Option<String> {
    if let Some(k) = hw_decrypt_string(stored) {
        return Some(k);
    }
    // Legacy plaintext (pre-encryption)
    if stored.starts_with("SV-") {
        Some(stored.to_string())
    } else {
        None
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Returns the machine's unique hardware ID (for display).
#[tauri::command]
#[specta::specta]
pub async fn get_hardware_id() -> Result<String, String> {
    let raw = hw_id_raw();
    Ok(hardware_id_formatted(&raw))
}

/// Activates a license key via the backend API.
/// On success the key is encrypted with AES-256-GCM and stored in SQLite.
#[tauri::command]
#[specta::specta]
pub async fn activate_license(
    state: State<'_, AppState>,
    key: String,
) -> Result<ActivateResult, String> {
    let hw_raw = hw_id_raw();
    let hw_id = hardware_id_formatted(&hw_raw);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .post(format!("{}/licenses/activate", LICENSE_API))
        .json(&LicenseApiRequest {
            key: key.clone(),
            hardware_id: hw_id,
        })
        .send()
        .await
        .map_err(|e| format!("Sunucuya bağlanılamadı: {}", e))?;

    let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    let is_activated = data.get("valid").and_then(|v| v.as_bool()).unwrap_or(false)
        || data.get("activated_at").and_then(|v| v.as_str()).is_some();

    if is_activated {
        let encrypted = hw_encrypt(&key).map_err(|e| e.to_string())?;
        queries::upsert_setting(&state.db, "license_key", &encrypted)
            .await
            .map_err(|e| e.to_string())?;
        Ok(ActivateResult {
            success: true,
            error: None,
        })
    } else {
        let msg = data
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Geçersiz lisans anahtarı.")
            .to_string();
        Ok(ActivateResult {
            success: false,
            error: Some(msg),
        })
    }
}

/// Validates the stored license key against the backend API.
/// Falls back to "valid" (offline grace) when the network is unreachable.
/// Rate-limited: skips the API call if validated within the last 60 seconds.
#[tauri::command]
#[specta::specta]
pub async fn validate_license(state: State<'_, AppState>) -> Result<ValidateResult, String> {
    let hw_id = hardware_id_formatted(&hw_id_raw());

    let stored = match queries::get_setting(&state.db, "license_key")
        .await
        .map_err(|e| e.to_string())?
    {
        Some(k) if !k.is_empty() => k,
        _ => {
            return Ok(ValidateResult {
                status: "invalid".to_string(),
                offline: None,
                cached: None,
            })
        }
    };

    let key = match resolve_stored_key(&stored) {
        Some(k) => k,
        None => {
            return Ok(ValidateResult {
                status: "invalid".to_string(),
                offline: None,
                cached: None,
            })
        }
    };

    // Rate limit: if last validation was < 60s ago, return cached "valid"
    if let Ok(Some(ts_str)) = queries::get_setting(&state.db, "license_validated_at").await {
        if let Ok(ts) = ts_str.parse::<i64>() {
            let elapsed = chrono::Utc::now().timestamp() - ts;
            if elapsed < 60 {
                return Ok(ValidateResult {
                    status: "valid".to_string(),
                    offline: None,
                    cached: Some(true),
                });
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    match client
        .post(format!("{}/licenses/validate", LICENSE_API))
        .json(&LicenseApiRequest {
            key,
            hardware_id: hw_id,
        })
        .send()
        .await
    {
        Ok(response) => {
            let data: serde_json::Value = response.json().await.unwrap_or_default();
            let valid = data.get("valid").and_then(|v| v.as_bool()).unwrap_or(false);
            if valid {
                let _ = queries::upsert_setting(
                    &state.db,
                    "license_validated_at",
                    &chrono::Utc::now().timestamp().to_string(),
                )
                .await;
            }
            let status = if valid { "valid" } else { "invalid" }.to_string();
            Ok(ValidateResult {
                status,
                offline: None,
                cached: None,
            })
        }
        Err(_) => Ok(ValidateResult {
            status: "valid".to_string(),
            offline: Some(true),
            cached: None,
        }),
    }
}

/// Returns the decrypted stored key (for display / compatibility).
#[tauri::command]
#[specta::specta]
pub async fn get_stored_license(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let stored = queries::get_setting(&state.db, "license_key")
        .await
        .map_err(|e| e.to_string())?;

    Ok(stored.and_then(|k| {
        if k.is_empty() {
            None
        } else {
            resolve_stored_key(&k)
        }
    }))
}

/// Stores a license key encrypted. Kept for backward compatibility.
#[tauri::command]
#[specta::specta]
pub async fn store_license(state: State<'_, AppState>, key: String) -> Result<(), String> {
    let encrypted = hw_encrypt(&key).map_err(|e| e.to_string())?;
    queries::upsert_setting(&state.db, "license_key", &encrypted)
        .await
        .map_err(|e| e.to_string())
}

/// Core activation logic shared between the Tauri command and the deep link handler.
/// Returns Ok(true) if activated, Ok(false) if invalid key.
pub async fn activate_license_with_key(
    db: &std::sync::Arc<sqlx::SqlitePool>,
    key: &str,
) -> anyhow::Result<bool> {
    let hw_id = hardware_id_formatted(&hw_id_raw());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client
        .post(format!("{}/licenses/activate", LICENSE_API))
        .json(&LicenseApiRequest {
            key: key.to_string(),
            hardware_id: hw_id,
        })
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Sunucuya bağlanılamadı: {}", e))?;

    let data: serde_json::Value = response.json().await?;

    let is_activated = data.get("valid").and_then(|v| v.as_bool()).unwrap_or(false)
        || data.get("activated_at").and_then(|v| v.as_str()).is_some();

    if is_activated {
        let encrypted = hw_encrypt(key)?;
        queries::upsert_setting(db, "license_key", &encrypted).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Clears the stored license key (local only, no server call).
#[tauri::command]
#[specta::specta]
pub async fn clear_license(state: State<'_, AppState>) -> Result<(), String> {
    queries::upsert_setting(&state.db, "license_key", "")
        .await
        .map_err(|e| e.to_string())
}

/// Deactivates the license on the server and clears it locally.
/// Allows the user to activate on a different device afterwards.
#[tauri::command]
#[specta::specta]
pub async fn deactivate_license(state: State<'_, AppState>) -> Result<(), String> {
    let hw_id = hardware_id_formatted(&hw_id_raw());

    let stored = queries::get_setting(&state.db, "license_key")
        .await
        .map_err(|e| e.to_string())?;

    if let Some(s) = stored.filter(|k| !k.is_empty()) {
        if let Some(key) = resolve_stored_key(&s) {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| e.to_string())?;

            let resp = client
                .post(format!("{}/licenses/deactivate", LICENSE_API))
                .json(&LicenseApiRequest {
                    key,
                    hardware_id: hw_id,
                })
                .send()
                .await;

            // If server is unreachable we still clear locally so the user isn't stuck.
            if let Ok(r) = resp {
                let data: serde_json::Value = r.json().await.unwrap_or_default();
                let ok = data
                    .get("deactivated")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                if !ok {
                    let msg = data
                        .get("message")
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
