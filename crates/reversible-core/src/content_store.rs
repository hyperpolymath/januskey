// SPDX-License-Identifier: MIT OR PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// Content-Addressed Storage with SHA256 hashing
// Provides deduplication and integrity verification

use crate::error::{ReversibleError, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// SHA256 content hash for content-addressed storage.
///
/// Format: `sha256:<hex-encoded-hash>`
///
/// Corresponds to ochrance's `Hash` type with `algorithm = SHA256`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ContentHash(pub String);

impl ContentHash {
    /// Create hash from content bytes
    pub fn from_bytes(content: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = hasher.finalize();
        Self(format!("sha256:{}", hex::encode(hash)))
    }

    /// Create hash from string content
    pub fn from_string(s: &str) -> Self {
        Self::from_bytes(s.as_bytes())
    }

    /// Get the raw hash portion (without algorithm prefix)
    pub fn raw_hash(&self) -> &str {
        self.0.strip_prefix("sha256:").unwrap_or(&self.0)
    }

    /// Get the algorithm name
    pub fn algorithm(&self) -> &str {
        "sha256"
    }

    /// Verify content matches this hash
    pub fn verify(&self, content: &[u8]) -> bool {
        let computed = Self::from_bytes(content);
        self.0 == computed.0
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Content-addressed storage for file content.
///
/// Stores content by SHA256 hash with optional gzip compression.
/// Automatic deduplication: identical content is stored once.
///
/// This is the shared storage backend that both januskey-cli and
/// valence-shell use for reversible operation data.
pub struct ContentStore {
    /// Root directory for content storage
    root: PathBuf,
    /// Whether to compress stored content
    compression: bool,
}

impl ContentStore {
    /// Create or open a content store at the given path
    pub fn new(root: PathBuf, compression: bool) -> Result<Self> {
        fs::create_dir_all(&root)?;
        Ok(Self { root, compression })
    }

    /// Get the root path of this content store
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get filesystem path for a content hash.
    ///
    /// Uses a 2-char prefix directory for distribution (git-style layout).
    pub fn content_path(&self, hash: &ContentHash) -> PathBuf {
        let raw = hash.raw_hash();
        // Use first 2 chars as directory for distribution
        let (dir, file) = raw.split_at(2.min(raw.len()));
        let mut path = self.root.join(dir);
        if self.compression {
            path = path.join(format!("{}.gz", file));
        } else {
            path = path.join(file);
        }
        path
    }

    /// Store content and return its hash.
    ///
    /// If content with the same hash already exists, this is a no-op
    /// (deduplication).
    pub fn store(&self, content: &[u8]) -> Result<ContentHash> {
        let hash = ContentHash::from_bytes(content);
        let path = self.content_path(&hash);

        // Skip if already stored (deduplication)
        if path.exists() {
            return Ok(hash);
        }

        // Create parent directory
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write content (optionally compressed)
        if self.compression {
            let file = File::create(&path)?;
            let mut encoder = GzEncoder::new(file, Compression::default());
            encoder.write_all(content)?;
            encoder.finish()?;
        } else {
            fs::write(&path, content)?;
        }

        Ok(hash)
    }

    /// Store content from a file path
    pub fn store_file(&self, file_path: &Path) -> Result<ContentHash> {
        let content = fs::read(file_path)?;
        self.store(&content)
    }

    /// Retrieve content by hash, verifying integrity on read
    pub fn retrieve(&self, hash: &ContentHash) -> Result<Vec<u8>> {
        let path = self.content_path(hash);

        if !path.exists() {
            return Err(ReversibleError::FileNotFound(hash.to_string()));
        }

        let content = if self.compression {
            let file = File::open(&path)?;
            let mut decoder = GzDecoder::new(file);
            let mut content = Vec::new();
            decoder.read_to_end(&mut content)?;
            content
        } else {
            fs::read(&path)?
        };

        // Verify integrity
        if !hash.verify(&content) {
            let actual = ContentHash::from_bytes(&content);
            return Err(ReversibleError::ContentIntegrityError {
                expected: hash.to_string(),
                actual: actual.to_string(),
            });
        }

        Ok(content)
    }

    /// Check if content exists in the store
    pub fn exists(&self, hash: &ContentHash) -> bool {
        self.content_path(hash).exists()
    }

    /// Delete content by hash (for garbage collection)
    pub fn delete(&self, hash: &ContentHash) -> Result<()> {
        let path = self.content_path(hash);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Get total size of content store in bytes
    pub fn total_size(&self) -> Result<u64> {
        let mut size = 0;
        for entry in walkdir::WalkDir::new(&self.root).into_iter().flatten() {
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    size += metadata.len();
                }
            }
        }
        Ok(size)
    }

    /// Count number of stored content blobs
    pub fn count(&self) -> Result<usize> {
        let mut count = 0;
        for entry in walkdir::WalkDir::new(&self.root).into_iter().flatten() {
            if entry.file_type().is_file() {
                count += 1;
            }
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_content_hash() {
        let content = b"hello world";
        let hash = ContentHash::from_bytes(content);
        assert!(hash.0.starts_with("sha256:"));
        assert_eq!(hash.algorithm(), "sha256");
        assert!(hash.verify(content));
        assert!(!hash.verify(b"different content"));
    }

    #[test]
    fn test_store_and_retrieve() {
        let tmp = TempDir::new().expect("TODO: handle error");
        let store = ContentStore::new(tmp.path().to_path_buf(), false).expect("TODO: handle error");

        let content = b"test content";
        let hash = store.store(content).expect("TODO: handle error");

        let retrieved = store.retrieve(&hash).expect("TODO: handle error");
        assert_eq!(content.to_vec(), retrieved);
    }

    #[test]
    fn test_store_compressed() {
        let tmp = TempDir::new().expect("TODO: handle error");
        let store = ContentStore::new(tmp.path().to_path_buf(), true).expect("TODO: handle error");

        let content = b"test content that should compress well when repeated ".repeat(100);
        let hash = store.store(&content).expect("TODO: handle error");

        let retrieved = store.retrieve(&hash).expect("TODO: handle error");
        assert_eq!(content, retrieved.as_slice());
    }

    #[test]
    fn test_deduplication() {
        let tmp = TempDir::new().expect("TODO: handle error");
        let store = ContentStore::new(tmp.path().to_path_buf(), false).expect("TODO: handle error");

        let content = b"duplicate content";
        let hash1 = store.store(content).expect("TODO: handle error");
        let hash2 = store.store(content).expect("TODO: handle error");

        assert_eq!(hash1, hash2);
        assert_eq!(store.count().expect("TODO: handle error"), 1);
    }
}
