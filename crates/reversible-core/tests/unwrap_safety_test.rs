// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
//
// Regression tests: verify that known-bad inputs do not panic.
// Each test here corresponds to a discovered issue.

use reversible_core::content_store::{ContentHash, ContentStore};
use reversible_core::transaction::TransactionManager;
use tempfile::TempDir;

/// Regression: retrieving a non-existent hash must return Err, not panic.
/// Previously, code paths could unwrap() on missing files.
#[test]
fn retrieve_nonexistent_hash_returns_error() {
    let tmp = TempDir::new().unwrap();
    let store = ContentStore::new(tmp.path().join("content"), false).unwrap();

    let fake_hash = ContentHash::from_bytes(b"this content was never stored");

    // Must return Err, not panic
    let result = store.retrieve(&fake_hash);
    assert!(result.is_err(), "Retrieving non-existent hash must return Err");
}

/// Regression: committing with no active transaction must return Err, not panic.
#[test]
fn commit_without_active_transaction_returns_error() {
    let tmp = TempDir::new().unwrap();
    let mut manager =
        TransactionManager::new(tmp.path().join("transactions.json")).unwrap();

    // No begin() called — commit must fail gracefully
    let result = manager.commit();
    assert!(result.is_err(), "Commit without active transaction must return Err");
}

/// Regression: rollback with no active transaction must return Err, not panic.
#[test]
fn rollback_without_active_transaction_returns_error() {
    let tmp = TempDir::new().unwrap();
    let mut manager =
        TransactionManager::new(tmp.path().join("transactions.json")).unwrap();

    let result = manager.mark_rolled_back();
    assert!(result.is_err(), "Rollback without active transaction must return Err");
}

/// Regression: content store with empty bytes must not panic.
#[test]
fn store_empty_content() {
    let tmp = TempDir::new().unwrap();
    let store = ContentStore::new(tmp.path().join("content"), false).unwrap();

    let hash = store.store(b"").unwrap();
    let retrieved = store.retrieve(&hash).unwrap();
    assert_eq!(retrieved, b"");
}

/// Regression: ContentHash::verify with mismatched content returns false, not panic.
#[test]
fn content_hash_verify_mismatch() {
    let hash = ContentHash::from_bytes(b"original");
    assert!(!hash.verify(b"tampered"), "Mismatched content must return false");
}
