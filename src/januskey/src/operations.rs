// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// Reversible File Operations
// Each operation stores sufficient metadata for perfect inversion

use crate::content_store::{ContentHash, ContentStore};
use crate::error::{JanusError, Result};
use crate::metadata::{FileMetadata, MetadataStore, OperationMetadata, OperationType};
use std::fs;
use std::path::{Path, PathBuf};

/// A file operation that can be executed and reversed
#[derive(Debug, Clone)]
pub enum FileOperation {
    /// Delete a file (reversible: restore from stored content)
    Delete {
        path: PathBuf,
    },
    /// Modify a file (reversible: restore original content)
    Modify {
        path: PathBuf,
        new_content: Vec<u8>,
    },
    /// Move/rename a file (reversible: move back)
    Move {
        source: PathBuf,
        destination: PathBuf,
    },
    /// Copy a file (reversible: delete the copy)
    Copy {
        source: PathBuf,
        destination: PathBuf,
    },
    /// Change permissions (reversible: restore original perms)
    #[cfg(unix)]
    Chmod {
        path: PathBuf,
        new_mode: u32,
    },
    /// Create a new file (reversible: delete)
    Create {
        path: PathBuf,
        content: Vec<u8>,
    },
}

impl FileOperation {
    /// Get operation type
    pub fn op_type(&self) -> OperationType {
        match self {
            Self::Delete { .. } => OperationType::Delete,
            Self::Modify { .. } => OperationType::Modify,
            Self::Move { .. } => OperationType::Move,
            Self::Copy { .. } => OperationType::Copy,
            #[cfg(unix)]
            Self::Chmod { .. } => OperationType::Chmod,
            Self::Create { .. } => OperationType::Create,
        }
    }

    /// Get primary path
    pub fn path(&self) -> &Path {
        match self {
            Self::Delete { path } => path,
            Self::Modify { path, .. } => path,
            Self::Move { source, .. } => source,
            Self::Copy { source, .. } => source,
            #[cfg(unix)]
            Self::Chmod { path, .. } => path,
            Self::Create { path, .. } => path,
        }
    }
}

/// Executor for file operations with reversibility support
pub struct OperationExecutor<'a> {
    content_store: &'a ContentStore,
    metadata_store: &'a mut MetadataStore,
    transaction_id: Option<String>,
}

impl<'a> OperationExecutor<'a> {
    pub fn new(
        content_store: &'a ContentStore,
        metadata_store: &'a mut MetadataStore,
    ) -> Self {
        Self {
            content_store,
            metadata_store,
            transaction_id: None,
        }
    }

    pub fn with_transaction(mut self, transaction_id: String) -> Self {
        self.transaction_id = Some(transaction_id);
        self
    }

    /// Execute an operation and record metadata for reversal
    pub fn execute(&mut self, operation: FileOperation) -> Result<OperationMetadata> {
        match operation {
            FileOperation::Delete { path } => self.execute_delete(&path),
            FileOperation::Modify { path, new_content } => {
                self.execute_modify(&path, &new_content)
            }
            FileOperation::Move { source, destination } => {
                self.execute_move(&source, &destination)
            }
            FileOperation::Copy { source, destination } => {
                self.execute_copy(&source, &destination)
            }
            #[cfg(unix)]
            FileOperation::Chmod { path, new_mode } => self.execute_chmod(&path, new_mode),
            FileOperation::Create { path, content } => self.execute_create(&path, &content),
        }
    }

    /// Execute delete operation
    fn execute_delete(&mut self, path: &Path) -> Result<OperationMetadata> {
        if !path.exists() {
            return Err(JanusError::FileNotFound(path.display().to_string()));
        }

        // Capture original content and metadata
        let content = fs::read(path)?;
        let file_metadata = FileMetadata::from_path(path)?;
        let content_hash = self.content_store.store(&content)?;

        // Create operation metadata
        let mut metadata = OperationMetadata::new(OperationType::Delete, path.to_path_buf())
            .with_content_hash(content_hash)
            .with_original_metadata(file_metadata);

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Perform the delete
        fs::remove_file(path)?;

        // Record and return
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
    }

