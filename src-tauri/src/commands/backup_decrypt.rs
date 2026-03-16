use tauri::State;
use crate::AppState;
use crate::engine::copier::derive_backup_key_from_password;
use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};

/// Decrypts all `.enc` files in `folder_path` that were encrypted by ShadowVault.
/// Reads the manifest in that folder to get the Argon2id salt, then decrypts each file.
/// `password` is the plaintext encryption password set by the user.
#[tauri::command]
#[specta::specta]
pub async fn decrypt_backup(
    _state: State<'_, AppState>,
    folder_path: String,
    password: String,
) -> Result<u32, String> {
    let dir = std::path::Path::new(&folder_path);

    // Read manifest
    let manifest_path = dir.join("shadowvault_enc_manifest.json");
    if !manifest_path.exists() {
        return Err("Bu klasörde şifreli yedek manifesti bulunamadı.".to_string());
    }
    let manifest_text = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_text).map_err(|e| e.to_string())?;

    let encrypted = manifest.get("encrypted").and_then(|v| v.as_bool()).unwrap_or(false);
    if !encrypted {
        return Err("Bu klasör şifreli değil.".to_string());
    }

    let argon2_salt = manifest.get("argon2_salt")
        .and_then(|v| v.as_str())
        .ok_or("Manifest'te argon2_salt bulunamadı.")?;

    let master_key = derive_backup_key_from_password(&password, argon2_salt)
        .map_err(|e| e.to_string())?;

    let cipher_key = Key::<Aes256Gcm>::from_slice(&master_key);
    let cipher = Aes256Gcm::new(cipher_key);

    let mut decrypted_count: u32 = 0;

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_file() { continue; }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("enc") { continue; }

        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        if data.len() < 12 {
            return Err(format!("Dosya çok kısa (bozuk): {:?}", path));
        }

        let nonce = Nonce::from_slice(&data[..12]);
        let plaintext = cipher.decrypt(nonce, &data[12..])
            .map_err(|_| format!("Şifre çözme başarısız — yanlış şifre veya bozuk dosya: {:?}", path))?;

        // Reconstruct original filename by removing .enc extension
        let original_name = path.file_name()
            .unwrap()
            .to_string_lossy();
        let original_name = original_name.strip_suffix(".enc")
            .unwrap_or(&original_name);
        let original_path = path.with_file_name(original_name);

        std::fs::write(&original_path, &plaintext).map_err(|e| e.to_string())?;
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
        decrypted_count += 1;
    }

    // Remove manifest after successful decryption
    if decrypted_count > 0 {
        std::fs::remove_file(&manifest_path).ok();
    }

    Ok(decrypted_count)
}
