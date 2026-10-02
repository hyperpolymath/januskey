// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// Obliteration must leave no recoverable trace of a path in `.januskey/`:
// no content-store blob (unless shared with another path), no operation-log
// entry, no undo. Every "absent" assertion below is preceded by the same
// probe returning "present" on the same store (positive control), because
// blobs are gzip-compressed and a naive byte grep would pass vacuously.

use assert_cmd::Command;
use flate2::read::GzDecoder;
use januskey::obliteration::{obliterate_path, ObliterationManager, OBLITERATION_LOG_FILE};
use januskey::{ContentHash, FileOperation, JanusKey, OperationExecutor};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const V0: &str = "OBLIT-MARKER-V0-c41f9e2a-original-plaintext";
const V1: &str = "OBLIT-MARKER-V1-7d03b6aa-first-edit-plaintext";
const V2: &str = "OBLIT-MARKER-V2-e85a1f30-second-edit-plaintext";

/// Read every file under `.januskey/`, gunzipping `.gz` files (falling back
/// to raw bytes if decompression fails), and return the markers found.
fn markers_in_store(root: &Path, markers: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for entry in walkdir(root.join(".januskey")) {
        let raw = fs::read(&entry).unwrap();
        let mut bytes = raw.clone();
        if entry.extension().is_some_and(|e| e == "gz") {
            let mut out = Vec::new();
            if GzDecoder::new(raw.as_slice()).read_to_end(&mut out).is_ok() {
                bytes = out;
            }
        }
        let text = String::from_utf8_lossy(&bytes);
        let raw_text = String::from_utf8_lossy(&raw);
        for m in markers {
            if text.contains(m) || raw_text.contains(m) {
                found.push(format!("{} in {}", m, entry.display()));
            }
        }
    }
    found
}

/// Recursively list regular files under `dir`.
fn walkdir(dir: PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out
}

/// Open the obliteration manager at its canonical location in the store.
fn manager(jk: &JanusKey) -> ObliterationManager {
    ObliterationManager::new(jk.root.join(".januskey").join(OBLITERATION_LOG_FILE)).unwrap()
}

/// Create `path` with `v0`, then modify it to each of `edits` in turn, via
/// the library operations. Returns the id of the last operation.
fn create_and_edit(jk: &mut JanusKey, path: &Path, v0: &str, edits: &[&str]) -> String {
    let mut ex = OperationExecutor::new(&jk.content_store, &mut jk.metadata_store);
    let mut last = ex
        .execute(FileOperation::Create {
            path: path.to_path_buf(),
            content: v0.as_bytes().to_vec(),
        })
        .unwrap();
    for e in edits {
        last = ex
            .execute(FileOperation::Modify {
                path: path.to_path_buf(),
                new_content: e.as_bytes().to_vec(),
            })
            .unwrap();
    }
    last.id
}

/// Every stored (blob-backed) hash recorded for entries mentioning `path`.
fn stored_hashes_for(jk: &JanusKey, path: &Path) -> Vec<ContentHash> {
    let mut v: Vec<ContentHash> = jk
        .metadata_store
        .operations()
        .iter()
        .filter(|op| op.path == path || op.path_secondary.as_deref() == Some(path))
        .flat_map(|op| op.content_hash.iter().chain(op.new_content_hash.iter()))
        .filter(|h| jk.content_store.exists(h))
        .cloned()
        .collect();
    v.dedup();
    v
}

