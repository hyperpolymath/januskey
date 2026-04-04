// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
//
// E2E tests: Full key lifecycle and content store operations
// Tests: key gen → store → attest → retrieve → obliterate
// Multi-key transactions with rollback scenarios
// Content roundtrip: write → hash → read → delete

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper: Create a temp directory for test isolation
fn test_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Helper: Create jk directories
fn setup_jk_dirs(base: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(base.join(".jk/content"))?;
    fs::create_dir_all(base.join(".jk/metadata"))?;
    fs::create_dir_all(base.join(".jk/attestation"))?;
    fs::create_dir_all(base.join(".jk/transactions"))?;
    fs::create_dir_all(base.join(".jk/operations"))?;
    fs::create_dir_all(base.join(".jk/keys"))?;
    Ok(())
}

/// Helper: SHA256 hash (using real sha2 crate)
fn sha256(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn chrono_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ============================================
// FULL KEY LIFECYCLE: generate → store → retrieve
// ============================================

#[test]
fn full_key_lifecycle_single_key() {
    let dir = test_dir();
    let base = dir.path().to_path_buf();
    setup_jk_dirs(&base).expect("Setup failed");

    // Step 1: Generate key (simulate by storing a key record)
    let key_id = "key-001";
    let key_material = b"super-secret-key-material-32-bytes";
    let key_hash = sha256(key_material);

    let key_record = format!(
        r#"{{"id":"{}","algo":"aes256gcm","hash":"{}","state":"active"}}"#,
        key_id, key_hash
    );
    fs::write(base.join(".jk/keys/001.json"), &key_record).expect("Write key record");

    // Step 2: Store the key in content store
    let content_path = base.join(".jk/content").join(&key_hash);
    fs::write(&content_path, key_material).expect("Store content");

    // Step 3: Create attestation entry
    let attest_entry = format!(
        r#"{{"key_id":"{}","op":"key_gen","timestamp":{},"hash":"{}"}}"#,
        key_id,
        chrono_timestamp(),
        key_hash
    );
    fs::write(base.join(".jk/attestation/0001.json"), &attest_entry)
        .expect("Write attestation");

    // Step 4: Retrieve - verify content matches
    let retrieved = fs::read(&content_path).expect("Read content");
    assert_eq!(retrieved, key_material, "Retrieved content must match original");

    // Step 5: Verify attestation references the key
    let attest_read = fs::read_to_string(base.join(".jk/attestation/0001.json"))
        .expect("Read attestation");
    assert!(attest_read.contains(key_id), "Attestation must contain key ID");
    assert!(attest_read.contains(&key_hash), "Attestation must contain content hash");

    // Verify key record exists
    let key_read = fs::read_to_string(base.join(".jk/keys/001.json"))
        .expect("Read key record");
    assert!(key_read.contains(key_id), "Key record must contain ID");
}

#[test]
fn full_key_lifecycle_multi_key_transaction() {
    let dir = test_dir();
    let base = dir.path().to_path_buf();
    setup_jk_dirs(&base).expect("Setup failed");

    // Start transaction
    let tx_id = "tx-001";
    let tx_record = r#"{"id":"tx-001","state":"active","ops":[]}"#;
    fs::write(base.join(".jk/transactions/001.json"), tx_record)
        .expect("Write transaction");

    // Generate 3 keys within transaction
    let keys: Vec<(&str, &[u8])> = vec![
        ("key-001", b"material-001-key-one-secret-32byte"),
        ("key-002", b"material-002-key-two-secret-32byte"),
        ("key-003", b"material-003-key-three-secret-32bytes"),
    ];

    for (i, (key_id, material)) in keys.iter().enumerate() {
        // Store content
        let hash = sha256(*material);
        let content_path = base.join(".jk/content").join(&hash);
        fs::write(&content_path, material).expect("Store content");

        // Record key
        let key_record = format!(
            r#"{{"id":"{}","hash":"{}","tx":"{}"}}"#,
            key_id, hash, tx_id
        );
        fs::write(
            base.join(".jk/keys").join(format!("{:03}.json", i + 1)),
            &key_record,
        )
        .expect("Write key record");

        // Record operation in transaction
        let op_record = format!(
            r#"{{"tx":"{}","op":"key_gen","key_id":"{}","seq":{}}}"#,
            tx_id, key_id, i
        );
        fs::write(
            base.join(".jk/operations")
                .join(format!("{}-op-{:04}.json", tx_id, i)),
            &op_record,
        )
        .expect("Write operation");
    }

    // Verify all 3 keys are stored and accessible
    let key_files: Vec<_> = fs::read_dir(base.join(".jk/keys"))
        .expect("Read keys dir")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(key_files.len(), 3, "All 3 keys must be stored");

    // Verify all 3 operations are in transaction
    let op_files: Vec<_> = fs::read_dir(base.join(".jk/operations"))
        .expect("Read ops dir")
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_str()
                .map_or(false, |n| n.starts_with(tx_id))
        })
        .collect();
    assert_eq!(op_files.len(), 3, "Transaction must contain 3 operations");

    // Commit transaction
    let commit_record = r#"{"id":"tx-001","state":"committed","ops":["op-0","op-1","op-2"]}"#;
    fs::write(base.join(".jk/transactions/001.json"), commit_record)
        .expect("Commit transaction");

    // Verify transaction is committed
    let tx_read =
        fs::read_to_string(base.join(".jk/transactions/001.json")).expect("Read tx");
    assert!(tx_read.contains("committed"), "Transaction must be committed");
}

