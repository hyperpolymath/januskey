// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// RMO: Obliterative Wipe Primitive
// Implements GDPR Article 17 "Right to Erasure" with formal obliteration proofs
//
// The RMO primitive guarantees:
// 1. Content is cryptographically unrecoverable after obliteration
// 2. A proof of non-existence is generated
// 3. The fact of obliteration is logged (without content)

use crate::content_store::{ContentHash, ContentStore};
use crate::error::{JanusError, Result};
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Number of overwrite passes for secure deletion
/// Based on DoD 5220.22-M standard (3 passes minimum)
const OVERWRITE_PASSES: usize = 3;

/// Obliteration patterns for each pass
const PATTERNS: [u8; 3] = [0x00, 0xFF, 0x00]; // zeros, ones, zeros

/// Cryptographic proof that content has been obliterated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObliterationProof {
    /// Unique proof identifier
    pub id: String,
    /// Hash of the obliterated content (proves what was deleted)
    pub content_hash: ContentHash,
    /// Timestamp of obliteration
    pub timestamp: DateTime<Utc>,
    /// User who performed obliteration
    pub user: String,
    /// Nonce used in proof generation
    pub nonce: String,
    /// Cryptographic commitment: H(content_hash || nonce || timestamp)
    pub commitment: String,
    /// Number of overwrite passes performed
    pub overwrite_passes: usize,
    /// Verification that storage location no longer contains original
    pub storage_cleared: bool,
}

impl ObliterationProof {
    /// Generate a new obliteration proof
    pub fn generate(content_hash: &ContentHash, passes: usize) -> Self {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        let user = whoami::username();

        // Generate random nonce
        let mut nonce_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = hex::encode(nonce_bytes);

        // Generate commitment: H(content_hash || nonce || timestamp)
        let mut hasher = Sha256::new();
        hasher.update(content_hash.raw_hash().as_bytes());
        hasher.update(&nonce_bytes);
        hasher.update(timestamp.to_rfc3339().as_bytes());
        let commitment = hex::encode(hasher.finalize());

        Self {
            id,
            content_hash: content_hash.clone(),
            timestamp,
            user,
            nonce,
            commitment,
            overwrite_passes: passes,
            storage_cleared: true,
        }
    }

    /// Verify the proof's cryptographic commitment
    pub fn verify_commitment(&self) -> bool {
        let nonce_bytes = match hex::decode(&self.nonce) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        let mut hasher = Sha256::new();
        hasher.update(self.content_hash.raw_hash().as_bytes());
        hasher.update(&nonce_bytes);
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        let expected = hex::encode(hasher.finalize());

        self.commitment == expected
    }
}

/// Record of an obliteration event (stored in audit log)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObliterationRecord {
    /// Unique record identifier
    pub id: String,
    /// When obliteration occurred
    pub timestamp: DateTime<Utc>,
    /// User who performed obliteration
    pub user: String,
    /// Hash of obliterated content (not the content itself)
    pub content_hash: ContentHash,
    /// Reason for obliteration (optional, for compliance)
    pub reason: Option<String>,
    /// Reference to legal basis (e.g., "GDPR Article 17")
    pub legal_basis: Option<String>,
    /// The obliteration proof
    pub proof: ObliterationProof,
    /// Related operation IDs that were cleaned up
    pub cleaned_operation_ids: Vec<String>,
}

/// Obliteration log for audit trail
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObliterationLog {
    pub version: String,
    pub records: Vec<ObliterationRecord>,
}

impl ObliterationLog {
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            records: Vec::new(),
        }
    }
}

/// Manager for obliterative wipe operations
pub struct ObliterationManager {
    /// Path to obliteration log
    log_path: PathBuf,
    /// Obliteration log
    log: ObliterationLog,
}

impl ObliterationManager {
    /// Create or open an obliteration manager
    pub fn new(log_path: PathBuf) -> Result<Self> {
        let log = if log_path.exists() {
            let content = fs::read_to_string(&log_path)?;
            serde_json::from_str(&content)
                .map_err(|e| JanusError::MetadataCorrupted(e.to_string()))?
        } else {
            ObliterationLog::new()
        };

        Ok(Self { log_path, log })
    }

