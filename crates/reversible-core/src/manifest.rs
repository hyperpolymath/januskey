// SPDX-License-Identifier: MIT OR PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// A2ML Manifest Emitter
//
// Generates A2ML (Attestation & Audit Markup Language) manifests from
// operation logs. These manifests can be consumed by ochrance's
// VerifiedSubsystem::verify to produce VerificationProof witnesses.
//
// Format follows ochrance-core/Ochrance/A2ML/Types.idr:
//   Manifest { manifestData, refs, attestation, policy }

use crate::content_store::ContentHash;
use crate::metadata::{MetadataStore, OperationMetadata};
use sha2::{Digest, Sha256};

/// Generates A2ML manifests from operation metadata.
///
/// The manifest contains:
/// - Manifest header (version, subsystem name, timestamp)
/// - Refs: one entry per operation, keyed by operation ID with content hash
/// - Merkle root: SHA256 hash chain of all operation hashes in order
///
/// Ochrance can parse this manifest and verify:
/// - Lax: manifest is well-formed
/// - Checked: all content hashes match the content store
/// - Attested: manifest is signed (requires external signing step)
pub struct ManifestEmitter;

impl ManifestEmitter {
    /// Generate an A2ML manifest from a metadata store.
    ///
    /// The manifest is a text format compatible with ochrance's A2ML parser.
    pub fn generate(subsystem: &str, store: &MetadataStore) -> String {
        let operations = store.operations();
        let timestamp = chrono::Utc::now().to_rfc3339();
        let merkle_root = Self::compute_merkle_root(operations);

        let mut manifest = String::new();

        // Header
        manifest.push_str("@manifest\n");
        manifest.push_str(&format!("  version = \"1.0\"\n"));
        manifest.push_str(&format!("  subsystem = \"{}\"\n", subsystem));
        manifest.push_str(&format!("  timestamp = \"{}\"\n", timestamp));
        manifest.push_str(&format!(
            "  merkle-root = \"sha256:{}\"\n",
            hex::encode(&merkle_root)
        ));
        manifest.push('\n');

        // Refs — one per operation
        manifest.push_str("@refs\n");
        for op in operations {
            let hash = op
                .content_hash
                .as_ref()
                .map(|h| h.0.clone())
                .unwrap_or_else(|| {
                    // For operations without content (mkdir, chmod), hash the metadata
                    let meta_json = serde_json::to_string(op).unwrap_or_default();
                    ContentHash::from_bytes(meta_json.as_bytes()).0
                });

            manifest.push_str(&format!(
                "  {} = {{ type = \"{}\", path = \"{}\", hash = \"{}\", undone = {} }}\n",
                op.id,
                op.op_type,
                op.path.display(),
                hash,
                op.undone,
            ));
        }
        manifest.push('\n');

        // Policy
        manifest.push_str("@policy\n");
        manifest.push_str("  mode = \"checked\"\n");
        manifest.push_str("  require-sig = false\n");
        manifest.push('\n');

        manifest
    }

    /// Compute a Merkle root hash from operation hashes.
    ///
    /// Uses a sequential hash chain (not a balanced tree) for simplicity.
    /// Each step: `H(n) = SHA256(H(n-1) || op_hash(n))`
    ///
    /// For balanced Merkle tree verification, ochrance's Idris2
    /// implementation provides height-indexed trees with compile-time proofs.
    fn compute_merkle_root(operations: &[OperationMetadata]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"reversible-core:merkle:v1");

        for op in operations {
            let op_hash = match &op.content_hash {
                Some(h) => h.raw_hash().as_bytes().to_vec(),
                None => {
                    let meta_json = serde_json::to_string(op).unwrap_or_default();
                    let h = Sha256::digest(meta_json.as_bytes());
                    h.to_vec()
                }
            };

            let current = hasher.finalize_reset();
            hasher.update(&current);
            hasher.update(&op_hash);
        }

        hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::OperationType;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_generate_manifest() {
        let tmp = TempDir::new().expect("TODO: handle error");
        let mut store =
            MetadataStore::new(tmp.path().join("metadata.json")).expect("TODO: handle error");

        let op = OperationMetadata::new(
            OperationType::Delete,
            PathBuf::from("/test/file.txt"),
        )
        .with_content_hash(ContentHash::from_bytes(b"file content"));

        store.append(op).expect("TODO: handle error");

        let manifest = ManifestEmitter::generate("januskey", &store);

        assert!(manifest.contains("@manifest"));
        assert!(manifest.contains("subsystem = \"januskey\""));
        assert!(manifest.contains("@refs"));
        assert!(manifest.contains("DELETE"));
        assert!(manifest.contains("@policy"));
        assert!(manifest.contains("merkle-root"));
    }

    #[test]
    fn test_empty_manifest() {
        let tmp = TempDir::new().expect("TODO: handle error");
        let store =
            MetadataStore::new(tmp.path().join("metadata.json")).expect("TODO: handle error");

        let manifest = ManifestEmitter::generate("test", &store);
        assert!(manifest.contains("@manifest"));
        assert!(manifest.contains("@refs"));
    }

    #[test]
    fn test_merkle_root_deterministic() {
        let tmp = TempDir::new().expect("TODO: handle error");
        let mut store =
            MetadataStore::new(tmp.path().join("metadata.json")).expect("TODO: handle error");

        let op = OperationMetadata::new(
            OperationType::Create,
            PathBuf::from("/a.txt"),
        );
        store.append(op).expect("TODO: handle error");

        let m1 = ManifestEmitter::generate("test", &store);
        let m2 = ManifestEmitter::generate("test", &store);

        // Merkle root should be the same for the same operations
        // (timestamps in manifest header will differ, but merkle-root won't)
        let extract_root = |m: &str| -> String {
            m.lines()
                .find(|l| l.contains("merkle-root"))
                .unwrap_or("")
                .to_string()
        };
        assert_eq!(extract_root(&m1), extract_root(&m2));
    }
}