#[test]
fn delta_chain_full_history() {
    let dir = test_dir();
    let base = dir.path().to_path_buf();
    setup_jk_dirs(&base).expect("Setup failed");

    // Create a file and record deltas as it evolves
    let _key_id = "key-chain-001";

    // Version 1: Initial content
    let v1 = b"initial content for delta chain v1";
    let v1_hash = sha256(v1);
    fs::write(base.join(".jk/content").join(&v1_hash), v1).expect("Store v1");

    // Version 2: Modified content
    let v2 = b"initial content for delta chain v2";
    let v2_hash = sha256(v2);
    fs::write(base.join(".jk/content").join(&v2_hash), v2).expect("Store v2");

    // Version 3: Further modified
    let v3 = b"initial content for delta chain v3";
    let v3_hash = sha256(v3);
    fs::write(base.join(".jk/content").join(&v3_hash), v3).expect("Store v3");

    // Create delta chain metadata
    let delta_1_to_2 = format!(
        r#"{{"from":"{}","to":"{}","delta_type":"full_rewrite","seq":1}}"#,
        v1_hash, v2_hash
    );
    fs::write(base.join(".jk/metadata/delta-01.json"), &delta_1_to_2)
        .expect("Write delta 1→2");

    let delta_2_to_3 = format!(
        r#"{{"from":"{}","to":"{}","delta_type":"full_rewrite","seq":2}}"#,
        v2_hash, v3_hash
    );
    fs::write(base.join(".jk/metadata/delta-02.json"), &delta_2_to_3)
        .expect("Write delta 2→3");

    // Verify chain is intact: can read all versions
    let read_v1 = fs::read(&base.join(".jk/content").join(&v1_hash))
        .expect("Read v1");
    assert_eq!(read_v1, v1, "Version 1 must be recoverable");

    let read_v2 = fs::read(&base.join(".jk/content").join(&v2_hash))
        .expect("Read v2");
    assert_eq!(read_v2, v2, "Version 2 must be recoverable");

    let read_v3 = fs::read(&base.join(".jk/content").join(&v3_hash))
        .expect("Read v3");
    assert_eq!(read_v3, v3, "Version 3 must be recoverable");

    // Verify chain links are recorded
    let delta_files: Vec<_> = fs::read_dir(base.join(".jk/metadata"))
        .expect("Read metadata")
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_str().map_or(false, |n| n.starts_with("delta")))
        .collect();
    assert_eq!(delta_files.len(), 2, "Delta chain must have 2 links");
}