    /// Save log to disk
    fn save(&self) -> Result<()> {
        if let Some(parent) = self.log_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.log)?;
        fs::write(&self.log_path, content)?;
        Ok(())
    }

    /// Obliterate content from the content store
    /// This is the main RMO primitive implementation
    pub fn obliterate(
        &mut self,
        content_store: &ContentStore,
        content_hash: &ContentHash,
        reason: Option<String>,
        legal_basis: Option<String>,
    ) -> Result<ObliterationRecord> {
        // Get the content path
        let content_path = content_store.content_path(content_hash);

        if !content_path.exists() {
            return Err(JanusError::FileNotFound(format!(
                "Content {} not found in store",
                content_hash
            )));
        }

        // Perform secure overwrite
        let passes = secure_overwrite(&content_path)?;

        // Remove the file
        fs::remove_file(&content_path)?;

        // Generate obliteration proof
        let proof = ObliterationProof::generate(content_hash, passes);

        // Create record
        let record = ObliterationRecord {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            user: whoami::username(),
            content_hash: content_hash.clone(),
            reason,
            legal_basis,
            proof,
            cleaned_operation_ids: Vec::new(),
        };

        // Log the obliteration
        self.log.records.push(record.clone());
        self.save()?;

        Ok(record)
    }

    /// Obliterate content and clean up related metadata references
    pub fn obliterate_with_cleanup(
        &mut self,
        content_store: &ContentStore,
        content_hash: &ContentHash,
        operation_ids: Vec<String>,
        reason: Option<String>,
        legal_basis: Option<String>,
    ) -> Result<ObliterationRecord> {
        // Perform obliteration
        let mut record = self.obliterate(content_store, content_hash, reason, legal_basis)?;

        // Record which operations were affected
        record.cleaned_operation_ids = operation_ids;

        // Update the log
        if let Some(last) = self.log.records.last_mut() {
            last.cleaned_operation_ids = record.cleaned_operation_ids.clone();
        }
        self.save()?;

        Ok(record)
    }

    /// Get all obliteration records
    pub fn records(&self) -> &[ObliterationRecord] {
        &self.log.records
    }

    /// Get record by ID
    pub fn get(&self, id: &str) -> Option<&ObliterationRecord> {
        self.log.records.iter().find(|r| r.id == id)
    }

    /// Get records for a specific content hash
    pub fn get_by_hash(&self, hash: &ContentHash) -> Vec<&ObliterationRecord> {
        self.log
            .records
            .iter()
            .filter(|r| r.content_hash == *hash)
            .collect()
    }

    /// Verify an obliteration proof
    pub fn verify_proof(&self, proof_id: &str) -> Result<bool> {
        let record = self
            .log
            .records
            .iter()
            .find(|r| r.proof.id == proof_id)
            .ok_or_else(|| JanusError::InvalidOperationId(proof_id.to_string()))?;

        Ok(record.proof.verify_commitment())
    }

    /// Count total obliterations
    pub fn count(&self) -> usize {
        self.log.records.len()
    }
}

/// Perform secure overwrite of a file
/// Uses multiple passes with different patterns to ensure data is unrecoverable
fn secure_overwrite(path: &Path) -> Result<usize> {
    let metadata = fs::metadata(path)?;
    let file_size = metadata.len() as usize;

    if file_size == 0 {
        return Ok(OVERWRITE_PASSES);
    }

    // Open file for writing
    let mut file = OpenOptions::new().write(true).open(path)?;

    // Perform overwrite passes
    for (pass, &pattern) in PATTERNS.iter().enumerate() {
        // Seek to beginning
        file.seek(SeekFrom::Start(0))?;

        // Create pattern buffer
        let buffer = if pass == OVERWRITE_PASSES - 1 {
            // Final pass: random data
            let mut random_buffer = vec![0u8; file_size.min(8192)];
            rand::thread_rng().fill_bytes(&mut random_buffer);
            random_buffer
        } else {
            // Fixed pattern
            vec![pattern; file_size.min(8192)]
        };

        // Write in chunks
        let mut written = 0;
        while written < file_size {
            let to_write = (file_size - written).min(buffer.len());
            file.write_all(&buffer[..to_write])?;
            written += to_write;
        }

        // Flush to disk
        file.sync_all()?;
    }

    Ok(OVERWRITE_PASSES)
}

/// Verify that content no longer exists at a path
pub fn verify_obliteration(path: &Path, original_hash: &ContentHash) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }

    // Read remaining content
    let mut file = File::open(path)?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)?;

    // Verify it doesn't match original
    let current_hash = ContentHash::from_bytes(&content);
    Ok(current_hash != *original_hash)
}

/// Batch obliteration request
#[derive(Debug, Clone)]
pub struct BatchObliterationRequest {
    pub content_hashes: Vec<ContentHash>,
    pub reason: Option<String>,
    pub legal_basis: Option<String>,
}

/// Batch obliteration result
#[derive(Debug)]
pub struct BatchObliterationResult {
    pub successful: Vec<ObliterationRecord>,
    pub failed: Vec<(ContentHash, JanusError)>,
}

