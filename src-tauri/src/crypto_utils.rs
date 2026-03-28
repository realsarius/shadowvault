use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
/// Hardware-ID tabanlı AES-256-GCM yardımcı fonksiyonları.
///
/// Bu modül, cihaza bağlı şifreleme için ortak mantığı tek yerde toplar.
/// Uygulama genelinde cloud config, SFTP/OAuth/WebDAV şifreleri ve lisans
/// anahtarı aynı HW-key mekanizmasını kullanır.
use anyhow::anyhow;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use sysinfo::System;

/// Cihaza özgü ham kimlik dizesi türetir.
/// Format: `shadowvault:<hostname>:<total_memory>:<cpu_count>`
pub fn hw_id_raw() -> String {
    let mut sys = System::new();
    sys.refresh_memory();
    let hostname = System::host_name().unwrap_or_else(|| "unknown-host".to_string());
    format!(
        "shadowvault:{}:{}:{}",
        hostname,
        sys.total_memory(),
        sys.cpus().len()
    )
}

/// HW kimliğinden SHA-256 ile 32 baytlık AES anahtarı türetir.
pub fn hw_aes_key() -> [u8; 32] {
    let raw = hw_id_raw();
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hasher.finalize().into()
}

/// Bir string'i HW anahtarıyla AES-256-GCM ile şifreler.
/// Döndürülen değer: Base64(nonce || ciphertext)
pub fn hw_encrypt(plaintext: &str) -> anyhow::Result<String> {
    let key_bytes = hw_aes_key();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| anyhow!("{}", e))?;
    let mut combined = nonce.to_vec();
    combined.extend(ciphertext);
    Ok(BASE64.encode(combined))
}

/// HW anahtarıyla şifrelenmiş bir Base64 değerini çözer.
/// Hata durumunda `None` döner (yanlış cihaz, bozuk veri vb.).
pub fn hw_decrypt(enc: &str) -> Option<Vec<u8>> {
    use aes_gcm::aead::KeyInit;
    let combined = BASE64.decode(enc).ok()?;
    if combined.len() < 13 {
        return None;
    }
    let key_bytes = hw_aes_key();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&combined[..12]);
    cipher.decrypt(nonce, &combined[12..]).ok()
}

/// Şifrelenmiş bir JSON değerini çözüp T'ye deserialize eder.
pub fn hw_decrypt_json<T: DeserializeOwned>(enc: &str) -> Option<T> {
    let plaintext = hw_decrypt(enc)?;
    let json = String::from_utf8(plaintext).ok()?;
    serde_json::from_str::<T>(&json).ok()
}

/// Şifrelenmiş bir değeri string olarak çözer.
pub fn hw_decrypt_string(enc: &str) -> Option<String> {
    let plaintext = hw_decrypt(enc)?;
    String::from_utf8(plaintext).ok()
}
