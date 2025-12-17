// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// Metadata Store: Operation log with complete reverse information
// Implements the formal model from the JanusKey white paper

use crate::content_store::ContentHash;
use crate::error::{JanusError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Operation type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OperationType {
    Delete,
    Modify,
    Move,
    Copy,
    Chmod,
    Chown,
    Create,
    // Extended operations
    Mkdir,
    Rmdir,
    Symlink,
    Append,
    Truncate,
    Touch,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Delete => write!(f, "DELETE"),
            Self::Modify => write!(f, "MODIFY"),
            Self::Move => write!(f, "MOVE"),
            Self::Copy => write!(f, "COPY"),
            Self::Chmod => write!(f, "CHMOD"),
            Self::Chown => write!(f, "CHOWN"),
            Self::Create => write!(f, "CREATE"),
            Self::Mkdir => write!(f, "MKDIR"),
            Self::Rmdir => write!(f, "RMDIR"),
            Self::Symlink => write!(f, "SYMLINK"),
            Self::Append => write!(f, "APPEND"),
            Self::Truncate => write!(f, "TRUNCATE"),
            Self::Touch => write!(f, "TOUCH"),
        }
    }
}

/// File metadata (permissions, timestamps, owner)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Unix permissions (e.g., 0o644)
    pub permissions: u32,
    /// File owner (username or uid)
    pub owner: String,
    /// File group (groupname or gid)
    pub group: String,
    /// Original file size
    pub size: u64,
    /// Last modification time
    pub modified: DateTime<Utc>,
    /// Is this a symbolic link?
    pub is_symlink: bool,
    /// Symlink target if is_symlink
    pub symlink_target: Option<String>,
}

impl FileMetadata {
    /// Capture metadata from a file path
    pub fn from_path(path: &Path) -> Result<Self> {
        let metadata = fs::symlink_metadata(path)?;

        #[cfg(unix)]
        let (permissions, owner, group) = {
            use std::os::unix::fs::MetadataExt;
            (
                metadata.mode(),
                metadata.uid().to_string(),
                metadata.gid().to_string(),
            )
        };

        #[cfg(not(unix))]
        let (permissions, owner, group) = (0o644, "unknown".to_string(), "unknown".to_string());

        let is_symlink = metadata.file_type().is_symlink();
        let symlink_target = if is_symlink {
            fs::read_link(path)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        } else {
            None
        };

        Ok(Self {
            permissions,
            owner,
            group,
            size: metadata.len(),
            modified: DateTime::from(metadata.modified()?),
            is_symlink,
            symlink_target,
        })
    }

    /// Apply metadata to a file
    #[cfg(unix)]
    pub fn apply(&self, path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(self.permissions);
        fs::set_permissions(path, perms)?;
        Ok(())
    }

    #[cfg(not(unix))]
    pub fn apply(&self, _path: &Path) -> Result<()> {
        Ok(())
    }
}

/// Complete metadata for an operation (sufficient for reversal)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetadata {
    /// Unique operation ID
    pub id: String,
    /// Operation type
    pub op_type: OperationType,
    /// When the operation occurred
    pub timestamp: DateTime<Utc>,
    /// User who performed the operation
    pub user: String,
    /// Primary path affected
    pub path: PathBuf,
    /// Secondary path (for move/copy operations)
    pub path_secondary: Option<PathBuf>,
    /// Hash of original content (for delete/modify)
    pub content_hash: Option<ContentHash>,
    /// Hash of new content (for modify)
    pub new_content_hash: Option<ContentHash>,
    /// Original file metadata
    pub original_metadata: Option<FileMetadata>,
    /// New metadata (for chmod/chown)
    pub new_metadata: Option<FileMetadata>,
    /// Transaction ID if part of a transaction
    pub transaction_id: Option<String>,
    /// Whether this operation has been undone
    pub undone: bool,
    /// ID of the undo operation (if undone)
    pub undo_operation_id: Option<String>,
    /// Whether content_hash points to a delta (not full content)
    #[serde(default)]
    pub is_delta: bool,
}

impl OperationMetadata {
    /// Create new operation metadata with generated ID
    pub fn new(op_type: OperationType, path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            op_type,
            timestamp: Utc::now(),
            user: whoami::username(),
            path,
            path_secondary: None,
            content_hash: None,
            new_content_hash: None,
            original_metadata: None,
            new_metadata: None,
            transaction_id: None,
            undone: false,
            undo_operation_id: None,
            is_delta: false,
        }
    }

    /// Builder pattern: set delta flag
    pub fn with_delta(mut self, is_delta: bool) -> Self {
        self.is_delta = is_delta;
        self
    }

    /// Builder pattern: set secondary path
    pub fn with_secondary_path(mut self, path: PathBuf) -> Self {
        self.path_secondary = Some(path);
        self
    }

    /// Builder pattern: set content hash
    pub fn with_content_hash(mut self, hash: ContentHash) -> Self {
        self.content_hash = Some(hash);
        self
    }

    /// Builder pattern: set new content hash
    pub fn with_new_content_hash(mut self, hash: ContentHash) -> Self {
        self.new_content_hash = Some(hash);
        self
    }

    /// Builder pattern: set original metadata
    pub fn with_original_metadata(mut self, metadata: FileMetadata) -> Self {
        self.original_metadata = Some(metadata);
        self
    }

    /// Builder pattern: set transaction ID
    pub fn with_transaction_id(mut self, id: String) -> Self {
        self.transaction_id = Some(id);
        self
    }
}

