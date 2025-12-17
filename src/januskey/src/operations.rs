// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// Reversible File Operations
// Each operation stores sufficient metadata for perfect inversion

use crate::content_store::{ContentHash, ContentStore};
use crate::delta::Delta;
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
    // === Extended Operations ===
    /// Create a directory (reversible: rmdir)
    Mkdir {
        path: PathBuf,
        /// Create parent directories if they don't exist
        parents: bool,
    },
    /// Remove an empty directory (reversible: mkdir)
    Rmdir {
        path: PathBuf,
    },
    /// Remove directory recursively (reversible: restore all contents)
    RmdirRecursive {
        path: PathBuf,
    },
    /// Create a symbolic link (reversible: remove symlink)
    #[cfg(unix)]
    Symlink {
        target: PathBuf,
        link_path: PathBuf,
    },
    /// Append content to a file (reversible: truncate)
    Append {
        path: PathBuf,
        content: Vec<u8>,
    },
    /// Truncate a file to a specific size (reversible: restore original)
    Truncate {
        path: PathBuf,
        new_size: u64,
    },
    /// Update file timestamps (reversible: restore original timestamps)
    Touch {
        path: PathBuf,
        /// If true, create file if it doesn't exist
        create: bool,
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
            Self::Mkdir { .. } => OperationType::Mkdir,
            Self::Rmdir { .. } | Self::RmdirRecursive { .. } => OperationType::Rmdir,
            #[cfg(unix)]
            Self::Symlink { .. } => OperationType::Symlink,
            Self::Append { .. } => OperationType::Append,
            Self::Truncate { .. } => OperationType::Truncate,
            Self::Touch { .. } => OperationType::Touch,
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
            Self::Mkdir { path, .. } => path,
            Self::Rmdir { path } | Self::RmdirRecursive { path } => path,
            #[cfg(unix)]
            Self::Symlink { link_path, .. } => link_path,
            Self::Append { path, .. } => path,
            Self::Truncate { path, .. } => path,
            Self::Touch { path, .. } => path,
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
            // Extended operations
            FileOperation::Mkdir { path, parents } => self.execute_mkdir(&path, parents),
            FileOperation::Rmdir { path } => self.execute_rmdir(&path),
            FileOperation::RmdirRecursive { path } => self.execute_rmdir_recursive(&path),
            #[cfg(unix)]
            FileOperation::Symlink { target, link_path } => {
                self.execute_symlink(&target, &link_path)
            }
            FileOperation::Append { path, content } => self.execute_append(&path, &content),
            FileOperation::Truncate { path, new_size } => self.execute_truncate(&path, new_size),
            FileOperation::Touch { path, create } => self.execute_touch(&path, create),
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

    /// Execute modify operation with optional delta storage for efficiency
    /// Note: Delta storage is currently disabled pending refinement of the diff algorithm.
    /// Set JANUSKEY_USE_DELTA=1 environment variable to enable experimental delta storage.
    fn execute_modify(&mut self, path: &Path, new_content: &[u8]) -> Result<OperationMetadata> {
        if !path.exists() {
            return Err(JanusError::FileNotFound(path.display().to_string()));
        }

        // Capture original content
        let original_content = fs::read(path)?;
        let file_metadata = FileMetadata::from_path(path)?;
        let new_hash = ContentHash::from_bytes(new_content);

        // Delta storage is opt-in for now (pending algorithm refinement)
        let use_experimental_delta = std::env::var("JANUSKEY_USE_DELTA").is_ok();

        let (content_hash, is_delta) = if use_experimental_delta {
            // Compute REVERSE delta (from new to original) for undo operations
            // This allows us to reconstruct original from new content during undo
            let reverse_delta = Delta::compute(new_content, &original_content);

            if !reverse_delta.is_full() {
                // Store the reverse delta (more efficient for large files with small changes)
                let delta_bytes = reverse_delta.into_bytes();
                let hash = self.content_store.store(&delta_bytes)?;
                (hash, true)
            } else {
                // Store full original content (for small files or large changes)
                let hash = self.content_store.store(&original_content)?;
                (hash, false)
            }
        } else {
            // Store full original content (default, most reliable)
            let hash = self.content_store.store(&original_content)?;
            (hash, false)
        };

        // Create operation metadata
        let mut metadata = OperationMetadata::new(OperationType::Modify, path.to_path_buf())
            .with_content_hash(content_hash)
            .with_new_content_hash(new_hash)
            .with_original_metadata(file_metadata)
            .with_delta(is_delta);

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

    // === Extended Operations Implementation ===

    /// Execute mkdir operation
    fn execute_mkdir(&mut self, path: &Path, parents: bool) -> Result<OperationMetadata> {
        if path.exists() {
            return Err(JanusError::PathExists(path.display().to_string()));
        }

        // Create operation metadata
        let mut metadata = OperationMetadata::new(OperationType::Mkdir, path.to_path_buf());

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Perform mkdir
        if parents {
            fs::create_dir_all(path)?;
        } else {
            fs::create_dir(path)?;
        }

        // Record and return
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
    }

    /// Execute rmdir operation (empty directory only)
    fn execute_rmdir(&mut self, path: &Path) -> Result<OperationMetadata> {
        if !path.exists() {
            return Err(JanusError::DirectoryNotFound(path.display().to_string()));
        }
        if !path.is_dir() {
            return Err(JanusError::OperationFailed(format!(
                "{} is not a directory",
                path.display()
            )));
        }

        // Capture metadata
        let file_metadata = FileMetadata::from_path(path)?;

        // Create operation metadata
        let mut metadata = OperationMetadata::new(OperationType::Rmdir, path.to_path_buf())
            .with_original_metadata(file_metadata);

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Perform rmdir
        fs::remove_dir(path)?;

        // Record and return
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
    }

    /// Execute recursive rmdir operation - stores all contents for reversal
    fn execute_rmdir_recursive(&mut self, path: &Path) -> Result<OperationMetadata> {
        if !path.exists() {
            return Err(JanusError::DirectoryNotFound(path.display().to_string()));
        }
        if !path.is_dir() {
            return Err(JanusError::OperationFailed(format!(
                "{} is not a directory",
                path.display()
            )));
        }

        // Collect all files in directory for storage
        let mut stored_files: Vec<(PathBuf, ContentHash)> = Vec::new();
        for entry in walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let content = fs::read(entry.path())?;
                let hash = self.content_store.store(&content)?;
                let relative_path = entry.path().strip_prefix(path).unwrap_or(entry.path());
                stored_files.push((relative_path.to_path_buf(), hash));
            }
        }

        // Store the file manifest as JSON
        let manifest = serde_json::to_vec(&stored_files)?;
        let manifest_hash = self.content_store.store(&manifest)?;

        // Capture metadata
        let file_metadata = FileMetadata::from_path(path)?;

        // Create operation metadata
        let mut metadata = OperationMetadata::new(OperationType::Rmdir, path.to_path_buf())
            .with_original_metadata(file_metadata)
            .with_content_hash(manifest_hash);

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Perform recursive remove
        fs::remove_dir_all(path)?;

        // Record and return
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
    }

    /// Execute symlink operation
    #[cfg(unix)]
    fn execute_symlink(&mut self, target: &Path, link_path: &Path) -> Result<OperationMetadata> {
        use std::os::unix::fs::symlink;

        if link_path.exists() {
            return Err(JanusError::PathExists(link_path.display().to_string()));
        }

        // Create parent directory if needed
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create operation metadata
        let mut metadata = OperationMetadata::new(OperationType::Symlink, link_path.to_path_buf())
            .with_secondary_path(target.to_path_buf());

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Create the symlink
        symlink(target, link_path)?;

        // Record and return
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
    }

    /// Execute append operation
    fn execute_append(&mut self, path: &Path, content: &[u8]) -> Result<OperationMetadata> {
        use std::io::Write;

        if !path.exists() {
            return Err(JanusError::FileNotFound(path.display().to_string()));
        }

        // Get original file size (for reversal via truncation)
        let original_metadata = FileMetadata::from_path(path)?;

        // Store the appended content for reference
        let content_hash = self.content_store.store(content)?;

        // Create operation metadata with original size stored
        let mut metadata = OperationMetadata::new(OperationType::Append, path.to_path_buf())
            .with_original_metadata(original_metadata)
            .with_content_hash(content_hash);

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Perform the append
        let mut file = fs::OpenOptions::new().append(true).open(path)?;
        file.write_all(content)?;

        // Record and return
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
    }

    /// Execute truncate operation
    fn execute_truncate(&mut self, path: &Path, new_size: u64) -> Result<OperationMetadata> {
        if !path.exists() {
            return Err(JanusError::FileNotFound(path.display().to_string()));
        }

        // Store original content for reversal
        let original_content = fs::read(path)?;
        let original_metadata = FileMetadata::from_path(path)?;
        let content_hash = self.content_store.store(&original_content)?;

        // Create operation metadata
        let mut metadata = OperationMetadata::new(OperationType::Truncate, path.to_path_buf())
            .with_original_metadata(original_metadata)
            .with_content_hash(content_hash);

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Perform the truncate
        let file = fs::OpenOptions::new().write(true).open(path)?;
        file.set_len(new_size)?;

        // Record and return
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
    }

    /// Execute touch operation
    fn execute_touch(&mut self, path: &Path, create: bool) -> Result<OperationMetadata> {
        use std::time::SystemTime;

        let file_existed = path.exists();

        // Capture original metadata if file exists
        let original_metadata = if file_existed {
            Some(FileMetadata::from_path(path)?)
        } else {
            None
        };

        // Create operation metadata
        let mut metadata = OperationMetadata::new(OperationType::Touch, path.to_path_buf());
        if let Some(orig_meta) = original_metadata {
            metadata = metadata.with_original_metadata(orig_meta);
        }

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Perform the touch
        if !file_existed && create {
            // Create empty file
            fs::write(path, b"")?;
        } else if file_existed {
            // Update modification time
            let now = SystemTime::now();
            filetime::set_file_mtime(path, filetime::FileTime::from_system_time(now))?;
        } else {
            return Err(JanusError::FileNotFound(path.display().to_string()));
        }

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
            // Extended operations
            OperationType::Mkdir => self.undo_mkdir(&original_op)?,
            OperationType::Rmdir => self.undo_rmdir(&original_op)?,
            OperationType::Symlink => {
                #[cfg(unix)]
                {
                    self.undo_symlink(&original_op)?
                }
                #[cfg(not(unix))]
                {
                    return Err(JanusError::OperationFailed(
                        "Symlink not supported on this platform".to_string(),
                    ));
                }
            }
            OperationType::Append => self.undo_append(&original_op)?,
            OperationType::Truncate => self.undo_truncate(&original_op)?,
            OperationType::Touch => self.undo_touch(&original_op)?,
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

    /// Undo modify: restore original content (handles both delta and full storage)
    fn undo_modify(&mut self, original: &OperationMetadata) -> Result<OperationMetadata> {
        let content_hash = original
            .content_hash
            .as_ref()
            .ok_or_else(|| JanusError::MetadataCorrupted("Missing content hash".to_string()))?;

        // Retrieve stored data (either delta or full content)
        let stored_data = self.content_store.retrieve(content_hash)?;

        // Reconstruct original content
        let original_content = if original.is_delta {
            // Stored data is a reverse delta - apply it to current file content
            let current_content = fs::read(&original.path)?;
            let delta = Delta::from_bytes(&stored_data)
                .ok_or_else(|| JanusError::MetadataCorrupted("Invalid delta format".to_string()))?;
            delta
                .apply(&current_content)
                .ok_or_else(|| JanusError::MetadataCorrupted("Failed to apply delta".to_string()))?
        } else {
            // Stored data is full original content
            stored_data
        };

        // Get current metadata before modification
        let file_metadata = FileMetadata::from_path(&original.path)?;
        let new_hash = ContentHash::from_bytes(&original_content);

        // Store the current content (for potential re-undo)
        // Since this is an undo, store full content, not delta
        let current_content = fs::read(&original.path)?;
        let current_hash = self.content_store.store(&current_content)?;

        // Create operation metadata for the undo
        let mut metadata = OperationMetadata::new(OperationType::Modify, original.path.clone())
            .with_content_hash(current_hash)
            .with_new_content_hash(new_hash)
            .with_original_metadata(file_metadata)
            .with_delta(false); // Undo operations store full content for reliability

        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }

        // Write the original content back
        fs::write(&original.path, &original_content)?;

        // Record and return
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
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

    // === Extended Undo Operations ===

    /// Undo mkdir: remove the created directory
    fn undo_mkdir(&mut self, original: &OperationMetadata) -> Result<OperationMetadata> {
        let rmdir_op = FileOperation::Rmdir {
            path: original.path.clone(),
        };

        self.execute(rmdir_op)
    }

    /// Undo rmdir: recreate the directory
    fn undo_rmdir(&mut self, original: &OperationMetadata) -> Result<OperationMetadata> {
        // If content_hash exists, this was a recursive rmdir - restore all files
        if let Some(ref manifest_hash) = original.content_hash {
            let manifest_data = self.content_store.retrieve(manifest_hash)?;
            let stored_files: Vec<(PathBuf, ContentHash)> = serde_json::from_slice(&manifest_data)?;

            // Recreate directory
            fs::create_dir_all(&original.path)?;

            // Restore all files
            for (relative_path, content_hash) in stored_files {
                let full_path = original.path.join(&relative_path);
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let content = self.content_store.retrieve(&content_hash)?;
                fs::write(&full_path, content)?;
            }

            // Create metadata for the undo
            let mut metadata = OperationMetadata::new(OperationType::Mkdir, original.path.clone());
            if let Some(ref tid) = self.transaction_id {
                metadata = metadata.with_transaction_id(tid.clone());
            }
            self.metadata_store.append(metadata.clone())?;
            Ok(metadata)
        } else {
            // Simple rmdir - just recreate empty directory
            let mkdir_op = FileOperation::Mkdir {
                path: original.path.clone(),
                parents: false,
            };
            self.execute(mkdir_op)
        }
    }

    /// Undo symlink: remove the symlink
    #[cfg(unix)]
    fn undo_symlink(&mut self, original: &OperationMetadata) -> Result<OperationMetadata> {
        // Remove the symlink (which is just an fs::remove_file)
        fs::remove_file(&original.path)?;

        let mut metadata = OperationMetadata::new(OperationType::Delete, original.path.clone());
        if let Some(ref tid) = self.transaction_id {
            metadata = metadata.with_transaction_id(tid.clone());
        }
        self.metadata_store.append(metadata.clone())?;
        Ok(metadata)
    }

    /// Undo append: truncate back to original size
    fn undo_append(&mut self, original: &OperationMetadata) -> Result<OperationMetadata> {
        let file_meta = original
            .original_metadata
            .as_ref()
            .ok_or_else(|| JanusError::MetadataCorrupted("Missing original metadata".to_string()))?;

        // Truncate to original size
        let truncate_op = FileOperation::Truncate {
            path: original.path.clone(),
            new_size: file_meta.size,
        };

        self.execute(truncate_op)
    }

    /// Undo truncate: restore original content
    fn undo_truncate(&mut self, original: &OperationMetadata) -> Result<OperationMetadata> {
        let content_hash = original
            .content_hash
            .as_ref()
            .ok_or_else(|| JanusError::MetadataCorrupted("Missing content hash".to_string()))?;

        // Retrieve and restore original content
        let content = self.content_store.retrieve(content_hash)?;

        let modify_op = FileOperation::Modify {
            path: original.path.clone(),
            new_content: content,
        };

        self.execute(modify_op)
    }

    /// Undo touch: restore original timestamp or delete if created
    fn undo_touch(&mut self, original: &OperationMetadata) -> Result<OperationMetadata> {
        if let Some(ref file_meta) = original.original_metadata {
            // File existed before - restore original timestamp
            let mtime = filetime::FileTime::from_system_time(file_meta.modified.into());
            filetime::set_file_mtime(&original.path, mtime)?;

            let mut metadata = OperationMetadata::new(OperationType::Touch, original.path.clone());
            if let Some(ref tid) = self.transaction_id {
                metadata = metadata.with_transaction_id(tid.clone());
            }
            self.metadata_store.append(metadata.clone())?;
            Ok(metadata)
        } else {
            // File was created by touch - delete it
            let delete_op = FileOperation::Delete {
                path: original.path.clone(),
            };
            self.execute(delete_op)
        }
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

    // === Tests for Extended Operations ===

    #[test]
    fn test_mkdir_and_undo() {
        let (tmp, content_store, mut metadata_store) = setup();

        let test_dir = tmp.path().join("new_dir");

        // Create directory
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        let mkdir_meta = executor
            .execute(FileOperation::Mkdir {
                path: test_dir.clone(),
                parents: false,
            })
            .unwrap();

        assert!(test_dir.exists());
        assert!(test_dir.is_dir());

        // Undo mkdir (removes directory)
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        executor.undo(&mkdir_meta.id).unwrap();

        assert!(!test_dir.exists());
    }

    #[test]
    fn test_rmdir_and_undo() {
        let (tmp, content_store, mut metadata_store) = setup();

        let test_dir = tmp.path().join("empty_dir");
        fs::create_dir(&test_dir).unwrap();

        // Remove directory
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        let rmdir_meta = executor
            .execute(FileOperation::Rmdir {
                path: test_dir.clone(),
            })
            .unwrap();

        assert!(!test_dir.exists());

        // Undo rmdir (recreates directory)
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        executor.undo(&rmdir_meta.id).unwrap();

        assert!(test_dir.exists());
        assert!(test_dir.is_dir());
    }

    #[test]
    fn test_rmdir_recursive_and_undo() {
        let (tmp, content_store, mut metadata_store) = setup();

        // Create directory with files
        let test_dir = tmp.path().join("dir_with_files");
        fs::create_dir(&test_dir).unwrap();
        fs::write(test_dir.join("file1.txt"), "content1").unwrap();
        fs::write(test_dir.join("file2.txt"), "content2").unwrap();
        fs::create_dir(test_dir.join("subdir")).unwrap();
        fs::write(test_dir.join("subdir").join("nested.txt"), "nested").unwrap();

        // Remove recursively
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        let rmdir_meta = executor
            .execute(FileOperation::RmdirRecursive {
                path: test_dir.clone(),
            })
            .unwrap();

        assert!(!test_dir.exists());

        // Undo rmdir (restores directory and contents)
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        executor.undo(&rmdir_meta.id).unwrap();

        assert!(test_dir.exists());
        assert_eq!(fs::read_to_string(test_dir.join("file1.txt")).unwrap(), "content1");
        assert_eq!(fs::read_to_string(test_dir.join("file2.txt")).unwrap(), "content2");
        assert_eq!(fs::read_to_string(test_dir.join("subdir/nested.txt")).unwrap(), "nested");
    }

    #[test]
    fn test_append_and_undo() {
        let (tmp, content_store, mut metadata_store) = setup();

        let test_file = tmp.path().join("append.txt");
        fs::write(&test_file, "original").unwrap();

        // Append content
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        let append_meta = executor
            .execute(FileOperation::Append {
                path: test_file.clone(),
                content: b" appended".to_vec(),
            })
            .unwrap();

        assert_eq!(fs::read_to_string(&test_file).unwrap(), "original appended");

        // Undo append (truncates back)
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        executor.undo(&append_meta.id).unwrap();

        assert_eq!(fs::read_to_string(&test_file).unwrap(), "original");
    }

    #[test]
    fn test_truncate_and_undo() {
        let (tmp, content_store, mut metadata_store) = setup();

        let test_file = tmp.path().join("truncate.txt");
        fs::write(&test_file, "this is a long string").unwrap();

        // Truncate to 4 bytes
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        let truncate_meta = executor
            .execute(FileOperation::Truncate {
                path: test_file.clone(),
                new_size: 4,
            })
            .unwrap();

        assert_eq!(fs::read_to_string(&test_file).unwrap(), "this");

        // Undo truncate (restores original)
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        executor.undo(&truncate_meta.id).unwrap();

        assert_eq!(fs::read_to_string(&test_file).unwrap(), "this is a long string");
    }

    #[cfg(unix)]
    #[test]
    fn test_symlink_and_undo() {
        let (tmp, content_store, mut metadata_store) = setup();

        let target = tmp.path().join("target.txt");
        let link = tmp.path().join("link.txt");
        fs::write(&target, "target content").unwrap();

        // Create symlink
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        let symlink_meta = executor
            .execute(FileOperation::Symlink {
                target: target.clone(),
                link_path: link.clone(),
            })
            .unwrap();

        assert!(link.exists());
        assert!(link.is_symlink());

        // Undo symlink (removes link)
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        executor.undo(&symlink_meta.id).unwrap();

        assert!(!link.exists());
        assert!(target.exists()); // Target should still exist
    }

    #[test]
    fn test_modify_large_file_and_undo() {
        let (tmp, content_store, mut metadata_store) = setup();

        // Create a large test file (above DELTA_THRESHOLD of 4KB)
        let test_file = tmp.path().join("large.txt");
        let original_content = "Line 1\nLine 2\nLine 3\n".repeat(500); // ~11KB
        fs::write(&test_file, &original_content).unwrap();

        // Modify with small change
        let mut modified_content = original_content.clone();
        modified_content.push_str("New line at end\n");

        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        let modify_meta = executor
            .execute(FileOperation::Modify {
                path: test_file.clone(),
                new_content: modified_content.as_bytes().to_vec(),
            })
            .unwrap();

        // Verify modification occurred
        assert_eq!(fs::read_to_string(&test_file).unwrap(), modified_content);

        // Without JANUSKEY_USE_DELTA, delta should not be used
        assert!(!modify_meta.is_delta, "Delta should be disabled by default");

        // Undo the modify
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        executor.undo(&modify_meta.id).unwrap();

        // Verify original content is restored
        assert_eq!(fs::read_to_string(&test_file).unwrap(), original_content);
    }

    #[test]
    fn test_modify_small_file_uses_full_storage() {
        let (tmp, content_store, mut metadata_store) = setup();

        // Create a small test file (below DELTA_THRESHOLD)
        let test_file = tmp.path().join("small.txt");
        let original_content = "small content";
        fs::write(&test_file, original_content).unwrap();

        // Modify it
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        let modify_meta = executor
            .execute(FileOperation::Modify {
                path: test_file.clone(),
                new_content: b"different small content".to_vec(),
            })
            .unwrap();

        // Small files should not use delta
        assert!(!modify_meta.is_delta);

        // Undo should still work
        let mut executor = OperationExecutor::new(&content_store, &mut metadata_store);
        executor.undo(&modify_meta.id).unwrap();

        assert_eq!(fs::read_to_string(&test_file).unwrap(), original_content);
    }
}
