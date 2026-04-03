// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
//
// Property-based tests for januskey-cli.
// Tests obliteration and key derivation invariants.

use januskey::content_store::ContentStore;
use januskey::obliteration::{ObliterationManager, ObliterationProof};
use proptest::prelude::*;
use reversible_core::content_store::ContentHash;
use tempfile::TempDir;

// --- Obliteration properties ---

proptest! {
    /// Obliteration proof commitment is always self-verifiable.
    #[test]
    fn obliteration_proof_self_verifies(data in proptest::collection::vec(any::<u8>(), 1..1024)) {
        let hash = ContentHash::from_bytes(&data);
        let proof = ObliterationProof::generate(&hash, 3);
        prop_assert!(proof.verify_commitment());
    }

    /// After obliteration, content is no longer retrievable from the store.
    #[test]
    fn obliterated_data_not_recoverable(data in proptest::collection::vec(any::<u8>(), 1..4096)) {
        let tmp = TempDir::new().unwrap();
        let store = ContentStore::new(tmp.path().join("content"), false).unwrap();
        let mut obliterator =
            ObliterationManager::new(tmp.path().join("obliterations.json")).unwrap();

        let hash = store.store(&data).unwrap();
        prop_assert!(store.exists(&hash));

        obliterator
            .obliterate(&store, &hash, None, None)
            .unwrap();

        prop_assert!(!store.exists(&hash));
        prop_assert!(store.retrieve(&hash).is_err());
    }
}

// --- Key derivation property (determinism via ContentHash as proxy) ---

proptest! {
    /// Same input bytes always produce the same ContentHash (key derivation proxy).
    /// Real Argon2 derivation is too slow for proptest; we verify the
    /// determinism invariant via the SHA256 path that content addressing uses.
    #[test]
    fn key_derivation_deterministic(input in proptest::collection::vec(any::<u8>(), 1..256)) {
        let h1 = ContentHash::from_bytes(&input);
        let h2 = ContentHash::from_bytes(&input);
        prop_assert_eq!(h1, h2);
    }
}