    /// Execute modify operation
    fn execute_modify(&mut self, path: &Path, new_content: &[u8]) -> Result<OperationMetadata> {
        if !path.exists() {
            return Err(JanusError::FileNotFound(path.display().to_string()));
        }

        // Capture original content
        let original_content = fs::read(path)?;
        let file_metadata = FileMetadata::from_path(path)?;
        let original_hash = self.content_store.store(&original_content)?;
        let new_hash = ContentHash::from_bytes(new_content);

        // Create operation metadata
        let mut metadata = OperationMetadata::new(OperationType::Modify, path.to_path_buf())
            .with_content_hash(original_hash)
            .with_new_content_hash(new_hash)
            .with_original_metadata(file_metadata);

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Perform the modify
        fs::write(path, new_content)?;

        // Record and return
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
    }

    /// Execute move operation
    fn execute_move(&mut self, source: &Path, destination: &Path) -> Result<OperationMetadata> {
        if !source.exists() {
            return Err(JanusError::FileNotFound(source.display().to_string()));
        }
        if destination.exists() {
            return Err(JanusError::PathExists(destination.display().to_string()));
        }

        // Create parent directory if needed
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        // Capture metadata
        let file_metadata = FileMetadata::from_path(source)?;

        // Create operation metadata
        let mut metadata = OperationMetadata::new(OperationType::Move, source.to_path_buf())
            .with_secondary_path(destination.to_path_buf())
            .with_original_metadata(file_metadata);

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Perform the move
        fs::rename(source, destination)?;

        // Record and return
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
    }

    /// Execute copy operation
    fn execute_copy(&mut self, source: &Path, destination: &Path) -> Result<OperationMetadata> {
        if !source.exists() {
            return Err(JanusError::FileNotFound(source.display().to_string()));
        }
        if destination.exists() {
            return Err(JanusError::PathExists(destination.display().to_string()));
        }

        // Create parent directory if needed
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create operation metadata
        let mut metadata = OperationMetadata::new(OperationType::Copy, source.to_path_buf())
            .with_secondary_path(destination.to_path_buf());

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Perform the copy
        fs::copy(source, destination)?;

        // Record and return
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
    }

    /// Execute chmod operation
    #[cfg(unix)]
    fn execute_chmod(&mut self, path: &Path, new_mode: u32) -> Result<OperationMetadata> {
        use std::os::unix::fs::PermissionsExt;

        if !path.exists() {
            return Err(JanusError::FileNotFound(path.display().to_string()));
        }

        // Capture original metadata
        let file_metadata = FileMetadata::from_path(path)?;

        // Create new metadata with new permissions
        let mut new_metadata = file_metadata.clone();
        new_metadata.permissions = new_mode;

        // Create operation metadata
        let mut metadata = OperationMetadata::new(OperationType::Chmod, path.to_path_buf())
            .with_original_metadata(file_metadata);
        metadata.new_metadata = Some(new_metadata);

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Perform the chmod
        let perms = fs::Permissions::from_mode(new_mode);
        fs::set_permissions(path, perms)?;

        // Record and return
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
    }

    /// Execute create operation
    fn execute_create(&mut self, path: &Path, content: &[u8]) -> Result<OperationMetadata> {
        if path.exists() {
            return Err(JanusError::PathExists(path.display().to_string()));
        }

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create operation metadata
        let content_hash = ContentHash::from_bytes(content);
        let mut metadata = OperationMetadata::new(OperationType::Create, path.to_path_buf())
            .with_new_content_hash(content_hash);

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Perform the create
        fs::write(path, content)?;

        // Record and return
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
    }

