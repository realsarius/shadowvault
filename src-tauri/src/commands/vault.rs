use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{State, Manager};
use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::AppState;
use crate::vault::{
    crypto::{derive_key, generate_salt},
    fs::{
        VaultEntry, VaultMeta,
        create_vault as vault_create_dir, import_file, import_directory,
        export_file, delete_entry, create_directory as vault_create_dir_entry,
        rename_entry, move_entry, generate_thumbnail, change_password,
        decrypt_to_temp, secure_delete_temp, reencrypt_from_temp,
    },
    session::{SessionStore, OpenFileSummary},
};

// ─── Yardımcı Tipler ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSummary {
    pub id: String,
    pub name: String,
    pub algorithm: String,
    pub vault_path: String,
    pub created_at: String,
    pub last_opened: Option<String>,
    pub unlocked: bool,
}

// ─── Yardımcı Fonksiyonlar ──────────────────────────────────────────────────

/// Uygulama data dizini içinde vaults klasörünü döner.
fn vaults_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = data.join("vaults");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn vault_path_for(app: &tauri::AppHandle, vault_id: &str) -> Result<PathBuf, String> {
    Ok(vaults_dir(app)?.join(vault_id))
}

/// SessionStore'u AppState'ten al (lazy init ile manage edildi).
fn get_session(state: &State<'_, Arc<SessionStore>>) -> Arc<SessionStore> {
    state.inner().clone()
}

