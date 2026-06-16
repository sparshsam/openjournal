use std::path::Path;

use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use anyhow::{anyhow, Context};
use argon2::Argon2;
use rand::RngCore;
use sha2::{Digest, Sha256};

const BACKUP_VERSION: u32 = 1;

/// Export an encrypted backup archive.
/// Format: version(4) + salt(16) + nonce(12) + ciphertext
pub fn export_encrypted_backup(
    db_path: &Path,
    output_path: &Path,
    passphrase: &str,
) -> anyhow::Result<(i64, String)> {
    let plaintext = std::fs::read(db_path).context("read database")?;

    // Derive key using argon2id
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), &salt, &mut key)
        .map_err(|e| anyhow!("key derivation failed: {e}"))?;

    // Encrypt with AES-256-GCM
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);

    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_ref())
        .map_err(|e| anyhow!("encryption failed: {e}"))?;

    // Write: version(4) + salt(16) + nonce(12) + ciphertext
    let mut output = Vec::with_capacity(4 + 16 + 12 + ciphertext.len());
    output.extend_from_slice(&BACKUP_VERSION.to_le_bytes());
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);
    std::fs::write(output_path, &output).context("write backup")?;

    let size = std::fs::metadata(output_path)
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    let checksum = format!("{:x}", Sha256::digest(&output));
    Ok((size, checksum))
}

/// Decrypt and restore an encrypted backup archive.
pub fn decrypt_and_restore(
    backup_path: &Path,
    db_path: &Path,
    passphrase: &str,
) -> anyhow::Result<String> {
    let data = std::fs::read(backup_path).context("read backup")?;
    if data.len() < 4 + 16 + 12 {
        return Err(anyhow!("Backup file too short or corrupted"));
    }

    let version = u32::from_le_bytes(data[0..4].try_into().unwrap());
    if version != BACKUP_VERSION {
        return Err(anyhow!("Unsupported backup version {version}"));
    }

    let salt: [u8; 16] = data[4..20].try_into().unwrap();
    let nonce: [u8; 12] = data[20..32].try_into().unwrap();
    let ciphertext = &data[32..];

    // Derive key
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), &salt, &mut key)
        .map_err(|e| anyhow!("key derivation failed: {e}"))?;

    // Decrypt
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext)
        .map_err(|_| anyhow!("Wrong passphrase or corrupted backup"))?;

    // Pre-restore safety backup
    let now = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let pre_path = db_path
        .parent()
        .unwrap()
        .join(format!("pre-restore-{}.sqlite3", now));
    if db_path.exists() {
        std::fs::copy(db_path, &pre_path).ok();
    }

    std::fs::write(db_path, &plaintext).context("write restored database")?;
    let cksum = format!("{:x}", Sha256::digest(&data));
    Ok(format!(
        "Restored. Backup checksum: {}. Pre-restore DB: {}",
        &cksum[..12],
        pre_path.display()
    ))
}