    /// Undo an operation using its metadata
    pub fn undo(&mut self, operation_id: &str) -> Result<OperationMetadata> {
        let original_op = self
            .metadata_store
            .get(operation_id)
            .ok_or_else(|| JanusError::InvalidOperationId(operation_id.to_string()))?
            .clone();

        if original_op.undone {
            return Err(JanusError::OperationFailed(format!(
                "Operation {} already undone",
                operation_id
            )));
        }

        let undo_metadata = match original_op.op_type {
            OperationType::Delete => self.undo_delete(&original_op)?,
            OperationType::Modify => self.undo_modify(&original_op)?,
            OperationType::Move => self.undo_move(&original_op)?,
            OperationType::Copy => self.undo_copy(&original_op)?,
            OperationType::Chmod => {
                #[cfg(unix)]
                {
                    self.undo_chmod(&original_op)?
                }
                #[cfg(not(unix))]
                {
                    return Err(JanusError::OperationFailed(
                        "Chmod not supported on this platform".to_string(),
                    ));
                }
            }
            OperationType::Create => self.undo_create(&original_op)?,
            OperationType::Chown => {
                return Err(JanusError::OperationFailed(
                    "Chown undo not yet implemented".to_string(),
                ))
            }
        };

        // Mark original operation as undone
        self.metadata_store.mark_undone(operation_id, &undo_metadata.id)?;

        Ok(undo_metadata)
    }

    /// Undo delete: restore file from content store
    fn undo_delete(&mut self, original: &OperationMetadata) -> Result<OperationMetadata> {
        let content_hash = original
            .content_hash
            .as_ref()
            .ok_or_else(|| JanusError::MetadataCorrupted("Missing content hash".to_string()))?;

        // Retrieve original content
        let content = self.content_store.retrieve(content_hash)?;

        // Create (restore) the file
        let create_op = FileOperation::Create {
            path: original.path.clone(),
            content,
        };

        let mut metadata = self.execute(create_op)?;

        // Restore original metadata (permissions, etc.)
        if let Some(ref file_meta) = original.original_metadata {
            file_meta.apply(&original.path)?;
        }

        metadata.op_type = OperationType::Create;
        Ok(metadata)
    }

    /// Undo modify: restore original content
    fn undo_modify(&mut self, original: &OperationMetadata) -> Result<OperationMetadata> {
        let content_hash = original
            .content_hash
            .as_ref()
            .ok_or_else(|| JanusError::MetadataCorrupted("Missing content hash".to_string()))?;

        // Retrieve original content
        let content = self.content_store.retrieve(content_hash)?;

        // Modify back to original
        let modify_op = FileOperation::Modify {
            path: original.path.clone(),
            new_content: content,
        };

        self.execute(modify_op)
    }

    /// Undo move: move back to original location
    fn undo_move(&mut self, original: &OperationMetadata) -> Result<OperationMetadata> {
        let destination = original
            .path_secondary
            .as_ref()
            .ok_or_else(|| JanusError::MetadataCorrupted("Missing secondary path".to_string()))?;

        let move_op = FileOperation::Move {
            source: destination.clone(),
            destination: original.path.clone(),
        };

        self.execute(move_op)
    }

    /// Undo copy: delete the copy
    fn undo_copy(&mut self, original: &OperationMetadata) -> Result<OperationMetadata> {
        let destination = original
            .path_secondary
            .as_ref()
            .ok_or_else(|| JanusError::MetadataCorrupted("Missing secondary path".to_string()))?;

        let delete_op = FileOperation::Delete {
            path: destination.clone(),
        };

        self.execute(delete_op)
    }

    /// Undo chmod: restore original permissions
    #[cfg(unix)]
    fn undo_chmod(&mut self, original: &OperationMetadata) -> Result<OperationMetadata> {
        let file_meta = original
            .original_metadata
            .as_ref()
            .ok_or_else(|| JanusError::MetadataCorrupted("Missing original metadata".to_string()))?;

        let chmod_op = FileOperation::Chmod {
            path: original.path.clone(),
            new_mode: file_meta.permissions,
        };

        self.execute(chmod_op)
    }

