// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
//
// Property-based tests for reversible-core.
// Uses proptest to generate arbitrary inputs and verify invariants.

use proptest::prelude::*;
use reversible_core::content_store::{ContentHash, ContentStore};
use reversible_core::transaction::{TransactionManager, TransactionState};
use tempfile::TempDir;

// --- Content hash properties ---

proptest! {
    /// Same input always produces the same hash (determinism).
    #[test]
    fn content_hash_deterministic(data in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let h1 = ContentHash::from_bytes(&data);
        let h2 = ContentHash::from_bytes(&data);
        prop_assert_eq!(h1, h2);
    }

    /// ContentHash::verify returns true for the original data.
    #[test]
    fn content_hash_verify_self(data in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let hash = ContentHash::from_bytes(&data);
        prop_assert!(hash.verify(&data));
    }

    /// ContentHash::verify returns false for different data (with high probability).
    /// We skip the trivial case where data and other happen to be equal.
    #[test]
    fn content_hash_verify_different(
        data in proptest::collection::vec(any::<u8>(), 1..1024),
        other in proptest::collection::vec(any::<u8>(), 1..1024),
    ) {
        prop_assume!(data != other);
        let hash = ContentHash::from_bytes(&data);
        prop_assert!(!hash.verify(&other));
    }
}

// --- Content store properties ---

proptest! {
    /// Store then retrieve roundtrip: retrieved data equals original.
    #[test]
    fn content_store_roundtrip(data in proptest::collection::vec(any::<u8>(), 0..8192)) {
        let tmp = TempDir::new().unwrap();
        let store = ContentStore::new(tmp.path().join("content"), false).unwrap();

        let hash = store.store(&data).unwrap();
        let retrieved = store.retrieve(&hash).unwrap();
        prop_assert_eq!(data, retrieved);
    }

    /// Store then retrieve with compression: roundtrip holds.
    #[test]
    fn content_store_compressed_roundtrip(data in proptest::collection::vec(any::<u8>(), 0..8192)) {
        let tmp = TempDir::new().unwrap();
        let store = ContentStore::new(tmp.path().join("content"), true).unwrap();

        let hash = store.store(&data).unwrap();
        let retrieved = store.retrieve(&hash).unwrap();
        prop_assert_eq!(data, retrieved);
    }

    /// Storing the same content twice yields the same hash and count stays 1.
    #[test]
    fn content_store_dedup_property(data in proptest::collection::vec(any::<u8>(), 1..4096)) {
        let tmp = TempDir::new().unwrap();
        let store = ContentStore::new(tmp.path().join("content"), false).unwrap();

        let h1 = store.store(&data).unwrap();
        let h2 = store.store(&data).unwrap();
        prop_assert_eq!(&h1, &h2);
        prop_assert_eq!(store.count().unwrap(), 1);
    }
}

// --- Transaction properties ---

proptest! {
    /// Begin-commit roundtrip: transaction ends in Committed state.
    #[test]
    fn transaction_begin_commit_roundtrip(name in "[a-z]{1,20}") {
        let tmp = TempDir::new().unwrap();
        let mut mgr = TransactionManager::new(tmp.path().join("tx.json")).unwrap();

        mgr.begin(Some(name)).unwrap();
        prop_assert!(mgr.has_active());

        let tx = mgr.commit().unwrap();
        prop_assert_eq!(tx.state, TransactionState::Committed);
        prop_assert!(!mgr.has_active());
    }

    /// Begin-rollback roundtrip: transaction ends in RolledBack state.
    #[test]
    fn transaction_begin_rollback_roundtrip(name in "[a-z]{1,20}") {
        let tmp = TempDir::new().unwrap();
        let mut mgr = TransactionManager::new(tmp.path().join("tx.json")).unwrap();

        mgr.begin(Some(name)).unwrap();
        let tx = mgr.mark_rolled_back().unwrap();
        prop_assert_eq!(tx.state, TransactionState::RolledBack);
        prop_assert!(!mgr.has_active());
    }

    /// Adding N operations to a transaction records exactly N operation IDs.
    #[test]
    fn transaction_operation_count(n in 0usize..50) {
        let tmp = TempDir::new().unwrap();
        let mut mgr = TransactionManager::new(tmp.path().join("tx.json")).unwrap();

        mgr.begin(Some("count-test".to_string())).unwrap();
        for i in 0..n {
            mgr.add_operation(format!("op-{}", i)).unwrap();
        }
        let tx = mgr.commit().unwrap();
        prop_assert_eq!(tx.operation_ids.len(), n);
    }
}
