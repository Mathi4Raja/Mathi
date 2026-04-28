use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::db::RuntimeDatabase;
use crate::types::RuntimeError;

const NONCE_SIZE: usize = 12;

#[derive(Debug, Clone)]
pub struct LocalVault {
    db: RuntimeDatabase,
    key: [u8; 32],
}

impl LocalVault {
    pub fn new_in_memory(passphrase: &str) -> Result<Self, RuntimeError> {
        Ok(Self {
            db: RuntimeDatabase::new_in_memory()?,
            key: derive_key(passphrase),
        })
    }

    pub fn with_database(db: RuntimeDatabase, passphrase: &str) -> Self {
        Self {
            db,
            key: derive_key(passphrase),
        }
    }

    pub fn store_secret(&self, key: &str, plaintext: &str) -> Result<(), RuntimeError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|error| RuntimeError::CryptoFailure(error.to_string()))?;

        let mut nonce_bytes = [0_u8; NONCE_SIZE];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let encrypted = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|error| RuntimeError::CryptoFailure(error.to_string()))?;

        self.db.save_secret(
            key,
            &base64::engine::general_purpose::STANDARD.encode(encrypted),
            &base64::engine::general_purpose::STANDARD.encode(nonce_bytes),
        )
    }

    pub fn load_secret(&self, key: &str) -> Result<String, RuntimeError> {
        let Some((cipher_text, nonce)) = self.db.load_secret(key)? else {
            return Err(RuntimeError::NotFound(format!("secret key {key}")));
        };

        let cipher_bytes = base64::engine::general_purpose::STANDARD
            .decode(cipher_text)
            .map_err(|error| RuntimeError::CryptoFailure(error.to_string()))?;
        let nonce_bytes = base64::engine::general_purpose::STANDARD
            .decode(nonce)
            .map_err(|error| RuntimeError::CryptoFailure(error.to_string()))?;

        if nonce_bytes.len() != NONCE_SIZE {
            return Err(RuntimeError::CryptoFailure("invalid nonce size".to_string()));
        }

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|error| RuntimeError::CryptoFailure(error.to_string()))?;
        let decrypted = cipher
            .decrypt(Nonce::from_slice(&nonce_bytes), cipher_bytes.as_ref())
            .map_err(|error| RuntimeError::CryptoFailure(error.to_string()))?;

        String::from_utf8(decrypted).map_err(|error| RuntimeError::CryptoFailure(error.to_string()))
    }

    pub fn revoke_secret(&self, key: &str) -> Result<(), RuntimeError> {
        self.db.delete_secret(key)
    }
}

fn derive_key(passphrase: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(passphrase.as_bytes());
    let digest = hasher.finalize();
    let mut key = [0_u8; 32];
    key.copy_from_slice(&digest[..32]);
    key
}
