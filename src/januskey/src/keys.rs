// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// JanusKey Key Management Module
// Implements secure key generation, storage, rotation, and recovery

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm as Argon2Algorithm, Argon2, Params, Version};
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::attestation::AuditLog;

/// Key management errors
#[derive(Error, Debug)]
pub enum KeyError {
    #[error("Key store not initialized")]
    NotInitialized,

    #[error("Key store already exists")]
    AlreadyExists,

    #[error("Invalid passphrase")]
    InvalidPassphrase,

    #[error("Key not found: {0}")]
    KeyNotFound(Uuid),

    #[error("Key already revoked: {0}")]
    AlreadyRevoked(Uuid),

    #[error("Cryptographic error: {0}")]
    CryptoError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, KeyError>;

/// Argon2id parameters (OWASP recommendations)
const ARGON2_MEMORY_KB: u32 = 65536; // 64 MB
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_PARALLELISM: u32 = 4;
const SALT_LENGTH: usize = 16;
const NONCE_LENGTH: usize = 12;
const KEY_LENGTH: usize = 32;

/// Key algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyAlgorithm {
    Aes256Gcm,
    Ed25519,
    X25519,
}

impl std::fmt::Display for KeyAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyAlgorithm::Aes256Gcm => write!(f, "AES-256-GCM"),
            KeyAlgorithm::Ed25519 => write!(f, "Ed25519"),
            KeyAlgorithm::X25519 => write!(f, "X25519"),
        }
    }
}

/// Key purpose
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyPurpose {
    Encryption,
    Signing,
    KeyWrap,
    Recovery,
}

impl std::fmt::Display for KeyPurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyPurpose::Encryption => write!(f, "encryption"),
            KeyPurpose::Signing => write!(f, "signing"),
            KeyPurpose::KeyWrap => write!(f, "key-wrap"),
            KeyPurpose::Recovery => write!(f, "recovery"),
        }
    }
}

/// Key lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyState {
    Generated,
    Active,
    Rotating,
    Suspended,
    Revoked,
    Obliterated,
}

impl std::fmt::Display for KeyState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyState::Generated => write!(f, "generated"),
            KeyState::Active => write!(f, "active"),
            KeyState::Rotating => write!(f, "rotating"),
            KeyState::Suspended => write!(f, "suspended"),
            KeyState::Revoked => write!(f, "revoked"),
            KeyState::Obliterated => write!(f, "obliterated"),
        }
    }
}

/// Key metadata (stored with wrapped key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    pub id: Uuid,
    pub algorithm: KeyAlgorithm,
    pub purpose: KeyPurpose,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub state: KeyState,
    pub rotation_of: Option<Uuid>,
    pub fingerprint: String,
    pub description: Option<String>,
}

/// Wrapped key (encrypted key material + metadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrappedKey {
    pub metadata: KeyMetadata,
    pub nonce: [u8; NONCE_LENGTH],
    pub ciphertext: Vec<u8>,
}

/// Key store header
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeyStoreHeader {
    magic: String,
    version: u32,
    salt: [u8; SALT_LENGTH],
    nonce: [u8; NONCE_LENGTH],
}

/// Key store (encrypted container for keys)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeyStoreData {
    header: KeyStoreHeader,
    keys: Vec<WrappedKey>,
}

/// Secret key material (zeroized on drop)
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretKey {
    bytes: [u8; KEY_LENGTH],
}

impl SecretKey {
    pub fn new(bytes: [u8; KEY_LENGTH]) -> Self {
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8; KEY_LENGTH] {
        &self.bytes
    }

    pub fn generate() -> Result<Self> {
        let mut bytes = [0u8; KEY_LENGTH];
        rand::thread_rng().fill_bytes(&mut bytes);
        Ok(Self { bytes })
    }
}

/// Key manager for JanusKey
pub struct KeyManager {
    store_path: PathBuf,
    root_path: PathBuf,
    kek: Option<SecretKey>,
    audit_log: AuditLog,
}

impl KeyManager {
    /// Create new key manager for a directory
    pub fn new(root: &Path) -> Self {
        let store_path = root.join(".januskey").join("keys");
        let audit_log = AuditLog::new(root);
        Self {
            store_path,
            root_path: root.to_path_buf(),
            kek: None,
            audit_log,
        }
    }

    /// Get reference to audit log
    pub fn audit_log(&self) -> &AuditLog {
        &self.audit_log
    }

    /// Check if key store is initialized
    pub fn is_initialized(&self) -> bool {
        self.store_path.join("keystore.jks").exists()
    }