/// Operation log data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLog {
    /// Version for format compatibility
    pub version: String,
    /// List of all operations (append-only)
    pub operations: Vec<OperationMetadata>,
}

impl Default for OperationLog {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            operations: Vec::new(),
        }
    }
}

/// Metadata store for operation logging
pub struct MetadataStore {
    /// Path to the metadata file
    path: PathBuf,
    /// Cached operation log
    log: OperationLog,
}

impl MetadataStore {
    /// Create or open a metadata store
    pub fn new(path: PathBuf) -> Result<Self> {
        let log = if path.exists() {
            let content = fs::read_to_string(&path)?;
            serde_json::from_str(&content).map_err(|e| JanusError::MetadataCorrupted(e.to_string()))?
        } else {
            OperationLog::default()
        };

        Ok(Self { path, log })
    }

    /// Append an operation to the log
    pub fn append(&mut self, metadata: OperationMetadata) -> Result<()> {
        self.log.operations.push(metadata);
        self.save()
    }

    /// Save the log to disk
    fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.log)?;
        fs::write(&self.path, content)?;
        Ok(())
    }

    /// Get all operations
    pub fn operations(&self) -> &[OperationMetadata] {
        &self.log.operations
    }

    /// Get operation by ID
    pub fn get(&self, id: &str) -> Option<&OperationMetadata> {
        self.log.operations.iter().find(|op| op.id == id)
    }

    /// Get mutable operation by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut OperationMetadata> {
        self.log.operations.iter_mut().find(|op| op.id == id)
    }

    /// Get last N operations (not undone)
    pub fn last_n(&self, n: usize) -> Vec<&OperationMetadata> {
        self.log
            .operations
            .iter()
            .rev()
            .filter(|op| !op.undone)
            .take(n)
            .collect()
    }

    /// Get last undoable operation
    pub fn last_undoable(&self) -> Option<&OperationMetadata> {
        self.log
            .operations
            .iter()
            .rev()
            .find(|op| !op.undone)
    }

    /// Get operations for a transaction
    pub fn transaction_operations(&self, transaction_id: &str) -> Vec<&OperationMetadata> {
        self.log
            .operations
            .iter()
            .filter(|op| op.transaction_id.as_deref() == Some(transaction_id))
            .collect()
    }

    /// Mark operation as undone
    pub fn mark_undone(&mut self, id: &str, undo_op_id: &str) -> Result<()> {
        if let Some(op) = self.get_mut(id) {
            op.undone = true;
            op.undo_operation_id = Some(undo_op_id.to_string());
            self.save()?;
        }
        Ok(())
    }

    /// Filter operations by type
    pub fn filter_by_type(&self, op_type: OperationType) -> Vec<&OperationMetadata> {
        self.log
            .operations
            .iter()
            .filter(|op| op.op_type == op_type)
            .collect()
    }

    /// Filter operations by path pattern
    pub fn filter_by_path(&self, pattern: &str) -> Result<Vec<&OperationMetadata>> {
        let glob_pattern = glob::Pattern::new(pattern)?;
        Ok(self
            .log
            .operations
            .iter()
            .filter(|op| glob_pattern.matches_path(&op.path))
            .collect())
    }

    /// Get operation count
    pub fn count(&self) -> usize {
        self.log.operations.len()
    }

    /// Prune old operations (keep last N)
    pub fn prune(&mut self, keep: usize) -> Result<usize> {
        let original_count = self.log.operations.len();
        if original_count <= keep {
            return Ok(0);
        }

        let to_remove = original_count - keep;
        self.log.operations.drain(0..to_remove);
        self.save()?;
        Ok(to_remove)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_operation_metadata_creation() {
        let meta = OperationMetadata::new(OperationType::Delete, PathBuf::from("/test/file.txt"));
        assert!(!meta.id.is_empty());
        assert_eq!(meta.op_type, OperationType::Delete);
        assert!(!meta.undone);
    }

    #[test]
    fn test_metadata_store() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("metadata.json");

        let mut store = MetadataStore::new(path.clone()).unwrap();

        let meta = OperationMetadata::new(OperationType::Delete, PathBuf::from("/test.txt"));
        let id = meta.id.clone();
        store.append(meta).unwrap();

        assert_eq!(store.count(), 1);
        assert!(store.get(&id).is_some());

        // Reopen and verify persistence
        let store2 = MetadataStore::new(path).unwrap();
        assert_eq!(store2.count(), 1);
        assert!(store2.get(&id).is_some());
    }

    #[test]
    fn test_last_undoable() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("metadata.json");
        let mut store = MetadataStore::new(path).unwrap();

        let meta1 = OperationMetadata::new(OperationType::Delete, PathBuf::from("/a.txt"));
        let meta2 = OperationMetadata::new(OperationType::Delete, PathBuf::from("/b.txt"));

        store.append(meta1).unwrap();
        store.append(meta2).unwrap();

        let last = store.last_undoable().unwrap();
        assert_eq!(last.path, PathBuf::from("/b.txt"));
    }
}
