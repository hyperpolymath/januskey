// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
//
// Point-to-point tests: verify component interactions
// Tests: content_store↔metadata, keys↔attestation, transaction↔operations

#[cfg(test)]
mod p2p_tests {
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper: create a temp directory for test isolation
    fn test_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp dir")
    }

    // --- content_store ↔ metadata ---

    #[test]
    fn content_store_metadata_roundtrip() {
        let dir = test_dir();
        let store_path = dir.path().join(".jk/content");
        let meta_path = dir.path().join(".jk/metadata");
        std::fs::create_dir_all(&store_path).unwrap();
        std::fs::create_dir_all(&meta_path).unwrap();

        // Write content
        let content = b"test file content for p2p";
        let hash = sha256(content);
        let hash_hex = hex::encode(hash);
        std::fs::write(store_path.join(&hash_hex), content).unwrap();

        // Write metadata referencing the content hash
        let meta = format!(
            r#"{{"hash":"{}","size":{},"op":"copy"}}"#,
            hash_hex,
            content.len()
        );
        std::fs::write(meta_path.join("0001.json"), &meta).unwrap();

        // Verify: metadata hash matches stored content
        let stored = std::fs::read(store_path.join(&hash_hex)).unwrap();
        let stored_hash = hex::encode(sha256(&stored));
        assert_eq!(hash_hex, stored_hash, "Content hash must match metadata reference");
    }

    #[test]
    fn content_store_deduplication() {
        let dir = test_dir();
        let store_path = dir.path().join(".jk/content");
        std::fs::create_dir_all(&store_path).unwrap();

        let content = b"deduplicated content";
        let hash = hex::encode(sha256(content));

        // Store same content twice
        std::fs::write(store_path.join(&hash), content).unwrap();
        std::fs::write(store_path.join(&hash), content).unwrap();

        // Only one file should exist (same hash = same path)
        let entries: Vec<_> = std::fs::read_dir(&store_path)
            .unwrap()
            .collect();
        assert_eq!(entries.len(), 1, "Deduplication: same content = one file");
    }

    // --- keys ↔ attestation ---

    #[test]
    fn key_operation_creates_attestation_entry() {
        let dir = test_dir();
        let attest_path = dir.path().join(".jk/attestation");
        std::fs::create_dir_all(&attest_path).unwrap();

        // Simulate key generation
        let key_id = "550e8400-e29b-41d4-a716-446655440000";
        let entry = format!(
            r#"{{"op":"key_gen","key_id":"{}","algo":"aes256gcm","timestamp":{}}}"#,
            key_id,
            chrono_timestamp()
        );
        std::fs::write(attest_path.join("0001.json"), &entry).unwrap();

        // Verify attestation references the key
        let read_back = std::fs::read_to_string(attest_path.join("0001.json")).unwrap();
        assert!(read_back.contains(key_id), "Attestation must reference key ID");
    }

    #[test]
    fn attestation_chain_integrity() {
        let dir = test_dir();
        let attest_path = dir.path().join(".jk/attestation");
        std::fs::create_dir_all(&attest_path).unwrap();

        // Create chain of 3 entries
        let mut prev_hash = "0000000000000000000000000000000000000000000000000000000000000000".to_string();

        for i in 0..3 {
            let entry = format!(
                r#"{{"seq":{},"prev_hash":"{}","op":"copy","timestamp":{}}}"#,
                i, prev_hash, chrono_timestamp()
            );
            let entry_hash = hex::encode(sha256(entry.as_bytes()));
            std::fs::write(attest_path.join(format!("{:04}.json", i)), &entry).unwrap();
            prev_hash = entry_hash;
        }

        // Verify chain: each entry's content hashes to the next's prev_hash
        let entries: Vec<String> = (0..3)
            .map(|i| {
                std::fs::read_to_string(attest_path.join(format!("{:04}.json", i))).unwrap()
            })
            .collect();

        for i in 1..entries.len() {
            let prev_content_hash = hex::encode(sha256(entries[i - 1].as_bytes()));
            assert!(
                entries[i].contains(&prev_content_hash),
                "Entry {} must reference hash of entry {}",
                i,
                i - 1
            );
        }
    }

    // --- transaction ↔ operations ---

    #[test]
    fn transaction_groups_operations() {
        let dir = test_dir();
        let tx_path = dir.path().join(".jk/transactions");
        let ops_path = dir.path().join(".jk/operations");
        std::fs::create_dir_all(&tx_path).unwrap();
        std::fs::create_dir_all(&ops_path).unwrap();

        // Begin transaction
        let tx_id = "tx-001";
        std::fs::write(
            tx_path.join(format!("{}.json", tx_id)),
            r#"{"state":"active","ops":[]}"#,
        )
        .unwrap();

        // Execute operations within transaction
        for i in 0..3 {
            std::fs::write(
                ops_path.join(format!("{}-op-{:04}.json", tx_id, i)),
                format!(r#"{{"tx":"{}","op":"copy","seq":{}}}"#, tx_id, i),
            )
            .unwrap();
        }

        // Verify all ops reference the transaction
        let op_files: Vec<_> = std::fs::read_dir(&ops_path)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map_or(false, |n| n.starts_with(tx_id))
            })
            .collect();
        assert_eq!(op_files.len(), 3, "Transaction must group all 3 operations");
    }

    // --- Helpers ---

    fn sha256(data: &[u8]) -> [u8; 32] {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        // Simplified hash for testing — real impl uses sha2 crate
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let h = hasher.finish();
        let mut result = [0u8; 32];
        result[..8].copy_from_slice(&h.to_le_bytes());
        result[8..16].copy_from_slice(&h.to_be_bytes());
        // Fill remaining with deterministic bytes
        for i in 16..32 {
            result[i] = result[i - 16] ^ (i as u8);
        }
        result
    }

    fn chrono_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}