    /// Initialize key store with passphrase
    pub fn init(&mut self, passphrase: &str) -> Result<()> {
        if self.is_initialized() {
            return Err(KeyError::AlreadyExists);
        }

        fs::create_dir_all(&self.store_path)?;

        // Generate salt
        let mut salt = [0u8; SALT_LENGTH];
        rand::thread_rng().fill_bytes(&mut salt);

        // Derive KEK from passphrase
        let kek = derive_kek(passphrase, &salt)?;

        // Derive attestation key from KEK
        let mut attestation_key = [0u8; 32];
        let mut hasher = Sha256::new();
        hasher.update(kek.as_bytes());
        hasher.update(b"attestation");
        attestation_key.copy_from_slice(&hasher.finalize());

        self.kek = Some(kek);

        // Generate initial nonce
        let mut nonce = [0u8; NONCE_LENGTH];
        rand::thread_rng().fill_bytes(&mut nonce);

        // Create empty key store
        let store = KeyStoreData {
            header: KeyStoreHeader {
                magic: "JKKEYS01".to_string(),
                version: 1,
                salt,
                nonce,
            },
            keys: Vec::new(),
        };

        self.save_store(&store)?;

        // Set restrictive permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = self.store_path.join("keystore.jks");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }

        // Initialize audit log and record event
        self.audit_log.init(attestation_key)?;
        let _ = self.audit_log.log_store_init();

        Ok(())
    }

    /// Unlock key store with passphrase
    pub fn unlock(&mut self, passphrase: &str) -> Result<()> {
        if !self.is_initialized() {
            return Err(KeyError::NotInitialized);
        }

        let store = self.load_store_raw()?;
        let kek = derive_kek(passphrase, &store.header.salt)?;

        // Verify passphrase by attempting to decrypt store
        if !self.verify_kek(&kek, &store)? {
            return Err(KeyError::InvalidPassphrase);
        }

        // Derive attestation key from KEK
        let mut attestation_key = [0u8; 32];
        let mut hasher = Sha256::new();
        hasher.update(kek.as_bytes());
        hasher.update(b"attestation");
        attestation_key.copy_from_slice(&hasher.finalize());

        self.kek = Some(kek);
        self.audit_log.set_attestation_key(attestation_key);
        let _ = self.audit_log.log_store_unlock();

        Ok(())
    }

    /// Generate a new key
    pub fn generate(
        &mut self,
        algorithm: KeyAlgorithm,
        purpose: KeyPurpose,
        description: Option<String>,
        expires_in_days: Option<u64>,
    ) -> Result<Uuid> {
        let kek = self.kek.as_ref().ok_or(KeyError::NotInitialized)?;
        let mut store = self.load_store()?;

        // Generate key material
        let key = SecretKey::generate()?;

        // Calculate fingerprint
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let fingerprint = hex::encode(&hasher.finalize()[..8]);

        // Create metadata
        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = expires_in_days.map(|days| now + chrono::Duration::days(days as i64));

        let metadata = KeyMetadata {
            id,
            algorithm,
            purpose,
            created_at: now,
            expires_at,
            state: KeyState::Active,
            rotation_of: None,
            fingerprint: fingerprint.clone(),
            description,
        };

        // Wrap key
        let wrapped = wrap_key(kek, key.as_bytes(), &metadata)?;
        store.keys.push(wrapped);

        self.save_store(&store)?;

        // Log key generation
        let _ = self.audit_log.log_key_generated(id, &fingerprint, algorithm, purpose);

        Ok(id)
    }

    /// List all keys
    pub fn list(&self) -> Result<Vec<KeyMetadata>> {
        if self.kek.is_none() {
            return Err(KeyError::NotInitialized);
        }

        let store = self.load_store()?;
        Ok(store.keys.into_iter().map(|k| k.metadata).collect())
    }

    /// Get key metadata by ID
    pub fn get(&self, id: Uuid) -> Result<KeyMetadata> {
        if self.kek.is_none() {
            return Err(KeyError::NotInitialized);
        }

        let store = self.load_store()?;
        store
            .keys
            .into_iter()
            .find(|k| k.metadata.id == id)
            .map(|k| k.metadata)
            .ok_or(KeyError::KeyNotFound(id))
    }

    /// Retrieve key material (use carefully!)
    pub fn retrieve(&self, id: Uuid) -> Result<SecretKey> {
        let kek = self.kek.as_ref().ok_or(KeyError::NotInitialized)?;
        let store = self.load_store()?;

        let wrapped = store
            .keys
            .into_iter()
            .find(|k| k.metadata.id == id)
            .ok_or(KeyError::KeyNotFound(id))?;

        if wrapped.metadata.state == KeyState::Revoked
            || wrapped.metadata.state == KeyState::Obliterated
        {
            return Err(KeyError::AlreadyRevoked(id));
        }

        // Log key retrieval
        let _ = self.audit_log.log_key_retrieved(id, &wrapped.metadata.fingerprint);

        unwrap_key(kek, &wrapped)
    }

