use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Result};
use chrono::Utc;
use uuid::Uuid;
use zeroize::Zeroize;

use super::crypto::{
    decrypt, decrypt_data, derive_subkey, encrypt, encrypt_data, generate_salt, KEY_LEN, SALT_LEN,
};

// ─── Vault Metadata Tipleri ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub kind: EntryKind,
    /// Bayt cinsinden boyut (sadece dosyalar)
    pub size: Option<u64>,
    pub modified: Option<String>,
    /// Dosya şifreleme nonce'u (hex), sadece dosyalar
    pub nonce: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMeta {
    pub version: u32,
    pub name: String,
    pub algorithm: String,
    pub kdf: String,
    pub kdf_params: KdfParams,
    /// Salt (hex)
    pub salt: String,
    pub entries: Vec<VaultEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdfParams {
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl VaultMeta {
    fn meta_file(vault_path: &Path) -> PathBuf {
        vault_path.join(".shadow_meta")
    }

    /// Metadata'yı şifrele ve diske yaz.
    pub fn save(&self, vault_path: &Path, master_key: &[u8; KEY_LEN]) -> Result<()> {
        let json = serde_json::to_vec(self)?;
        let mut meta_key = derive_subkey(master_key, b"meta");
        let encrypted = encrypt(&meta_key, &json)?;
        meta_key.zeroize();
        std::fs::write(Self::meta_file(vault_path), &encrypted)?;
        Ok(())
    }

    /// Diskten oku, şifre çöz, parse et.
    pub fn load(vault_path: &Path, master_key: &[u8; KEY_LEN]) -> Result<Self> {
        let encrypted = std::fs::read(Self::meta_file(vault_path))
            .map_err(|_| anyhow!("Vault metadata not found"))?;
        let mut meta_key = derive_subkey(master_key, b"meta");
        let plaintext = decrypt(&meta_key, &encrypted)
            .map_err(|_| anyhow!("Wrong password or corrupted vault"))?;
        meta_key.zeroize();
        let meta: VaultMeta = serde_json::from_slice(&plaintext)?;
        Ok(meta)
    }
}

// ─── Kasa CRUD ──────────────────────────────────────────────────────────────

/// Yeni kasa klasörünü ve ilk metadata'yı oluşturur.
/// `vaults_dir`: ~/.shadowvault/vaults/
pub fn create_vault(
    vaults_dir: &Path,
    vault_id: &str,
    name: &str,
    algorithm: &str,
    master_key: &[u8; KEY_LEN],
) -> Result<PathBuf> {
    let vault_path = vaults_dir.join(vault_id);
    std::fs::create_dir_all(&vault_path)?;

    let salt = generate_salt();
    let meta = VaultMeta {
        version: 1,
        name: name.to_string(),
        algorithm: algorithm.to_string(),
        kdf: "Argon2id".to_string(),
        kdf_params: KdfParams {
            m_cost: super::crypto::ARGON2_M_COST,
            t_cost: super::crypto::ARGON2_T_COST,
            p_cost: super::crypto::ARGON2_P_COST,
        },
        salt: hex::encode(salt),
        entries: vec![],
    };
    meta.save(&vault_path, master_key)?;
    Ok(vault_path)
}

// ─── Dosya İşlemleri ────────────────────────────────────────────────────────

/// Bir dosyayı kasaya şifreleyerek ekler.
pub fn import_file(
    vault_path: &Path,
    meta: &mut VaultMeta,
    src: &Path,
    parent_id: Option<&str>,
    master_key: &[u8; KEY_LEN],
) -> Result<VaultEntry> {
    let file_id = Uuid::new_v4().simple().to_string();
    let plaintext = std::fs::read(src)?;
    let size = plaintext.len() as u64;

    // Her dosyanın kendi anahtarı
    let mut file_key = derive_subkey(master_key, file_id.as_bytes());
    let ciphertext = encrypt_data(&meta.algorithm, &file_key, &plaintext)?;
    file_key.zeroize();

    std::fs::write(vault_path.join(&file_id), &ciphertext)?;

    let name = src
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    let entry = VaultEntry {
        id: file_id,
        name,
        parent_id: parent_id.map(str::to_string),
        kind: EntryKind::File,
        size: Some(size),
        modified: Some(Utc::now().to_rfc3339()),
        nonce: None, // nonce ciphertext içinde embedded
    };
    meta.entries.push(entry.clone());
    meta.save(vault_path, master_key)?;
    Ok(entry)
}

/// Bir klasörü ve içeriğini özyinelemeli olarak kasaya ekler.
pub fn import_directory(
    vault_path: &Path,
    meta: &mut VaultMeta,
    src_dir: &Path,
    parent_id: Option<&str>,
    master_key: &[u8; KEY_LEN],
) -> Result<VaultEntry> {
    let dir_id = Uuid::new_v4().simple().to_string();
    let name = src_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("folder")
        .to_string();

    let dir_entry = VaultEntry {
        id: dir_id.clone(),
        name,
        parent_id: parent_id.map(str::to_string),
        kind: EntryKind::Directory,
        size: None,
        modified: Some(Utc::now().to_rfc3339()),
        nonce: None,
    };
    meta.entries.push(dir_entry.clone());

    for entry in std::fs::read_dir(src_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            import_directory(vault_path, meta, &path, Some(&dir_id), master_key)?;
        } else if path.is_file() {
            import_file(vault_path, meta, &path, Some(&dir_id), master_key)?;
        }
    }

    meta.save(vault_path, master_key)?;
    Ok(dir_entry)
}

/// Bir dosyayı kasadan dışa aktarır (decrypt → hedef yola yaz).
pub fn export_file(
    vault_path: &Path,
    entry: &VaultEntry,
    dest: &Path,
    master_key: &[u8; KEY_LEN],
    algorithm: &str,
) -> Result<()> {
    if entry.kind != EntryKind::File {
        return Err(anyhow!("Not a file"));
    }
    let ciphertext = std::fs::read(vault_path.join(&entry.id))?;
    let mut file_key = derive_subkey(master_key, entry.id.as_bytes());
    let plaintext = decrypt_data(algorithm, &file_key, &ciphertext)?;
    file_key.zeroize();
    std::fs::write(dest, &plaintext)?;
    Ok(())
}

/// Bir dosyayı OS temp klasörüne decrypt ederek açar.
/// Güvenli silme: çağıran taraftan bekleniyor (open_file command'ında).
pub fn decrypt_to_temp(
    vault_path: &Path,
    entry: &VaultEntry,
    master_key: &[u8; KEY_LEN],
    algorithm: &str,
) -> Result<PathBuf> {
    if entry.kind != EntryKind::File {
        return Err(anyhow!("Not a file"));
    }
    let ciphertext = std::fs::read(vault_path.join(&entry.id))?;
    let mut file_key = derive_subkey(master_key, entry.id.as_bytes());
    let mut plaintext = decrypt_data(algorithm, &file_key, &ciphertext)?;
    file_key.zeroize();

    let tmp_dir = std::env::temp_dir().join("shadowvault_tmp");
    std::fs::create_dir_all(&tmp_dir)?;

    let ext = Path::new(&entry.name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let tmp_name = if ext.is_empty() {
        entry.name.clone()
    } else {
        format!("{}.{}", &entry.id[..8], ext)
    };
    let tmp_path = tmp_dir.join(tmp_name);
    std::fs::write(&tmp_path, &plaintext)?;
    plaintext.zeroize();
    Ok(tmp_path)
}

/// Temp dosyadaki güncel içeriği alıp kasadaki blob'u re-encrypt eder.
/// Kullanıcı dışarıda bir dosyayı düzenlediğinde değişiklikleri kasaya geri yazar.
pub fn reencrypt_from_temp(
    vault_path: &Path,
    entry_id: &str,
    tmp_path: &Path,
    master_key: &[u8; KEY_LEN],
    algorithm: &str,
) -> Result<()> {
    let mut plaintext = std::fs::read(tmp_path)?;
    let mut file_key = derive_subkey(master_key, entry_id.as_bytes());
    let ciphertext = encrypt_data(algorithm, &file_key, &plaintext)?;
    file_key.zeroize();
    plaintext.zeroize();
    std::fs::write(vault_path.join(entry_id), ciphertext)?;
    Ok(())
}

/// Temp dosyayı güvenli sil (üzerine random yaz + sil).
pub fn secure_delete_temp(path: &Path) -> Result<()> {
    use rand::RngCore;
    let len = std::fs::metadata(path)?.len() as usize;
    let mut garbage = vec![0u8; len];
    rand::rngs::OsRng.fill_bytes(&mut garbage);
    std::fs::write(path, &garbage)?;
    std::fs::remove_file(path)?;
    Ok(())
}

/// Entry'yi yeniden adlandırır.
pub fn rename_entry(meta: &mut VaultMeta, entry_id: &str, new_name: &str) -> Result<()> {
    let entry = meta
        .entries
        .iter_mut()
        .find(|e| e.id == entry_id)
        .ok_or_else(|| anyhow!("Entry not found"))?;
    entry.name = new_name.to_string();
    Ok(())
}

/// Entry'yi başka bir klasöre taşır.
pub fn move_entry(
    meta: &mut VaultMeta,
    entry_id: &str,
    new_parent_id: Option<&str>,
) -> Result<()> {
    let entry = meta
        .entries
        .iter_mut()
        .find(|e| e.id == entry_id)
        .ok_or_else(|| anyhow!("Entry not found"))?;
    entry.parent_id = new_parent_id.map(str::to_string);
    Ok(())
}

/// Entry'yi ve varsa alt entry'lerini (klasör ise) siler.
pub fn delete_entry(
    vault_path: &Path,
    meta: &mut VaultMeta,
    entry_id: &str,
) -> Result<()> {
    // Özyinelemeli olarak tüm altındakileri topla
    let to_delete = collect_subtree(meta, entry_id);

    for id in &to_delete {
        // Blob dosyasını sil (klasörler için blob yok)
        let blob = vault_path.join(id);
        if blob.exists() {
            std::fs::remove_file(&blob).ok();
        }
    }

    meta.entries.retain(|e| !to_delete.contains(&e.id));
    Ok(())
}

fn collect_subtree(meta: &VaultMeta, root_id: &str) -> Vec<String> {
    let mut result = vec![root_id.to_string()];
    let mut i = 0;
    while i < result.len() {
        let current = result[i].clone();
        for e in &meta.entries {
            if e.parent_id.as_deref() == Some(&current) {
                result.push(e.id.clone());
            }
        }
        i += 1;
    }
    result
}

/// Yeni boş klasör entry'si oluşturur.
pub fn create_directory(
    meta: &mut VaultMeta,
    name: &str,
    parent_id: Option<&str>,
) -> Result<VaultEntry> {
    let entry = VaultEntry {
        id: Uuid::new_v4().simple().to_string(),
        name: name.to_string(),
        parent_id: parent_id.map(str::to_string),
        kind: EntryKind::Directory,
        size: None,
        modified: Some(Utc::now().to_rfc3339()),
        nonce: None,
    };
    meta.entries.push(entry.clone());
    Ok(entry)
}

/// Thumbnail üretir (base64 PNG, max 200x200). Sadece resim dosyaları.
pub fn generate_thumbnail(
    vault_path: &Path,
    entry: &VaultEntry,
    master_key: &[u8; KEY_LEN],
    algorithm: &str,
) -> Result<String> {
    use image::imageops::FilterType;
    use base64::{Engine as _, engine::general_purpose};

    let ciphertext = std::fs::read(vault_path.join(&entry.id))?;
    let mut file_key = derive_subkey(master_key, entry.id.as_bytes());
    let mut plaintext = decrypt_data(algorithm, &file_key, &ciphertext)?;
    file_key.zeroize();

    let img = image::load_from_memory(&plaintext)
        .map_err(|e| anyhow!("Not an image: {e}"))?;
    plaintext.zeroize();

    let thumb = img.resize(200, 200, FilterType::Triangle);
    let mut png_bytes: Vec<u8> = Vec::new();
    thumb.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )?;

    Ok(general_purpose::STANDARD.encode(&png_bytes))
}

/// Şifre değiştirme: tüm blob'ları yeni key ile yeniden şifreler.
pub fn change_password(
    vault_path: &Path,
    old_key: &[u8; KEY_LEN],
    new_key: &[u8; KEY_LEN],
    new_salt: &[u8; SALT_LEN],
) -> Result<()> {
    let mut meta = VaultMeta::load(vault_path, old_key)?;

    // Tüm dosya blob'larını yeniden şifrele
    let file_ids: Vec<String> = meta
        .entries
        .iter()
        .filter(|e| e.kind == EntryKind::File)
        .map(|e| e.id.clone())
        .collect();

    let algorithm = meta.algorithm.clone();
    for file_id in &file_ids {
        let blob_path = vault_path.join(file_id);
        let ciphertext = std::fs::read(&blob_path)?;
        let mut old_file_key = derive_subkey(old_key, file_id.as_bytes());
        let mut plaintext = decrypt_data(&algorithm, &old_file_key, &ciphertext)?;
        old_file_key.zeroize();

        let mut new_file_key = derive_subkey(new_key, file_id.as_bytes());
        let new_ciphertext = encrypt_data(&algorithm, &new_file_key, &plaintext)?;
        new_file_key.zeroize();
        plaintext.zeroize();

        std::fs::write(&blob_path, new_ciphertext)?;
    }

    // Meta güncelle ve yeni key ile kaydet
    meta.salt = hex::encode(new_salt);
    meta.save(vault_path, new_key)?;
    Ok(())
}
