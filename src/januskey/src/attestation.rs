// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// JanusKey Attestation & Audit Log Module
// Tamper-evident logging with cryptographic attestations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::keys::{KeyAlgorithm, KeyPurpose, KeyState};

/// Audit event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEventType {
    /// Key store initialized
    StoreInitialized,
    /// Key store unlocked
    StoreUnlocked,
    /// New key generated
    KeyGenerated,
    /// Key retrieved (decrypted)
    KeyRetrieved,
    /// Key rotated
    KeyRotated,
    /// Key revoked
    KeyRevoked,
    /// Key obliterated
    KeyObliterated,
    /// Backup created
    BackupCreated,
    /// Store restored from backup
    BackupRestored,
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditEventType::StoreInitialized => write!(f, "STORE_INITIALIZED"),
            AuditEventType::StoreUnlocked => write!(f, "STORE_UNLOCKED"),
            AuditEventType::KeyGenerated => write!(f, "KEY_GENERATED"),
            AuditEventType::KeyRetrieved => write!(f, "KEY_RETRIEVED"),
            AuditEventType::KeyRotated => write!(f, "KEY_ROTATED"),
            AuditEventType::KeyRevoked => write!(f, "KEY_REVOKED"),
            AuditEventType::KeyObliterated => write!(f, "KEY_OBLITERATED"),
            AuditEventType::BackupCreated => write!(f, "BACKUP_CREATED"),
            AuditEventType::BackupRestored => write!(f, "BACKUP_RESTORED"),
        }
    }
}

/// Key-specific event details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEventDetails {
    pub key_id: Uuid,
    pub fingerprint: String,
    pub algorithm: Option<KeyAlgorithm>,
    pub purpose: Option<KeyPurpose>,
    pub old_state: Option<KeyState>,
    pub new_state: Option<KeyState>,
    /// For rotation events, the ID of the new key
    pub rotated_to: Option<Uuid>,
    /// For rotation events, the ID of the old key
    pub rotated_from: Option<Uuid>,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique event ID
    pub event_id: Uuid,
    /// Event timestamp (UTC)
    pub timestamp: DateTime<Utc>,
    /// Type of event
    pub event_type: AuditEventType,
    /// Actor (user/system identifier)
    pub actor: String,
    /// Key-specific details (if applicable)
    pub key_details: Option<KeyEventDetails>,
    /// Additional context/reason
    pub reason: Option<String>,
    /// SHA-256 hash of previous entry (chain link)
    pub previous_hash: String,
    /// Attestation: HMAC-SHA256(event_data || previous_hash)
    pub attestation: String,
}

impl AuditEntry {
    /// Compute the hash of this entry for chain linking
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.event_id.as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(self.event_type.to_string().as_bytes());
        hasher.update(self.actor.as_bytes());
        if let Some(ref details) = self.key_details {
            hasher.update(details.key_id.as_bytes());
            hasher.update(details.fingerprint.as_bytes());
        }
        if let Some(ref reason) = self.reason {
            hasher.update(reason.as_bytes());
        }
        hasher.update(self.previous_hash.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Audit log manager
pub struct AuditLog {
    log_path: PathBuf,
    /// Secret for HMAC attestations (derived from store)
    attestation_key: Option<[u8; 32]>,
}

impl AuditLog {
    /// Create audit log manager for a directory
    pub fn new(root: &Path) -> Self {
        let log_path = root.join(".januskey").join("keys").join("audit.log");
        Self {
            log_path,
            attestation_key: None,
        }
    }

    /// Initialize audit log with attestation key
    pub fn init(&mut self, attestation_key: [u8; 32]) -> std::io::Result<()> {
        self.attestation_key = Some(attestation_key);

        // Create log file if it doesn't exist
        if !self.log_path.exists() {
            if let Some(parent) = self.log_path.parent() {
                fs::create_dir_all(parent)?;
            }
            File::create(&self.log_path)?;

            // Set restrictive permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&self.log_path, fs::Permissions::from_mode(0o600))?;
            }
        }

        Ok(())
    }

    /// Set the attestation key (for unlocking existing stores)
    pub fn set_attestation_key(&mut self, key: [u8; 32]) {
        self.attestation_key = Some(key);
    }

    /// Get the last entry's hash (for chain linking)
    fn get_last_hash(&self) -> std::io::Result<String> {
        if !self.log_path.exists() {
            return Ok("0".repeat(64)); // Genesis hash
        }

        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);

        let mut last_entry: Option<AuditEntry> = None;
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<AuditEntry>(&line) {
                last_entry = Some(entry);
            }
        }