    /// Undo create: delete the created file
    fn undo_create(&mut self, original: &OperationMetadata) -> Result<OperationMetadata> {
        let delete_op = FileOperation::Delete {
            path: original.path.clone(),
        };

        self.execute(delete_op)
    }
}

/// Delete files matching a glob pattern
pub fn delete_glob(
    pattern: &str,
    base_dir: &Path,
    content_store: &ContentStore,
    metadata_store: &mut MetadataStore,
    transaction_id: Option<String>,
) -> Result<Vec<OperationMetadata>> {
    let full_pattern = base_dir.join(pattern);
    let pattern_str = full_pattern.to_string_lossy();
    let paths: Vec<PathBuf> = glob::glob(&pattern_str)?
        .filter_map(|r| r.ok())
        .filter(|p| p.is_file())
        .collect();

    let mut results = Vec::new();
    for path in paths {
        let mut executor = OperationExecutor::new(content_store, metadata_store);
        if let Some(ref tid) = transaction_id {
            executor = executor.with_transaction(tid.clone());
        }
        let meta = executor.execute(FileOperation::Delete { path })?;
        results.push(meta);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, ContentStore, MetadataStore) {
        let tmp = TempDir::new().unwrap();
        let content_store =
            ContentStore::new(tmp.path().join("content"), false).unwrap();
        let metadata_store =
            MetadataStore::new(tmp.path().join("metadata.json")).unwrap();
        (tmp, content_store, metadata_store)
    }

    #[test]
    fn test_delete_and_undo() {
        let (tmp, content_store, mut metadata_store) = setup();

        // Create a test file
        let test_file = tmp.path().join("test.txt");
        fs::write(&test_file, "hello world").unwrap();

        // Delete it
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        let delete_meta = executor
            .execute(FileOperation::Delete {
                path: test_file.clone(),
            })
            .unwrap();

        assert!(!test_file.exists());

        // Undo the delete
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        executor.undo(&delete_meta.id).unwrap();

        assert!(test_file.exists());
        assert_eq!(fs::read_to_string(&test_file).unwrap(), "hello world");
    }

    #[test]
    fn test_modify_and_undo() {
        let (tmp, content_store, mut metadata_store) = setup();

        // Create a test file
        let test_file = tmp.path().join("test.txt");
        fs::write(&test_file, "original content").unwrap();

        // Modify it
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        let modify_meta = executor
            .execute(FileOperation::Modify {
                path: test_file.clone(),
                new_content: b"modified content".to_vec(),
            })
            .unwrap();

        assert_eq!(fs::read_to_string(&test_file).unwrap(), "modified content");

        // Undo the modify
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        executor.undo(&modify_meta.id).unwrap();

        assert_eq!(fs::read_to_string(&test_file).unwrap(), "original content");
    }

    #[test]
    fn test_move_and_undo() {
        let (tmp, content_store, mut metadata_store) = setup();

        // Create a test file
        let source = tmp.path().join("source.txt");
        let dest = tmp.path().join("dest.txt");
        fs::write(&source, "content").unwrap();

        // Move it
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        let move_meta = executor
            .execute(FileOperation::Move {
                source: source.clone(),
                destination: dest.clone(),
            })
            .unwrap();

        assert!(!source.exists());
        assert!(dest.exists());

        // Undo the move
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        executor.undo(&move_meta.id).unwrap();

        assert!(source.exists());
        assert!(!dest.exists());
    }

    #[test]
    fn test_copy_and_undo() {
        let (tmp, content_store, mut metadata_store) = setup();

        // Create a test file
        let source = tmp.path().join("source.txt");
        let dest = tmp.path().join("dest.txt");
        fs::write(&source, "content").unwrap();

        // Copy it
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        let copy_meta = executor
            .execute(FileOperation::Copy {
                source: source.clone(),
                destination: dest.clone(),
            })
            .unwrap();

        assert!(source.exists());
        assert!(dest.exists());

        // Undo the copy (deletes the copy)
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        executor.undo(&copy_meta.id).unwrap();

        assert!(source.exists());
        assert!(!dest.exists());
    }
}