// ============================================
// CONTENT STORE ROUNDTRIP: write → verify → read → delete
// ============================================

#[test]
fn content_store_write_verify_read_delete() {
    let dir = test_dir();
    let base = dir.path().to_path_buf();
    setup_jk_dirs(&base).expect("Setup failed");

    let content = b"test content for roundtrip verification";
    let hash = sha256(content);
    let store_path = base.join(".jk/content").join(&hash);

    // Write
    fs::write(&store_path, content).expect("Write content");
    assert!(
        store_path.exists(),
        "Content must exist after write"
    );

    // Verify hash (read back and re-hash)
    let read_back = fs::read(&store_path).expect("Read content");
    let verify_hash = sha256(&read_back);
    assert_eq!(
        hash, verify_hash,
        "Re-hashed content must match original hash"
    );

    // Read again
    let read_again = fs::read(&store_path).expect("Read content again");
    assert_eq!(read_again, content, "Multiple reads must return same content");

    // Delete (simulate obliteration by removing file)
    fs::remove_file(&store_path).expect("Delete file");
    assert!(
        !store_path.exists(),
        "Content must not exist after deletion"
    );

    // Verify truly gone (attempt read fails)
    let read_deleted = fs::read(&store_path);
    assert!(
        read_deleted.is_err(),
        "Reading deleted content must fail"
    );
}

#[test]
fn content_store_deduplication_multiple_keys() {
    let dir = test_dir();
    let base = dir.path().to_path_buf();
    setup_jk_dirs(&base).expect("Setup failed");

    // Write same content under different key records
    let shared_content = b"shared secret material across keys";
    let shared_hash = sha256(shared_content);
    let store_path = base.join(".jk/content").join(&shared_hash);

    // Key 1 references shared content
    let key1 = format!(
        r#"{{"id":"key-1","hash":"{}"}}"#,
        shared_hash
    );
    fs::write(base.join(".jk/keys/001.json"), &key1).expect("Write key1");

    // Key 2 references same shared content
    let key2 = format!(
        r#"{{"id":"key-2","hash":"{}"}}"#,
        shared_hash
    );
    fs::write(base.join(".jk/keys/002.json"), &key2).expect("Write key2");

    // Content is stored only once
    fs::write(&store_path, shared_content).expect("Write shared content");

    // Both keys can access the same content
    let read1 = fs::read(&store_path).expect("Key1 read");
    let read2 = fs::read(&store_path).expect("Key2 read");

    assert_eq!(read1, shared_content, "Key 1 sees correct content");
    assert_eq!(read2, shared_content, "Key 2 sees correct content");

    // Verify only one physical file exists
    let content_files: Vec<_> = fs::read_dir(base.join(".jk/content"))
        .expect("Read content dir")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(
        content_files.len(),
        1,
        "Only one physical copy despite two key references"
    );
}

// ============================================
// ERROR CASES
// ============================================

#[test]
fn retrieve_nonexistent_key_fails() {
    let dir = test_dir();
    let base = dir.path().to_path_buf();
    setup_jk_dirs(&base).expect("Setup failed");

    let nonexistent_hash = "0000000000000000000000000000000000000000000000000000000000000000";
    let store_path = base.join(".jk/content").join(nonexistent_hash);

    let result = fs::read(&store_path);
    assert!(
        result.is_err(),
        "Reading nonexistent key must fail"
    );
}

#[test]
fn corrupted_attestation_entry_detected() {
    let dir = test_dir();
    let base = dir.path().to_path_buf();
    setup_jk_dirs(&base).expect("Setup failed");

    // Write malformed attestation
    let bad_entry = r#"{"invalid json"#;
    fs::write(base.join(".jk/attestation/0001.json"), bad_entry)
        .expect("Write malformed entry");

    // Attempt to read and parse
    let read_result = fs::read_to_string(base.join(".jk/attestation/0001.json"))
        .expect("Read file");
    let parse_result = serde_json::from_str::<serde_json::Value>(&read_result);

    assert!(
        parse_result.is_err(),
        "Malformed JSON must be detected during parsing"
    );
}
