//! Secrets encryption and decryption
//!
//! Provides AES-GCM encryption for sensitive data like environment secrets.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::{Error, Result};

const NONCE_SIZE: usize = 12;

/// Secrets manager for encrypting and decrypting sensitive data
pub struct SecretsManager {
    cipher: Aes256Gcm,
}

impl SecretsManager {
    /// Create a new secrets manager with the given encryption key
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new(key.into());
        Self { cipher }
    }

    /// Create a secrets manager from a passphrase
    pub fn from_passphrase(passphrase: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(passphrase.as_bytes());
        let key: [u8; 32] = hasher.finalize().into();
        Self::new(&key)
    }

    /// Encrypt a single value
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        // Generate random nonce
        let nonce_bytes: [u8; NONCE_SIZE] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| Error::Encryption(e.to_string()))?;

        // Combine nonce + ciphertext and encode as base64
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(&result))
    }

    /// Decrypt a single value
    pub fn decrypt(&self, encrypted: &str) -> Result<String> {
        // Decode from base64
        let data = BASE64
            .decode(encrypted)
            .map_err(|e| Error::Encryption(format!("Base64 decode error: {}", e)))?;

        if data.len() < NONCE_SIZE {
            return Err(Error::Encryption("Invalid encrypted data".to_string()));
        }

        // Split nonce and ciphertext
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| Error::Encryption(e.to_string()))?;

        String::from_utf8(plaintext)
            .map_err(|e| Error::Encryption(format!("UTF-8 decode error: {}", e)))
    }

    /// Encrypt a map of secrets
    pub fn encrypt_secrets(&self, secrets: &HashMap<String, String>) -> Result<String> {
        let encrypted: Result<HashMap<String, String>> = secrets
            .iter()
            .map(|(k, v)| {
                let encrypted_value = self.encrypt(v)?;
                Ok((k.clone(), encrypted_value))
            })
            .collect();

        let encrypted_map = encrypted?;
        serde_json::to_string(&encrypted_map)
            .map_err(|e| Error::Encryption(format!("JSON serialize error: {}", e)))
    }

    /// Decrypt a map of secrets
    pub fn decrypt_secrets(&self, encrypted_json: &str) -> Result<HashMap<String, String>> {
        let encrypted_map: HashMap<String, String> = serde_json::from_str(encrypted_json)
            .map_err(|e| Error::Encryption(format!("JSON deserialize error: {}", e)))?;

        encrypted_map
            .iter()
            .map(|(k, v)| {
                let decrypted_value = self.decrypt(v)?;
                Ok((k.clone(), decrypted_value))
            })
            .collect()
    }
}

/// Get the default encryption key from environment or use a fallback
pub fn get_encryption_key() -> [u8; 32] {
    if let Ok(key_str) = std::env::var("ORCHESTRATE_ENCRYPTION_KEY") {
        // Try to decode from hex
        if let Ok(key_bytes) = hex::decode(&key_str) {
            if key_bytes.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&key_bytes);
                return key;
            }
        }
        // Otherwise hash it
        let mut hasher = Sha256::new();
        hasher.update(key_str.as_bytes());
        hasher.finalize().into()
    } else {
        // WARNING: This is a fallback for development only!
        // In production, ORCHESTRATE_ENCRYPTION_KEY should be set
        tracing::warn!("ORCHESTRATE_ENCRYPTION_KEY not set, using default key (not secure for production!)");
        let mut hasher = Sha256::new();
        hasher.update(b"orchestrate-default-key-change-in-production");
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let manager = SecretsManager::from_passphrase("test-passphrase");

        let plaintext = "my-secret-value";
        let encrypted = manager.encrypt(plaintext).unwrap();

        // Encrypted should be different from plaintext
        assert_ne!(encrypted, plaintext);

        // Decrypt should give back original
        let decrypted = manager.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_map() {
        let manager = SecretsManager::from_passphrase("test-passphrase");

        let mut secrets = HashMap::new();
        secrets.insert("API_KEY".to_string(), "secret-key-123".to_string());
        secrets.insert("DB_PASSWORD".to_string(), "super-secret".to_string());

        let encrypted_json = manager.encrypt_secrets(&secrets).unwrap();

        // Should not contain plaintext values
        assert!(!encrypted_json.contains("secret-key-123"));
        assert!(!encrypted_json.contains("super-secret"));

        // Decrypt should give back original
        let decrypted = manager.decrypt_secrets(&encrypted_json).unwrap();
        assert_eq!(decrypted.get("API_KEY").unwrap(), "secret-key-123");
        assert_eq!(decrypted.get("DB_PASSWORD").unwrap(), "super-secret");
    }

    #[test]
    fn test_different_nonces() {
        let manager = SecretsManager::from_passphrase("test-passphrase");

        let plaintext = "same-value";
        let encrypted1 = manager.encrypt(plaintext).unwrap();
        let encrypted2 = manager.encrypt(plaintext).unwrap();

        // Same plaintext should produce different ciphertext (due to random nonce)
        assert_ne!(encrypted1, encrypted2);

        // Both should decrypt to same value
        assert_eq!(manager.decrypt(&encrypted1).unwrap(), plaintext);
        assert_eq!(manager.decrypt(&encrypted2).unwrap(), plaintext);
    }

    #[test]
    fn test_invalid_encrypted_data() {
        let manager = SecretsManager::from_passphrase("test-passphrase");

        // Invalid base64
        assert!(manager.decrypt("not-valid-base64!").is_err());

        // Too short
        assert!(manager.decrypt("YWJj").is_err());
    }
}