        Ok(last_entry
            .map(|e| e.compute_hash())
            .unwrap_or_else(|| "0".repeat(64)))
    }

    /// Compute HMAC-SHA256 attestation
    fn compute_attestation(&self, data: &str, previous_hash: &str) -> String {
        let key = self.attestation_key.unwrap_or([0u8; 32]);

        // Simple HMAC-SHA256: H(key || data || previous_hash)
        let mut hasher = Sha256::new();
        hasher.update(&key);
        hasher.update(data.as_bytes());
        hasher.update(previous_hash.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Log an event
    pub fn log_event(
        &self,
        event_type: AuditEventType,
        key_details: Option<KeyEventDetails>,
        reason: Option<String>,
    ) -> std::io::Result<AuditEntry> {
        let previous_hash = self.get_last_hash()?;
        let actor = get_actor();
        let event_id = Uuid::new_v4();
        let timestamp = Utc::now();

        // Build attestation data
        let attestation_data = format!(
            "{}:{}:{}:{}",
            event_id,
            timestamp.to_rfc3339(),
            event_type,
            actor
        );
        let attestation = self.compute_attestation(&attestation_data, &previous_hash);

        let entry = AuditEntry {
            event_id,
            timestamp,
            event_type,
            actor,
            key_details,
            reason,
            previous_hash,
            attestation,
        };

        // Append to log file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        let json = serde_json::to_string(&entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(file, "{}", json)?;

        Ok(entry)
    }

    /// Log store initialization
    pub fn log_store_init(&self) -> std::io::Result<AuditEntry> {
        self.log_event(AuditEventType::StoreInitialized, None, None)
    }

    /// Log store unlock
    pub fn log_store_unlock(&self) -> std::io::Result<AuditEntry> {
        self.log_event(AuditEventType::StoreUnlocked, None, None)
    }

    /// Log key generation
    pub fn log_key_generated(
        &self,
        key_id: Uuid,
        fingerprint: &str,
        algorithm: KeyAlgorithm,
        purpose: KeyPurpose,
    ) -> std::io::Result<AuditEntry> {
        let details = KeyEventDetails {
            key_id,
            fingerprint: fingerprint.to_string(),
            algorithm: Some(algorithm),
            purpose: Some(purpose),
            old_state: None,
            new_state: Some(KeyState::Active),
            rotated_to: None,
            rotated_from: None,
        };
        self.log_event(AuditEventType::KeyGenerated, Some(details), None)
    }

    /// Log key retrieval
    pub fn log_key_retrieved(&self, key_id: Uuid, fingerprint: &str) -> std::io::Result<AuditEntry> {
        let details = KeyEventDetails {
            key_id,
            fingerprint: fingerprint.to_string(),
            algorithm: None,
            purpose: None,
            old_state: None,
            new_state: None,
            rotated_to: None,
            rotated_from: None,
        };
        self.log_event(AuditEventType::KeyRetrieved, Some(details), None)
    }

    /// Log key rotation
    pub fn log_key_rotated(
        &self,
        old_key_id: Uuid,
        old_fingerprint: &str,
        new_key_id: Uuid,
        new_fingerprint: &str,
    ) -> std::io::Result<AuditEntry> {
        let details = KeyEventDetails {
            key_id: new_key_id,
            fingerprint: new_fingerprint.to_string(),
            algorithm: None,
            purpose: None,
            old_state: Some(KeyState::Active),
            new_state: Some(KeyState::Active),
            rotated_to: None,
            rotated_from: Some(old_key_id),
        };
        let reason = format!(
            "Rotated from key {} ({})",
            old_key_id,
            old_fingerprint
        );
        self.log_event(AuditEventType::KeyRotated, Some(details), Some(reason))
    }

    /// Log key revocation
    pub fn log_key_revoked(
        &self,
        key_id: Uuid,
        fingerprint: &str,
        reason: Option<&str>,
    ) -> std::io::Result<AuditEntry> {
        let details = KeyEventDetails {
            key_id,
            fingerprint: fingerprint.to_string(),
            algorithm: None,
            purpose: None,
            old_state: Some(KeyState::Active),
            new_state: Some(KeyState::Revoked),
            rotated_to: None,
            rotated_from: None,
        };
        self.log_event(
            AuditEventType::KeyRevoked,
            Some(details),
            reason.map(|s| s.to_string()),
        )
    }

    /// Log backup creation
    pub fn log_backup_created(&self, path: &Path) -> std::io::Result<AuditEntry> {
        let reason = format!("Backup created at: {}", path.display());
        self.log_event(AuditEventType::BackupCreated, None, Some(reason))
    }

    /// Read all audit entries
    pub fn read_all(&self) -> std::io::Result<Vec<AuditEntry>> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<AuditEntry>(&line) {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Read last N entries
    pub fn read_last_n(&self, n: usize) -> std::io::Result<Vec<AuditEntry>> {
        let all = self.read_all()?;
        let start = all.len().saturating_sub(n);
        Ok(all[start..].to_vec())
    }

    /// Verify chain integrity
    pub fn verify_integrity(&self) -> std::io::Result<IntegrityReport> {
        let entries = self.read_all()?;

        if entries.is_empty() {
            return Ok(IntegrityReport {
                valid: true,
                total_entries: 0,
                first_invalid_index: None,
                message: "Audit log is empty".to_string(),
            });
        }

        let genesis_hash = "0".repeat(64);
        let mut expected_previous = genesis_hash;

        for (i, entry) in entries.iter().enumerate() {
            // Verify chain link
            if entry.previous_hash != expected_previous {
                return Ok(IntegrityReport {
                    valid: false,
                    total_entries: entries.len(),
                    first_invalid_index: Some(i),
                    message: format!(
                        "Chain broken at entry {}: expected previous_hash {}, got {}",
                        i, expected_previous, entry.previous_hash
                    ),
                });
            }

            // Verify attestation
            let attestation_data = format!(
                "{}:{}:{}:{}",
                entry.event_id,
                entry.timestamp.to_rfc3339(),
                entry.event_type,
                entry.actor
            );
            let expected_attestation =
                self.compute_attestation(&attestation_data, &entry.previous_hash);

            if entry.attestation != expected_attestation {
                return Ok(IntegrityReport {
                    valid: false,
                    total_entries: entries.len(),
                    first_invalid_index: Some(i),
                    message: format!("Invalid attestation at entry {}", i),
                });
            }

            expected_previous = entry.compute_hash();
        }

        Ok(IntegrityReport {
            valid: true,
            total_entries: entries.len(),
            first_invalid_index: None,
            message: format!(
                "Audit log integrity verified: {} entries",
                entries.len()
            ),
        })
    }

    /// Get entries for a specific key
    pub fn get_key_history(&self, key_id: Uuid) -> std::io::Result<Vec<AuditEntry>> {
        let all = self.read_all()?;
        Ok(all
            .into_iter()
            .filter(|e| {
                e.key_details
                    .as_ref()
                    .map(|d| d.key_id == key_id || d.rotated_from == Some(key_id))
                    .unwrap_or(false)
            })
            .collect())
    }

    /// Export audit log to JSON
    pub fn export_json(&self, output: &Path) -> std::io::Result<()> {
        let entries = self.read_all()?;
        let json = serde_json::to_string_pretty(&entries)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(output, json)
    }
}

/// Integrity verification report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub valid: bool,
    pub total_entries: usize,
    pub first_invalid_index: Option<usize>,
    pub message: String,
}

/// Get current actor (user@hostname)
fn get_actor() -> String {
    let user = whoami::username();
    let host = whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string());
    format!("{}@{}", user, host)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_audit_log_init() {
        let tmp = TempDir::new().unwrap();
        let mut log = AuditLog::new(tmp.path());
        log.init([0u8; 32]).unwrap();

        assert!(tmp.path().join(".januskey/keys/audit.log").exists());
    }

    #[test]
    fn test_audit_log_events() {
        let tmp = TempDir::new().unwrap();
        let mut log = AuditLog::new(tmp.path());
        log.init([1u8; 32]).unwrap();

        // Log some events
        log.log_store_init().unwrap();
        log.log_key_generated(
            Uuid::new_v4(),
            "abc123",
            KeyAlgorithm::Aes256Gcm,
            KeyPurpose::Encryption,
        )
        .unwrap();

        let entries = log.read_all().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].event_type, AuditEventType::StoreInitialized);
        assert_eq!(entries[1].event_type, AuditEventType::KeyGenerated);
    }

    #[test]
    fn test_audit_log_chain_integrity() {
        let tmp = TempDir::new().unwrap();
        let mut log = AuditLog::new(tmp.path());
        log.init([2u8; 32]).unwrap();

        log.log_store_init().unwrap();
        log.log_store_unlock().unwrap();
        log.log_key_generated(
            Uuid::new_v4(),
            "def456",
            KeyAlgorithm::Ed25519,
            KeyPurpose::Signing,
        )
        .unwrap();

        let report = log.verify_integrity().unwrap();
        assert!(report.valid);
        assert_eq!(report.total_entries, 3);
    }

    #[test]
    fn test_key_history() {
        let tmp = TempDir::new().unwrap();
        let mut log = AuditLog::new(tmp.path());
        log.init([3u8; 32]).unwrap();

        let key_id = Uuid::new_v4();
        let new_key_id = Uuid::new_v4();

        log.log_key_generated(key_id, "abc", KeyAlgorithm::Aes256Gcm, KeyPurpose::Encryption)
            .unwrap();
        log.log_key_rotated(key_id, "abc", new_key_id, "def")
            .unwrap();
        log.log_key_revoked(key_id, "abc", Some("rotated"))
            .unwrap();

        let history = log.get_key_history(key_id).unwrap();
        assert_eq!(history.len(), 3);
    }
}