    /// Rotate a key
    pub fn rotate(&mut self, id: Uuid) -> Result<Uuid> {
        let kek = self.kek.as_ref().ok_or(KeyError::NotInitialized)?;
        let mut store = self.load_store()?;

        // Find old key
        let old_idx = store
            .keys
            .iter()
            .position(|k| k.metadata.id == id)
            .ok_or(KeyError::KeyNotFound(id))?;

        if store.keys[old_idx].metadata.state == KeyState::Revoked {
            return Err(KeyError::AlreadyRevoked(id));
        }

        // Generate new key with same properties
        let old_meta = &store.keys[old_idx].metadata;
        let new_key = SecretKey::generate()?;

        let mut hasher = Sha256::new();
        hasher.update(new_key.as_bytes());
        let fingerprint = hex::encode(&hasher.finalize()[..8]);

        let new_id = Uuid::new_v4();
        let now = Utc::now();

        let new_metadata = KeyMetadata {
            id: new_id,
            algorithm: old_meta.algorithm,
            purpose: old_meta.purpose,
            created_at: now,
            expires_at: old_meta
                .expires_at
                .map(|_| now + chrono::Duration::days(365)),
            state: KeyState::Active,
            rotation_of: Some(id),
            fingerprint: fingerprint.clone(),
            description: old_meta.description.clone(),
        };

        // Wrap new key
        let new_wrapped = wrap_key(kek, new_key.as_bytes(), &new_metadata)?;
        store.keys.push(new_wrapped);

        // Mark old key as revoked
        let old_fingerprint = store.keys[old_idx].metadata.fingerprint.clone();
        store.keys[old_idx].metadata.state = KeyState::Revoked;

        self.save_store(&store)?;

        // Log rotation event
        let _ = self.audit_log.log_key_rotated(id, &old_fingerprint, new_id, &fingerprint);

        Ok(new_id)
    }

    /// Revoke a key
    pub fn revoke(&mut self, id: Uuid) -> Result<()> {
        if self.kek.is_none() {
            return Err(KeyError::NotInitialized);
        }

        let mut store = self.load_store()?;

        let key = store
            .keys
            .iter_mut()
            .find(|k| k.metadata.id == id)
            .ok_or(KeyError::KeyNotFound(id))?;

        if key.metadata.state == KeyState::Revoked {
            return Err(KeyError::AlreadyRevoked(id));
        }

        let fingerprint = key.metadata.fingerprint.clone();
        key.metadata.state = KeyState::Revoked;
        self.save_store(&store)?;

        // Log revocation
        let _ = self.audit_log.log_key_revoked(id, &fingerprint, None);

        Ok(())
    }

    /// Revoke a key with reason
    pub fn revoke_with_reason(&mut self, id: Uuid, reason: &str) -> Result<()> {
        if self.kek.is_none() {
            return Err(KeyError::NotInitialized);
        }

        let mut store = self.load_store()?;

        let key = store
            .keys
            .iter_mut()
            .find(|k| k.metadata.id == id)
            .ok_or(KeyError::KeyNotFound(id))?;

        if key.metadata.state == KeyState::Revoked {
            return Err(KeyError::AlreadyRevoked(id));
        }

        let fingerprint = key.metadata.fingerprint.clone();
        key.metadata.state = KeyState::Revoked;
        self.save_store(&store)?;

        // Log revocation with reason
        let _ = self.audit_log.log_key_revoked(id, &fingerprint, Some(reason));

        Ok(())
    }

    /// Create encrypted backup
    pub fn backup(&self, output: &Path) -> Result<()> {
        if self.kek.is_none() {
            return Err(KeyError::NotInitialized);
        }

        let store_path = self.store_path.join("keystore.jks");
        fs::copy(store_path, output)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(output, fs::Permissions::from_mode(0o600))?;
        }

        // Log backup creation
        let _ = self.audit_log.log_backup_created(output);

        Ok(())
    }

    // Internal helpers

    fn load_store_raw(&self) -> Result<KeyStoreData> {
        let path = self.store_path.join("keystore.jks");
        let content = fs::read_to_string(&path)?;
        let store: KeyStoreData = serde_json::from_str(&content)?;
        Ok(store)
    }

    fn load_store(&self) -> Result<KeyStoreData> {
        self.load_store_raw()
    }

    fn save_store(&self, store: &KeyStoreData) -> Result<()> {
        let path = self.store_path.join("keystore.jks");
        let content = serde_json::to_string_pretty(store)?;
        fs::write(&path, content)?;
        Ok(())
    }

    fn verify_kek(&self, kek: &SecretKey, store: &KeyStoreData) -> Result<bool> {
        // If there are any keys, try to unwrap the first one
        if let Some(wrapped) = store.keys.first() {
            match unwrap_key(kek, wrapped) {
                Ok(_) => Ok(true),
                Err(KeyError::CryptoError(_)) => Ok(false),
                Err(e) => Err(e),
            }
        } else {
            // No keys yet, verify by re-deriving and checking magic
            Ok(store.header.magic == "JKKEYS01")
        }
    }
}

