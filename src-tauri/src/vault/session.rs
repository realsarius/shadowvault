use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Dışarıda açılmış (temp'e decrypt edilmiş) bir dosyanın kaydı.
#[derive(Debug, Clone)]
pub struct OpenFileEntry {
    pub vault_id: String,
    pub entry_id: String,
    /// Kullanıcıya gösterilecek dosya adı
    pub file_name: String,
    /// Kasanın disk üzerindeki klasörü
    pub vault_path: PathBuf,
}

/// Bellekteki açık kasa durumu — key diske asla yazılmaz.
pub struct VaultSession {
    /// vault_id → derived_key (32 byte, zeroize on drop)
    keys: HashMap<String, [u8; 32]>,
    /// tmp_path → açık dosya bilgisi
    open_files: HashMap<PathBuf, OpenFileEntry>,
}

/// Frontend için serialize edilebilir özet.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct OpenFileSummary {
    pub entry_id: String,
    pub file_name: String,
    pub tmp_path: String,
}

impl VaultSession {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            open_files: HashMap::new(),
        }
    }

    /// Kasanın kilidini açar, key'i bellekte saklar.
    pub fn unlock(&mut self, vault_id: &str, key: [u8; 32]) {
        self.keys.insert(vault_id.to_string(), key);
    }

    /// Kasayı kilitler, key bellekten sıfırlanarak silinir.
    /// Açık dosya kaydı silinir ama silme/sync işlemi burada yapılmaz
    /// (caller sorumluluğunda).
    pub fn lock(&mut self, vault_id: &str) {
        if let Some(mut key) = self.keys.remove(vault_id) {
            key.zeroize();
        }
        self.open_files.retain(|_, v| v.vault_id != vault_id);
    }

    /// Key'in kopyasını döner (caller zeroize etmeli).
    pub fn get_key(&self, vault_id: &str) -> Option<[u8; 32]> {
        self.keys.get(vault_id).copied()
    }

    pub fn is_unlocked(&self, vault_id: &str) -> bool {
        self.keys.contains_key(vault_id)
    }

    /// Temp'e decrypt edilen dosyayı kayıt altına al.
    pub fn register_open_file(
        &mut self,
        tmp_path: PathBuf,
        vault_id: &str,
        entry_id: &str,
        file_name: &str,
        vault_path: PathBuf,
    ) {
        self.open_files.insert(
            tmp_path,
            OpenFileEntry {
                vault_id: vault_id.to_string(),
                entry_id: entry_id.to_string(),
                file_name: file_name.to_string(),
                vault_path,
            },
        );
    }

    /// Temp dosyasını kayıttan çıkar.
    pub fn unregister_open_file(&mut self, tmp_path: &PathBuf) {
        self.open_files.remove(tmp_path);
    }

    /// Belirli bir kasa için açık dosyaları döner.
    pub fn get_open_files_for_vault(&self, vault_id: &str) -> Vec<(PathBuf, OpenFileEntry)> {
        self.open_files
            .iter()
            .filter(|(_, e)| e.vault_id == vault_id)
            .map(|(p, e)| (p.clone(), e.clone()))
            .collect()
    }

    /// Tüm açık dosyaları döner (uygulama kapatılırken sync için).
    pub fn get_all_open_files(&self) -> Vec<(PathBuf, OpenFileEntry)> {
        self.open_files
            .iter()
            .map(|(p, e)| (p.clone(), e.clone()))
            .collect()
    }

    /// Uygulama kapanışında tüm key'leri sıfırla.
    pub fn lock_all(&mut self) {
        for key in self.keys.values_mut() {
            key.zeroize();
        }
        self.keys.clear();
        self.open_files.clear();
    }
}

impl Drop for VaultSession {
    fn drop(&mut self) {
        self.lock_all();
    }
}

/// Thread-safe wrapper
pub struct SessionStore(pub Mutex<VaultSession>);

impl SessionStore {
    pub fn new() -> Self {
        Self(Mutex::new(VaultSession::new()))
    }
}
