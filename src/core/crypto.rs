//! Encryption module — AES-256-GCM with Argon2id key derivation

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, Algorithm as Argon2Algorithm, Params, Version};
use rand::RngCore;
use anyhow::Result;

/// Argon2id parameters for key derivation
const ARGON2_MEMORY_KB: u32 = 65536;   // 64 MB
const ARGON2_ITERATIONS: u32 = 3;       // 3 passes
const ARGON2_PARALLELISM: u32 = 4;      // 4 threads
const KEY_LENGTH: usize = 32;           // 256 bits for AES-256
const SALT_LENGTH: usize = 16;          // 128-bit salt
const NONCE_LENGTH: usize = 12;         // 96-bit nonce for GCM

/// Encryption engine holding the derived key
#[derive(Clone)]
pub struct CryptoEngine {
    cipher: Vec<u8>, // raw key bytes (32 bytes)
}

impl CryptoEngine {
    /// Creates a new CryptoEngine by deriving a key from password + salt
    pub fn new(password: &str, salt: &[u8; SALT_LENGTH]) -> Result<Self> {
        let key = derive_key(password, salt)?;
        Ok(Self { cipher: key })
    }

    /// Encrypts data with AES-256-GCM, returns (ciphertext, nonce)
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<(Vec<u8>, [u8; NONCE_LENGTH])> {
        let cipher = Aes256Gcm::new_from_slice(&self.cipher)
            .map_err(|e| anyhow::anyhow!("Failed to create cipher: {}", e))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_LENGTH];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        Ok((ciphertext, nonce_bytes))
    }

    /// Decrypts data with AES-256-GCM using the provided nonce
    pub fn decrypt(&self, ciphertext: &[u8], nonce_bytes: &[u8; NONCE_LENGTH]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(&self.cipher)
            .map_err(|e| anyhow::anyhow!("Failed to create cipher: {}", e))?;

        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| anyhow::anyhow!("Decryption failed — wrong password or corrupted data"))?;

        Ok(plaintext)
    }
}

/// Derives a 256-bit key from a password using Argon2id
fn derive_key(password: &str, salt: &[u8; SALT_LENGTH]) -> Result<Vec<u8>> {
    let params = Params::new(
        ARGON2_MEMORY_KB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(KEY_LENGTH),
    )
    .map_err(|e| anyhow::anyhow!("Invalid Argon2 parameters: {}", e))?;

    let argon2 = Argon2::new(Argon2Algorithm::Argon2id, Version::V0x13, params);

    let mut key = vec![0u8; KEY_LENGTH];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| anyhow::anyhow!("Key derivation failed: {}", e))?;

    Ok(key)
}

/// Generates a random salt for Argon2
pub fn generate_salt() -> [u8; SALT_LENGTH] {
    let mut salt = [0u8; SALT_LENGTH];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    salt
}

/// Verifies that a password can decrypt by attempting decryption of a test block
pub fn verify_password(password: &str, salt: &[u8; SALT_LENGTH], test_ciphertext: &[u8], test_nonce: &[u8; NONCE_LENGTH]) -> bool {
    if let Ok(engine) = CryptoEngine::new(password, salt) {
        engine.decrypt(test_ciphertext, test_nonce).is_ok()
    } else {
        false
    }
}
