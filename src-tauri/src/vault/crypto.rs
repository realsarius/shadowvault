use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use chacha20poly1305::{ChaCha20Poly1305, XChaCha20Poly1305};
use argon2::{Argon2, Params, Version};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroize;
use anyhow::{anyhow, Result};

/// Desteklenen algoritmalar
pub const ALGO_AES_256_GCM: &str = "AES-256-GCM";
pub const ALGO_CHACHA20_POLY1305: &str = "ChaCha20-Poly1305";
pub const ALGO_XCHACHA20_POLY1305: &str = "XChaCha20-Poly1305";

pub const ARGON2_M_COST: u32 = 65536; // 64 MiB
pub const ARGON2_T_COST: u32 = 3;
pub const ARGON2_P_COST: u32 = 4;
pub const SALT_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;    // AES-GCM + ChaCha20
pub const XNONCE_LEN: usize = 24;   // XChaCha20
pub const KEY_LEN: usize = 32;

/// Argon2id ile master key türetme.
pub fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    let params = Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, Some(KEY_LEN))
        .map_err(|e| anyhow!("Argon2 params error: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);

    let mut key = [0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| anyhow!("Argon2 hash error: {e}"))?;
    Ok(key)
}

/// HKDF ile alt anahtar türetme (key separation).
pub fn derive_subkey(master: &[u8; KEY_LEN], info: &[u8]) -> [u8; KEY_LEN] {
    let hk = Hkdf::<Sha256>::new(None, master);
    let mut subkey = [0u8; KEY_LEN];
    hk.expand(info, &mut subkey)
        .expect("HKDF expand failed — info too long");
    subkey
}

// ─── AES-256-GCM ─────────────────────────────────────────────────────────────

/// AES-256-GCM şifreleme. Rastgele nonce üretir; [nonce(12) | ciphertext+tag] döner.
pub fn encrypt(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| anyhow!("AES-GCM encrypt error: {e}"))?;

    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// AES-256-GCM şifre çözme. Giriş: [nonce(12) | ciphertext+tag].
pub fn decrypt(key: &[u8; KEY_LEN], data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < NONCE_LEN {
        return Err(anyhow!("Ciphertext too short"));
    }
    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow!("Decryption failed — wrong password or corrupted data"))?;
    Ok(plaintext)
}

// ─── ChaCha20-Poly1305 ───────────────────────────────────────────────────────

fn encrypt_chacha20(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| anyhow!("ChaCha20 key error: {e}"))?;
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| anyhow!("ChaCha20 encrypt error: {e}"))?;

    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn decrypt_chacha20(key: &[u8; KEY_LEN], data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < NONCE_LEN {
        return Err(anyhow!("Ciphertext too short"));
    }
    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| anyhow!("ChaCha20 key error: {e}"))?;
    let nonce = chacha20poly1305::Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow!("ChaCha20 decryption failed"))
}

// ─── XChaCha20-Poly1305 ──────────────────────────────────────────────────────

fn encrypt_xchacha20(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| anyhow!("XChaCha20 key error: {e}"))?;
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| anyhow!("XChaCha20 encrypt error: {e}"))?;

    let mut out = Vec::with_capacity(XNONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn decrypt_xchacha20(key: &[u8; KEY_LEN], data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < XNONCE_LEN {
        return Err(anyhow!("Ciphertext too short"));
    }
    let (nonce_bytes, ciphertext) = data.split_at(XNONCE_LEN);
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| anyhow!("XChaCha20 key error: {e}"))?;
    let nonce = chacha20poly1305::XNonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow!("XChaCha20 decryption failed"))
}

// ─── Birleşik dispatch ───────────────────────────────────────────────────────

/// Algoritmaya göre doğru şifrelemeyi seçer.
pub fn encrypt_data(algorithm: &str, key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<Vec<u8>> {
    match algorithm {
        ALGO_CHACHA20_POLY1305 => encrypt_chacha20(key, plaintext),
        ALGO_XCHACHA20_POLY1305 => encrypt_xchacha20(key, plaintext),
        _ => encrypt(key, plaintext), // AES-256-GCM default
    }
}

/// Algoritmaya göre doğru şifre çözmeyi seçer.
pub fn decrypt_data(algorithm: &str, key: &[u8; KEY_LEN], data: &[u8]) -> Result<Vec<u8>> {
    match algorithm {
        ALGO_CHACHA20_POLY1305 => decrypt_chacha20(key, data),
        ALGO_XCHACHA20_POLY1305 => decrypt_xchacha20(key, data),
        _ => decrypt(key, data), // AES-256-GCM default
    }
}

/// Rastgele salt üret.
pub fn generate_salt() -> [u8; SALT_LEN] {
    use rand::RngCore;
    let mut salt = [0u8; SALT_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    salt
}

pub fn zeroize_key(key: &mut [u8; KEY_LEN]) {
    key.zeroize();
}
