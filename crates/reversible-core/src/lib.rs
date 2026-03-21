// SPDX-License-Identifier: MIT OR PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// reversible-core: Shared types for provably reversible operations
//
// Extracted from JanusKey — the development proof-of-concept for
// reversibility theory established in absolute-zero (maa-framework).
//
// Lineage:
//   maa-framework (policy) → absolute-zero (theory) → januskey (PoC)
//     → ochrance, valence-shell, aletheia (applications)
//
// This crate provides the shared foundation that all three applications
// use: content-addressed storage, operation metadata, and the
// ReversibleExecutor trait.

#![forbid(unsafe_code)]

pub mod content_store;
pub mod error;
pub mod manifest;
pub mod metadata;
pub mod transaction;

pub use content_store::{ContentHash, ContentStore};
pub use error::{ReversibleError, Result};
pub use manifest::ManifestEmitter;
pub use metadata::{
    FileMetadata, MetadataStore, OperationLog, OperationMetadata, OperationType,
};
pub use transaction::{
    OperationPreview, Transaction, TransactionLog, TransactionManager, TransactionPreview,
    TransactionState,
};

/// Trait that any reversible operation system must implement.
///
/// This is the Rust-side mirror of ochrance's `VerifiedSubsystem` interface.
/// Implementations provide:
/// - Forward execution of operations with metadata for reversal
/// - Undo of previously executed operations
/// - A2ML manifest generation for external verification by ochrance
///
/// # Absolute-Zero Correspondence
///
/// For any operation `op` and its inverse `inv`:
///   `execute(op) ; execute(inv) ≡ CNO`
/// where CNO is a Certified Null Operation (identity on state).
pub trait ReversibleExecutor {
    /// The operation type this executor handles
    type Op;
    /// Metadata produced by execution (sufficient for reversal)
    type Metadata;
    /// Error type
    type Error;

    /// Execute an operation, returning metadata sufficient for reversal.
    ///
    /// The returned metadata must contain enough information to perfectly
    /// reconstruct the pre-operation state via `undo`.
    fn execute(&mut self, op: Self::Op) -> std::result::Result<Self::Metadata, Self::Error>;

    /// Undo a previously executed operation using its metadata ID.
    ///
    /// Implements the inverse: `undo(execute(op)) ≡ CNO`.
    fn undo(&mut self, metadata_id: &str) -> std::result::Result<Self::Metadata, Self::Error>;

    /// Generate an A2ML manifest of all operations for external verification.
    ///
    /// The manifest can be consumed by ochrance's `VerifiedSubsystem::verify`
    /// to produce a `VerificationProof` (Lax, Checked, or Attested).
    fn generate_manifest(&self) -> std::result::Result<String, Self::Error>;
}
