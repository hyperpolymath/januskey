// SPDX-License-Identifier: MIT OR PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// Error types for reversible-core

use thiserror::Error;

/// Result type alias for reversible-core operations
pub type Result<T> = std::result::Result<T, ReversibleError>;

/// Error types for reversible operations
#[derive(Error, Debug)]
pub enum ReversibleError {
    #[error("Directory not initialized: {0}")]
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
}