// ─── Tauri Komutları ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn create_vault(
    app: tauri::AppHandle,
    db_state: State<'_, AppState>,
    session: State<'_, Arc<SessionStore>>,
    name: String,
    password: String,
    algorithm: Option<String>,
) -> Result<VaultSummary, String> {
    use uuid::Uuid;
    use chrono::Utc;

    // Freemium: 3 kasa limiti
    let pool = &db_state.db;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vaults")
        .fetch_one(pool.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    // License kontrolü
    let is_pro = sqlx::query_scalar::<_, String>(
        "SELECT value FROM settings WHERE key = 'license_status'",
    )
    .fetch_optional(pool.as_ref())
    .await
    .map_err(|e| e.to_string())?
    .map(|v| v == "valid")
    .unwrap_or(false);

    if !is_pro && count >= 3 {
        return Err("vault_limit_reached".to_string());
    }

    let vault_id = Uuid::new_v4().simple().to_string();
    let dir = vaults_dir(&app)?;

    let algo = algorithm.unwrap_or_else(|| "AES-256-GCM".to_string());

    // Salt + key türet
    let salt = generate_salt();
    let master_key = derive_key(&password, &salt).map_err(|e| e.to_string())?;

    // Kasa klasörü + ilk meta oluştur
    vault_create_dir(&dir, &vault_id, &name, &algo, &master_key).map_err(|e| e.to_string())?;

    // Salt'ı ayrı bir plaintext dosyasına yaz (unlock için gerekli)
    let salt_path = dir.join(&vault_id).join(".shadow_salt");
    std::fs::write(&salt_path, hex::encode(salt)).map_err(|e| e.to_string())?;

    // DB'ye kaydet
    let now = Utc::now().to_rfc3339();
    let vault_path_str = dir.join(&vault_id).to_string_lossy().to_string();

    sqlx::query(
        "INSERT INTO vaults (id, name, algorithm, vault_path, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&vault_id)
    .bind(&name)
    .bind(&algo)
    .bind(&vault_path_str)
    .bind(&now)
    .execute(pool.as_ref())
    .await
    .map_err(|e| e.to_string())?;

    // Session'a ekle
    let sess = get_session(&session);
    sess.0.lock().unwrap().unlock(&vault_id, master_key);

    Ok(VaultSummary {
        id: vault_id,
        name,
        algorithm: algo,
        vault_path: vault_path_str,
        created_at: now,
        last_opened: None,
        unlocked: true,
    })
}

#[tauri::command]
pub async fn list_vaults(
    db_state: State<'_, AppState>,
    session: State<'_, Arc<SessionStore>>,
) -> Result<Vec<VaultSummary>, String> {
    let pool = &db_state.db;
    let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<String>)>(
        "SELECT id, name, algorithm, vault_path, created_at, last_opened FROM vaults ORDER BY created_at DESC",
    )
    .fetch_all(pool.as_ref())
    .await
    .map_err(|e| e.to_string())?;

    let sess = get_session(&session);
    let guard = sess.0.lock().unwrap();

    let vaults = rows
        .into_iter()
        .map(|(id, name, algorithm, vault_path, created_at, last_opened)| {
            let unlocked = guard.is_unlocked(&id);
            VaultSummary { id, name, algorithm, vault_path, created_at, last_opened, unlocked }
        })
        .collect();

    Ok(vaults)
}

#[tauri::command]
pub async fn unlock_vault(
    app: tauri::AppHandle,
    db_state: State<'_, AppState>,
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    password: String,
) -> Result<(), String> {
    use chrono::Utc;

    let pool = &db_state.db;
    let vault_path = vault_path_for(&app, &vault_id)?;

    // Meta dosyasını okuyarak salt al
    let meta_path = vault_path.join(".shadow_meta");
    if !meta_path.exists() {
        return Err("Vault not found".to_string());
    }

    // Salt için önce geçici bir anahtar türetemeyiz; salt meta içinde.
    // Meta'yı decrypt etmeden önce salt'ı meta'nın dışına çıkarmamız gerekiyor.
    // Çözüm: ilk 32 byte'ı salt olarak encode'lamak yerine,
    // vault meta dosyasının önünde unencrypted salt sakla.
    // Ancak mevcut tasarımda salt, şifreli meta içinde. Bu nedenle
    // salt'ı ayrı bir plaintext dosyasında tutalım (.shadow_salt).
    let salt_path = vault_path.join(".shadow_salt");
    let salt_hex = std::fs::read_to_string(&salt_path)
        .map_err(|_| "Vault salt not found".to_string())?;
    let salt_bytes = hex::decode(salt_hex.trim())
        .map_err(|_| "Invalid vault salt".to_string())?;

    let master_key = derive_key(&password, &salt_bytes).map_err(|e| e.to_string())?;

    // Meta'yı decrypt etmeye çalış (yanlış şifre → hata)
    VaultMeta::load(&vault_path, &master_key).map_err(|e| e.to_string())?;

    // Session'a ekle — [u8;32] Copy olduğundan session kopyayı alır
    let sess = get_session(&session);
    sess.0.lock().unwrap().unlock(&vault_id, master_key);

    // last_opened güncelle
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE vaults SET last_opened = ? WHERE id = ?")
        .bind(&now)
        .bind(&vault_id)
        .execute(pool.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn lock_vault(
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
) -> Result<(), String> {
    let sess = get_session(&session);
    sess.0.lock().unwrap().lock(&vault_id);
    Ok(())
}

#[tauri::command]
pub async fn list_entries(
    app: tauri::AppHandle,
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    parent_id: Option<String>,
) -> Result<Vec<VaultEntry>, String> {
    let sess = get_session(&session);
    let guard = sess.0.lock().unwrap();
    let master_key = guard
        .get_key(&vault_id)
        .ok_or_else(|| "Vault is locked".to_string())?;

    let vault_path = vault_path_for(&app, &vault_id)?;
    let meta = VaultMeta::load(&vault_path, &master_key).map_err(|e| e.to_string())?;

    let entries: Vec<VaultEntry> = meta
        .entries
        .into_iter()
        .filter(|e| e.parent_id.as_deref() == parent_id.as_deref())
        .collect();

    Ok(entries)
}

#[tauri::command]
pub async fn import_file_cmd(
    app: tauri::AppHandle,
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    src_path: String,
    parent_id: Option<String>,
) -> Result<VaultEntry, String> {
    let sess = get_session(&session);
    let guard = sess.0.lock().unwrap();
    let master_key = guard
        .get_key(&vault_id)
        .ok_or_else(|| "Vault is locked".to_string())?;

    let vault_path = vault_path_for(&app, &vault_id)?;
    let mut meta = VaultMeta::load(&vault_path, &master_key).map_err(|e| e.to_string())?;

    let entry = import_file(
        &vault_path,
        &mut meta,
        Path::new(&src_path),
        parent_id.as_deref(),
        &master_key,
    )
    .map_err(|e| e.to_string())?;

    Ok(entry)
}

#[tauri::command]
pub async fn import_directory_cmd(
    app: tauri::AppHandle,
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    src_path: String,
    parent_id: Option<String>,
) -> Result<VaultEntry, String> {
    let sess = get_session(&session);
    let guard = sess.0.lock().unwrap();
    let master_key = guard
        .get_key(&vault_id)
        .ok_or_else(|| "Vault is locked".to_string())?;

    let vault_path = vault_path_for(&app, &vault_id)?;
    let mut meta = VaultMeta::load(&vault_path, &master_key).map_err(|e| e.to_string())?;

    let entry = import_directory(
        &vault_path,
        &mut meta,
        Path::new(&src_path),
        parent_id.as_deref(),
        &master_key,
    )
    .map_err(|e| e.to_string())?;

    Ok(entry)
}

#[tauri::command]
pub async fn export_file_cmd(
    app: tauri::AppHandle,
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    entry_id: String,
    dest_path: String,
) -> Result<(), String> {
    let sess = get_session(&session);
    let guard = sess.0.lock().unwrap();
    let master_key = guard
        .get_key(&vault_id)
        .ok_or_else(|| "Vault is locked".to_string())?;

    let vault_path = vault_path_for(&app, &vault_id)?;
    let meta = VaultMeta::load(&vault_path, &master_key).map_err(|e| e.to_string())?;
    let entry = meta
        .entries
        .iter()
        .find(|e| e.id == entry_id)
        .ok_or_else(|| "Entry not found".to_string())?;

    export_file(&vault_path, entry, Path::new(&dest_path), &master_key, &meta.algorithm)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_file_cmd(
    app: tauri::AppHandle,
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    entry_id: String,
) -> Result<(), String> {
    let sess = get_session(&session);
    let mut guard = sess.0.lock().unwrap();
    let master_key = guard
        .get_key(&vault_id)
        .ok_or_else(|| "Vault is locked".to_string())?;

    let vault_path = vault_path_for(&app, &vault_id)?;
    let meta = VaultMeta::load(&vault_path, &master_key).map_err(|e| e.to_string())?;
    let entry = meta
        .entries
        .iter()
        .find(|e| e.id == entry_id)
        .cloned()
        .ok_or_else(|| "Entry not found".to_string())?;

    let algorithm = meta.algorithm.clone();
    let tmp_path = decrypt_to_temp(&vault_path, &entry, &master_key, &algorithm)
        .map_err(|e| e.to_string())?;

    // Açık dosyayı session'a kaydet — kilitlemeden önce re-encrypt için
    guard.register_open_file(
        tmp_path.clone(),
        &vault_id,
        &entry.id,
        &entry.name,
        vault_path,
    );
    drop(guard);

    open::that(&tmp_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Belirli bir kasa için dışarıda açık olan dosyaları listeler.
#[tauri::command]
pub async fn get_open_files(
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
) -> Result<Vec<OpenFileSummary>, String> {
    let sess = get_session(&session);
    let guard = sess.0.lock().unwrap();
    let list = guard
        .get_open_files_for_vault(&vault_id)
        .into_iter()
        .map(|(path, entry)| OpenFileSummary {
            entry_id: entry.entry_id,
            file_name: entry.file_name,
            tmp_path: path.to_string_lossy().to_string(),
        })
        .collect();
    Ok(list)
}

/// Açık dosyaları sync edip kasayı kilitler.
/// `save = true` → değişiklikleri re-encrypt et, sonra kilitle.
/// `save = false` → değişiklikleri yok say, sadece kilitle (discard).
#[tauri::command]
pub async fn sync_and_lock_vault(
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    save: bool,
) -> Result<(), String> {
    let sess = get_session(&session);

    // Açık dosyaları ve key'i al (mutex serbest bırak, I/O öncesinde)
    let (open_files, master_key) = {
        let guard = sess.0.lock().unwrap();
        let key = guard.get_key(&vault_id);
        let files = guard.get_open_files_for_vault(&vault_id);
        (files, key)
    };

    // Sync veya discard
    for (tmp_path, entry) in &open_files {
        if save {
            if let Some(key) = &master_key {
                // Algorithm'i meta'dan oku
                let algorithm = VaultMeta::load(&entry.vault_path, key)
                    .map(|m| m.algorithm)
                    .unwrap_or_else(|_| "AES-256-GCM".to_string());
                reencrypt_from_temp(&entry.vault_path, &entry.entry_id, tmp_path, key, &algorithm)
                    .map_err(|e| e.to_string())?;
            }
        }
        secure_delete_temp(tmp_path).ok();
    }

    // Session'dan temizle + kilitle
    {
        let mut guard = sess.0.lock().unwrap();
        for (tmp_path, _) in &open_files {
            guard.unregister_open_file(tmp_path);
        }
        guard.lock(&vault_id);
    }

    Ok(())
}

#[tauri::command]
pub async fn rename_entry_cmd(
    app: tauri::AppHandle,
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    entry_id: String,
    new_name: String,
) -> Result<(), String> {
    let sess = get_session(&session);
    let guard = sess.0.lock().unwrap();
    let master_key = guard
        .get_key(&vault_id)
        .ok_or_else(|| "Vault is locked".to_string())?;

    let vault_path = vault_path_for(&app, &vault_id)?;
    let mut meta = VaultMeta::load(&vault_path, &master_key).map_err(|e| e.to_string())?;
    rename_entry(&mut meta, &entry_id, &new_name).map_err(|e| e.to_string())?;
    meta.save(&vault_path, &master_key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn move_entry_cmd(
    app: tauri::AppHandle,
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    entry_id: String,
    new_parent_id: Option<String>,
) -> Result<(), String> {
    let sess = get_session(&session);
    let guard = sess.0.lock().unwrap();
    let master_key = guard
        .get_key(&vault_id)
        .ok_or_else(|| "Vault is locked".to_string())?;

    let vault_path = vault_path_for(&app, &vault_id)?;
    let mut meta = VaultMeta::load(&vault_path, &master_key).map_err(|e| e.to_string())?;
    move_entry(&mut meta, &entry_id, new_parent_id.as_deref())
        .map_err(|e| e.to_string())?;
    meta.save(&vault_path, &master_key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_entry_cmd(
    app: tauri::AppHandle,
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    entry_id: String,
) -> Result<(), String> {
    let sess = get_session(&session);
    let guard = sess.0.lock().unwrap();
    let master_key = guard
        .get_key(&vault_id)
        .ok_or_else(|| "Vault is locked".to_string())?;

    let vault_path = vault_path_for(&app, &vault_id)?;
    let mut meta = VaultMeta::load(&vault_path, &master_key).map_err(|e| e.to_string())?;
    delete_entry(&vault_path, &mut meta, &entry_id).map_err(|e| e.to_string())?;
    meta.save(&vault_path, &master_key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_directory_cmd(
    app: tauri::AppHandle,
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    name: String,
    parent_id: Option<String>,
) -> Result<VaultEntry, String> {
    let sess = get_session(&session);
    let guard = sess.0.lock().unwrap();
    let master_key = guard
        .get_key(&vault_id)
        .ok_or_else(|| "Vault is locked".to_string())?;

    let vault_path = vault_path_for(&app, &vault_id)?;
    let mut meta = VaultMeta::load(&vault_path, &master_key).map_err(|e| e.to_string())?;
    let entry = vault_create_dir_entry(&mut meta, &name, parent_id.as_deref())
        .map_err(|e| e.to_string())?;
    meta.save(&vault_path, &master_key).map_err(|e| e.to_string())?;
    Ok(entry)
}

#[tauri::command]
pub async fn get_thumbnail(
    app: tauri::AppHandle,
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    entry_id: String,
) -> Result<String, String> {
    let sess = get_session(&session);
    let guard = sess.0.lock().unwrap();
    let master_key = guard
        .get_key(&vault_id)
        .ok_or_else(|| "Vault is locked".to_string())?;

    let vault_path = vault_path_for(&app, &vault_id)?;
    let meta = VaultMeta::load(&vault_path, &master_key).map_err(|e| e.to_string())?;
    let entry = meta
        .entries
        .iter()
        .find(|e| e.id == entry_id)
        .ok_or_else(|| "Entry not found".to_string())?;

    generate_thumbnail(&vault_path, entry, &master_key, &meta.algorithm).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_vault(
    app: tauri::AppHandle,
    db_state: State<'_, AppState>,
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    password: String,
) -> Result<(), String> {
    let vault_path = vault_path_for(&app, &vault_id)?;

    // Şifreyi doğrula
    let salt_path = vault_path.join(".shadow_salt");
    let salt_hex = std::fs::read_to_string(&salt_path)
        .map_err(|_| "Vault salt not found".to_string())?;
    let salt_bytes = hex::decode(salt_hex.trim())
        .map_err(|_| "Invalid vault salt".to_string())?;

    let master_key = derive_key(&password, &salt_bytes).map_err(|e| e.to_string())?;
    VaultMeta::load(&vault_path, &master_key).map_err(|_| "Wrong password".to_string())?;

    // Session'dan kaldır
    let sess = get_session(&session);
    sess.0.lock().unwrap().lock(&vault_id);

    // Klasörü sil
    std::fs::remove_dir_all(&vault_path).map_err(|e| e.to_string())?;

    // DB'den sil
    sqlx::query("DELETE FROM vaults WHERE id = ?")
        .bind(&vault_id)
        .execute(db_state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn change_vault_password(
    app: tauri::AppHandle,
    session: State<'_, Arc<SessionStore>>,
    vault_id: String,
    old_password: String,
    new_password: String,
) -> Result<(), String> {
    let vault_path = vault_path_for(&app, &vault_id)?;

    let salt_path = vault_path.join(".shadow_salt");
    let salt_hex = std::fs::read_to_string(&salt_path)
        .map_err(|_| "Vault salt not found".to_string())?;
    let old_salt = hex::decode(salt_hex.trim())
        .map_err(|_| "Invalid vault salt".to_string())?;

    let old_key = derive_key(&old_password, &old_salt).map_err(|e| e.to_string())?;
    let new_salt = generate_salt();
    let new_key = derive_key(&new_password, &new_salt).map_err(|e| e.to_string())?;

    change_password(&vault_path, &old_key, &new_key, &new_salt)
        .map_err(|e| e.to_string())?;

    // Salt dosyasını güncelle
    std::fs::write(&salt_path, hex::encode(new_salt)).map_err(|e| e.to_string())?;

    // Varsa session'ı güncelle
    let sess = get_session(&session);
    let mut guard = sess.0.lock().unwrap();
    if guard.is_unlocked(&vault_id) {
        guard.unlock(&vault_id, new_key);
    }

    Ok(())
}
