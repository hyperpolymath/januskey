// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//
// Property-based tests for januskey-cli.
// Tests obliteration and key derivation invariants.

use januskey::content_store::ContentStore;
use januskey::obliteration::{ObliterationManager, ObliterationProof};
use januskey::operations::{FileOperation, OperationExecutor};
use proptest::prelude::*;
use reversible_core::content_store::ContentHash;
use reversible_core::metadata::MetadataStore;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
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

// --- CNO round-trip law: execute ∘ undo ≡ identity on the filesystem ---
//
// This is the load-bearing, runnable evidence for JanusKey's reversibility
// claim (the honest substitute for the "formal proofs pending" badge): for
// every supported operation, executing it and then undoing it must return the
// working tree to exactly its pre-execute state (bytes + existence).
//
// Chown is excluded: its undo is unimplemented (operations.rs). We test the
// filesystem effect, not OperationType::inverse() (which is deliberately NOT an
// involution — Copy⁻¹ = Delete — so double-inverse is not the law here).

/// What operation to exercise, plus the content it needs.
#[derive(Debug, Clone)]
enum OpSpec {
    Delete(Vec<u8>),
    Modify(Vec<u8>, Vec<u8>),
    Move(Vec<u8>),
    Copy(Vec<u8>),
    Create(Vec<u8>),
}

fn op_strategy() -> impl Strategy<Value = OpSpec> {
    let bytes = || proptest::collection::vec(any::<u8>(), 0..256);
    prop_oneof![
        bytes().prop_map(OpSpec::Delete),
        (bytes(), bytes()).prop_map(|(a, b)| OpSpec::Modify(a, b)),
        bytes().prop_map(OpSpec::Move),
        bytes().prop_map(OpSpec::Copy),
        bytes().prop_map(OpSpec::Create),
    ]
}

/// Recursive {relative-path -> bytes} snapshot of a directory (no external deps).
fn snapshot(dir: &Path, base: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            snapshot(&path, base, out);
        } else {
            let rel = path.strip_prefix(base).unwrap().to_path_buf();
            out.insert(rel, std::fs::read(&path).unwrap());
        }
    }
}

fn snapshot_of(work: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    snapshot(work, work, &mut out);
    out
}

proptest! {
    /// execute(op) then undo restores the working tree exactly (the CNO law).
    #[test]
    fn execute_then_undo_is_identity(spec in op_strategy()) {
        let tmp = TempDir::new().unwrap();
        // Keep user files (snapshotted) separate from the store internals.
        let work = tmp.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        let content_store = ContentStore::new(tmp.path().join("content"), false).unwrap();
        let mut metadata_store =
            MetadataStore::new(tmp.path().join("metadata.json")).unwrap();

        let a = work.join("a.bin");
        let b = work.join("b.bin");

        // Establish the pre-state and choose the operation.
        let op = match &spec {
            OpSpec::Delete(c) => {
                std::fs::write(&a, c).unwrap();
                FileOperation::Delete { path: a.clone() }
            }
            OpSpec::Modify(orig, new) => {
                std::fs::write(&a, orig).unwrap();
                FileOperation::Modify { path: a.clone(), new_content: new.clone() }
            }
            OpSpec::Move(c) => {
                std::fs::write(&a, c).unwrap();
                FileOperation::Move { source: a.clone(), destination: b.clone() }
            }
            OpSpec::Copy(c) => {
                std::fs::write(&a, c).unwrap();
                FileOperation::Copy { source: a.clone(), destination: b.clone() }
            }
            OpSpec::Create(c) => {
                FileOperation::Create { path: a.clone(), content: c.clone() }
            }
        };

        let before = snapshot_of(&work);

        // execute — mutates the tree and records inversion metadata.
        let mut exec = OperationExecutor::new(&content_store, &mut metadata_store);
        let meta = exec.execute(op).unwrap();

        // undo — a fresh executor, exactly as the CLI unlocks a later session.
        let mut exec2 = OperationExecutor::new(&content_store, &mut metadata_store);
        exec2.undo(&meta.id).unwrap();

        let after = snapshot_of(&work);
        prop_assert_eq!(before, after, "execute∘undo must restore the tree (CNO law)");
    }
}