/// Derive Key Encryption Key from passphrase
fn derive_kek(passphrase: &str, salt: &[u8; SALT_LENGTH]) -> Result<SecretKey> {
    let params = Params::new(
        ARGON2_MEMORY_KB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(KEY_LENGTH),
    )
    .map_err(|e| KeyError::CryptoError(e.to_string()))?;

    let argon2 = Argon2::new(Argon2Algorithm::Argon2id, Version::V0x13, params);

    let mut kek = [0u8; KEY_LENGTH];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut kek)
        .map_err(|e| KeyError::CryptoError(e.to_string()))?;

    Ok(SecretKey::new(kek))
}

/// Wrap (encrypt) key material
fn wrap_key(kek: &SecretKey, key: &[u8], metadata: &KeyMetadata) -> Result<WrappedKey> {
    let mut nonce_bytes = [0u8; NONCE_LENGTH];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    let cipher = Aes256Gcm::new(kek.as_bytes().into());
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Use metadata ID as additional authenticated data
    let aad = metadata.id.as_bytes();

    let ciphertext = cipher
        .encrypt(nonce, aes_gcm::aead::Payload { msg: key, aad })
        .map_err(|e| KeyError::CryptoError(e.to_string()))?;

    Ok(WrappedKey {
        metadata: metadata.clone(),
        nonce: nonce_bytes,
        ciphertext,
    })
}

/// Unwrap (decrypt) key material
fn unwrap_key(kek: &SecretKey, wrapped: &WrappedKey) -> Result<SecretKey> {
    let cipher = Aes256Gcm::new(kek.as_bytes().into());
    let nonce = Nonce::from_slice(&wrapped.nonce);

    let aad = wrapped.metadata.id.as_bytes();

    let plaintext = cipher
        .decrypt(
            nonce,
            aes_gcm::aead::Payload {
                msg: &wrapped.ciphertext,
                aad,
            },
        )
        .map_err(|_| KeyError::CryptoError("Decryption failed".to_string()))?;

    if plaintext.len() != KEY_LENGTH {
        return Err(KeyError::CryptoError("Invalid key length".to_string()));
    }

    let mut bytes = [0u8; KEY_LENGTH];
    bytes.copy_from_slice(&plaintext);
    Ok(SecretKey::new(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_key_manager_init() {
        let tmp = TempDir::new().unwrap();
        let mut km = KeyManager::new(tmp.path());

        assert!(!km.is_initialized());
        km.init("test-passphrase").unwrap();
        assert!(km.is_initialized());
    }

    #[test]
    fn test_key_generation_and_retrieval() {
        let tmp = TempDir::new().unwrap();
        let mut km = KeyManager::new(tmp.path());

        km.init("test-passphrase").unwrap();

        let id = km
            .generate(
                KeyAlgorithm::Aes256Gcm,
                KeyPurpose::Encryption,
                Some("Test key".to_string()),
                None,
            )
            .unwrap();

        let meta = km.get(id).unwrap();
        assert_eq!(meta.id, id);
        assert_eq!(meta.algorithm, KeyAlgorithm::Aes256Gcm);
        assert_eq!(meta.purpose, KeyPurpose::Encryption);
        assert_eq!(meta.state, KeyState::Active);

        let key = km.retrieve(id).unwrap();
        assert_eq!(key.as_bytes().len(), 32);
    }

    #[test]
    fn test_key_rotation() {
        let tmp = TempDir::new().unwrap();
        let mut km = KeyManager::new(tmp.path());

        km.init("test-passphrase").unwrap();

        let old_id = km
            .generate(KeyAlgorithm::Aes256Gcm, KeyPurpose::Encryption, None, None)
            .unwrap();

        let new_id = km.rotate(old_id).unwrap();

        let old_meta = km.get(old_id).unwrap();
        let new_meta = km.get(new_id).unwrap();

        assert_eq!(old_meta.state, KeyState::Revoked);
        assert_eq!(new_meta.state, KeyState::Active);
        assert_eq!(new_meta.rotation_of, Some(old_id));
    }

    #[test]
    fn test_wrong_passphrase() {
        let tmp = TempDir::new().unwrap();
        let mut km = KeyManager::new(tmp.path());

        km.init("correct-passphrase").unwrap();

        // Generate a key so we have something to verify against
        km.generate(KeyAlgorithm::Aes256Gcm, KeyPurpose::Encryption, None, None)
            .unwrap();

        // Re-open with wrong passphrase
        let mut km2 = KeyManager::new(tmp.path());
        let result = km2.unlock("wrong-passphrase");
        assert!(matches!(result, Err(KeyError::InvalidPassphrase)));
    }
}