/// Create + two edits, then obliterate: blobs, log entries, plaintext and undo are all gone.
#[test]
fn obliterate_scrubs_blobs_log_and_undo() {
    let tmp = TempDir::new().unwrap();
    let mut jk = JanusKey::init(tmp.path()).unwrap();
    assert!(jk.config.compression, "test assumes gzip blobs (default)");
    let file = jk.root.join("secret.txt");
    let last_id = create_and_edit(&mut jk, &file, V0, &[V1, V2]);

    // ---- Positive control: the probes see the prior plaintext. ----
    let plaintext: HashMap<ContentHash, &str> = [V0, V1, V2]
        .iter()
        .map(|v| (ContentHash::from_string(v), *v))
        .collect();
    let hashes = stored_hashes_for(&jk, &file);
    assert!(
        hashes.contains(&ContentHash::from_string(V0))
            && hashes.contains(&ContentHash::from_string(V1)),
        "expected blobs for V0 and V1, got {hashes:?}"
    );
    for h in &hashes {
        let p = jk.content_store.content_path(h);
        assert!(p.exists(), "blob {} missing before obliterate", p.display());
        assert!(p.extension().is_some_and(|e| e == "gz"));
        let got = jk.content_store.retrieve(h).unwrap();
        assert_eq!(got, plaintext[h].as_bytes());
    }
    let before = markers_in_store(&jk.root, &[V0, V1]);
    assert_eq!(before.len(), 2, "walk must find V0 and V1: {before:?}");
    assert!(jk
        .metadata_store
        .operations()
        .iter()
        .any(|op| op.path == file));
    let blob_paths: Vec<PathBuf> = hashes
        .iter()
        .map(|h| jk.content_store.content_path(h))
        .collect();

    // ---- Obliterate (relative spelling, to exercise path normalisation). ----
    let mut mgr = manager(&jk);
    let report = obliterate_path(
        &mut jk,
        &mut mgr,
        Path::new("secret.txt"),
        Some("test".into()),
        Some("GDPR Article 17".into()),
    )
    .unwrap();
    assert!(report.file_proof.is_some());
    assert_eq!(report.blob_records.len(), hashes.len());
    assert!(report.retained_shared.is_empty());
    assert_eq!(report.purged_operation_ids.len(), 3);

    // ---- Nothing recoverable. ----
    assert!(!file.exists());
    for (h, p) in hashes.iter().zip(&blob_paths) {
        assert!(!p.exists(), "blob {} survived", p.display());
        assert!(jk.content_store.retrieve(h).is_err());
    }
    assert!(jk
        .metadata_store
        .operations()
        .iter()
        .all(|op| op.path != file && op.path_secondary.as_deref() != Some(&*file)));
    let after = markers_in_store(&jk.root, &[V0, V1, V2, "secret.txt"]);
    assert!(after.is_empty(), "plaintext/path survived: {after:?}");
    // Reopen from disk: the purge was persisted.
    let mut jk2 = JanusKey::open(tmp.path()).unwrap();
    assert_eq!(jk2.metadata_store.count(), 0);
    let mut ex = OperationExecutor::new(&jk2.content_store, &mut jk2.metadata_store);
    assert!(
        ex.undo(&last_id).is_err(),
        "undo must fail after obliterate"
    );
    // The shreds are recorded (hashes only, never content).
    assert_eq!(manager(&jk2).count(), 1 + hashes.len());
}

/// A blob deduplicated with another path survives, and that path can still undo.
#[test]
fn obliterate_keeps_blob_shared_with_another_path() {
    let tmp = TempDir::new().unwrap();
    let mut jk = JanusKey::init(tmp.path()).unwrap();
    let shared = "OBLIT-SHARED-9b2e44c1-identical-bytes-in-two-files";
    let a1 = "OBLIT-A1-03f7c2d8-only-in-a";
    let a2 = "OBLIT-A2-5e1a9b7f-only-in-a";
    let b1 = "OBLIT-B1-c6d2e0a4-only-in-b";
    let a = jk.root.join("a.txt");
    let b = jk.root.join("b.txt");
    create_and_edit(&mut jk, &a, shared, &[a1, a2]);
    let b_last = create_and_edit(&mut jk, &b, shared, &[b1]);

    let shared_h = ContentHash::from_string(shared);
    let a1_h = ContentHash::from_string(a1);
    // Positive control: both blobs present and readable.
    assert_eq!(
        jk.content_store.retrieve(&shared_h).unwrap(),
        shared.as_bytes()
    );
    assert_eq!(jk.content_store.retrieve(&a1_h).unwrap(), a1.as_bytes());

    let mut mgr = manager(&jk);
    let report = obliterate_path(&mut jk, &mut mgr, &a, None, None).unwrap();
    assert!(report.retained_shared.contains(&shared_h));

    // a's unshared blob is gone; the shared one is intact.
    assert!(!jk.content_store.content_path(&a1_h).exists());
    assert!(jk.content_store.retrieve(&a1_h).is_err());
    assert_eq!(
        jk.content_store.retrieve(&shared_h).unwrap(),
        shared.as_bytes()
    );
    assert!(markers_in_store(&jk.root, &[a1, a2, "a.txt"]).is_empty());

    // b's history is untouched and its undo still restores the shared bytes.
    assert!(b.exists());
    let mut ex = OperationExecutor::new(&jk.content_store, &mut jk.metadata_store);
    ex.undo(&b_last).unwrap();
    assert_eq!(fs::read_to_string(&b).unwrap(), shared);
}

