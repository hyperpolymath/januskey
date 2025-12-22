// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// JanusKey Error Types

use thiserror::Error;

/// Result type alias for JanusKey operations
pub type Result<T> = std::result::Result<T, JanusError>;

/// JanusKey error types
#[derive(Error, Debug)]
pub enum JanusError {
    #[error("Directory not initialized: {0}. Run 'jk init' first.")]
    NotInitialized(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("Path already exists: {0}")]
    PathExists(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("No active transaction")]
    NoActiveTransaction,

    #[error("Transaction already active: {0}")]
    TransactionActive(String),

    #[error("Nothing to undo")]
    NothingToUndo,

    #[error("Invalid operation ID: {0}")]
    InvalidOperationId(String),

    #[error("Content integrity error: expected {expected}, got {actual}")]
    ContentIntegrityError { expected: String, actual: String },

    #[error("Metadata corrupted: {0}")]
    MetadataCorrupted(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid glob pattern: {0}")]
    InvalidPattern(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Glob pattern error: {0}")]
    Glob(#[from] glob::PatternError),

    #[error("User cancelled operation")]
    UserCancelled,
}