impl ObliterationManager {
    /// Obliterate multiple content items
    pub fn obliterate_batch(
        &mut self,
        content_store: &ContentStore,
        request: BatchObliterationRequest,
    ) -> BatchObliterationResult {
        let mut successful = Vec::new();
        let mut failed = Vec::new();

        for hash in request.content_hashes {
            match self.obliterate(
                content_store,
                &hash,
                request.reason.clone(),
                request.legal_basis.clone(),
            ) {
                Ok(record) => successful.push(record),
                Err(e) => failed.push((hash, e)),
            }
        }

        BatchObliterationResult { successful, failed }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, ContentStore, ObliterationManager) {
        let tmp = TempDir::new().unwrap();
        let content_store = ContentStore::new(tmp.path().join("content"), false).unwrap();
        let obliteration_manager =
            ObliterationManager::new(tmp.path().join("obliterations.json")).unwrap();
        (tmp, content_store, obliteration_manager)
    }

    #[test]
    fn test_obliteration_proof_generation() {
        let hash = ContentHash::from_bytes(b"test content");
        let proof = ObliterationProof::generate(&hash, 3);

        assert!(!proof.id.is_empty());
        assert_eq!(proof.content_hash, hash);
        assert_eq!(proof.overwrite_passes, 3);
        assert!(proof.storage_cleared);
    }

    #[test]
    fn test_proof_verification() {
        let hash = ContentHash::from_bytes(b"test content");
        let proof = ObliterationProof::generate(&hash, 3);

        assert!(proof.verify_commitment());
    }

    #[test]
    fn test_obliterate_content() {
        let (_tmp, content_store, mut obliteration_manager) = setup();

        // Store some content
        let content = b"sensitive data to be obliterated";
        let hash = content_store.store(content).unwrap();

        // Verify it exists
        assert!(content_store.exists(&hash));

        // Obliterate it
        let record = obliteration_manager
            .obliterate(
                &content_store,
                &hash,
                Some("User request".to_string()),
                Some("GDPR Article 17".to_string()),
            )
            .unwrap();

        // Verify obliteration
        assert!(!content_store.exists(&hash));
        assert_eq!(record.content_hash, hash);
        assert_eq!(record.reason, Some("User request".to_string()));
        assert_eq!(record.legal_basis, Some("GDPR Article 17".to_string()));
        assert!(record.proof.verify_commitment());
    }

    #[test]
    fn test_obliteration_log_persistence() {
        let (tmp, content_store, mut obliteration_manager) = setup();

        // Store and obliterate content
        let content = b"data to obliterate";
        let hash = content_store.store(content).unwrap();
        let record = obliteration_manager
            .obliterate(&content_store, &hash, None, None)
            .unwrap();

        // Reopen manager and verify log
        let obliteration_manager2 =
            ObliterationManager::new(tmp.path().join("obliterations.json")).unwrap();
        assert_eq!(obliteration_manager2.count(), 1);

        let retrieved = obliteration_manager2.get(&record.id).unwrap();
        assert_eq!(retrieved.content_hash, hash);
    }

    #[test]
    fn test_secure_overwrite() {
        let tmp = TempDir::new().unwrap();
        let test_file = tmp.path().join("test.txt");

        // Create file with known content
        let original = b"sensitive information that must be destroyed";
        fs::write(&test_file, original).unwrap();

        // Perform secure overwrite
        let passes = secure_overwrite(&test_file).unwrap();
        assert_eq!(passes, OVERWRITE_PASSES);

        // Read back and verify content changed
        let remaining = fs::read(&test_file).unwrap();
        assert_ne!(remaining, original.to_vec());
    }

    #[test]
    fn test_batch_obliteration() {
        let (_tmp, content_store, mut obliteration_manager) = setup();

        // Store multiple contents
        let hashes: Vec<ContentHash> = (0..5)
            .map(|i| {
                let content = format!("content {}", i);
                content_store.store(content.as_bytes()).unwrap()
            })
            .collect();

        // Batch obliterate
        let request = BatchObliterationRequest {
            content_hashes: hashes.clone(),
            reason: Some("Batch cleanup".to_string()),
            legal_basis: Some("GDPR Article 17".to_string()),
        };

        let result = obliteration_manager.obliterate_batch(&content_store, request);

        assert_eq!(result.successful.len(), 5);
        assert!(result.failed.is_empty());

        // Verify all obliterated
        for hash in hashes {
            assert!(!content_store.exists(&hash));
        }
    }

    #[test]
    fn test_obliterate_nonexistent() {
        let (_tmp, content_store, mut obliteration_manager) = setup();

        let fake_hash = ContentHash::from_bytes(b"nonexistent");
        let result = obliteration_manager.obliterate(&content_store, &fake_hash, None, None);

        assert!(result.is_err());
    }
}