/// `delete` then `obliterate`: no working file, but the stored history is still scrubbed.
#[test]
fn obliterate_after_delete_scrubs_history() {
    let tmp = TempDir::new().unwrap();
    let mut jk = JanusKey::init(tmp.path()).unwrap();
    let file = jk.root.join("gone.txt");
    create_and_edit(&mut jk, &file, V0, &[V1]);
    {
        let mut ex = OperationExecutor::new(&jk.content_store, &mut jk.metadata_store);
        ex.execute(FileOperation::Delete { path: file.clone() })
            .unwrap();
    }
    assert!(!file.exists());
    // Positive control: deleted content is still recoverable from the store.
    assert_eq!(markers_in_store(&jk.root, &[V0, V1]).len(), 2);

    let mut mgr = manager(&jk);
    let report = obliterate_path(&mut jk, &mut mgr, &file, None, None).unwrap();
    assert!(report.file_proof.is_none());
    assert!(!report.blob_records.is_empty());
    assert!(markers_in_store(&jk.root, &[V0, V1, "gone.txt"]).is_empty());
    assert_eq!(jk.metadata_store.count(), 0);
}

/// A path with neither a working file nor history is reported as not found.
#[test]
fn obliterate_untracked_absent_path_errs() {
    let tmp = TempDir::new().unwrap();
    let mut jk = JanusKey::init(tmp.path()).unwrap();
    let mut mgr = manager(&jk);
    assert!(obliterate_path(&mut jk, &mut mgr, Path::new("nope.txt"), None, None).is_err());
}

/// End to end through the `jk` binary: modify, obliterate, then undo finds nothing.
#[test]
fn cli_obliterate_scrubs_store_and_undo_has_nothing() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    Command::cargo_bin("jk")
        .unwrap()
        .arg("init")
        .arg(dir)
        .assert()
        .success();
    fs::write(dir.join("f.txt"), V0).unwrap();
    Command::cargo_bin("jk")
        .unwrap()
        .args(["-C"])
        .arg(dir)
        .args(["-y", "modify", "s/V0/VX/", "f.txt"])
        .assert()
        .success();
    // Positive control: the modify stored V0 as a gzip blob.
    assert_eq!(markers_in_store(dir, &[V0]).len(), 1);

    Command::cargo_bin("jk")
        .unwrap()
        .args(["-C"])
        .arg(dir)
        .args(["-y", "obliterate", "f.txt"])
        .assert()
        .success();
    assert!(!dir.join("f.txt").exists());
    assert!(markers_in_store(dir, &[V0, "OBLIT-MARKER-VX", "f.txt"]).is_empty());

    let out = Command::cargo_bin("jk")
        .unwrap()
        .args(["-C"])
        .arg(dir)
        .arg("undo")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(
        String::from_utf8_lossy(&out).contains("Nothing to undo"),
        "undo output: {}",
        String::from_utf8_lossy(&out)
    );
}
