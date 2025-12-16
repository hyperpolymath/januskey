// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// Content-Addressed Storage with SHA256 hashing
// Provides deduplication and integrity verification

use crate::error::{JanusError, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// SHA256 content hash
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

    /// Create hash from string
    pub fn from_str(s: &str) -> Self {
        Self::from_bytes(s.as_bytes())
    }

    /// Get the raw hash portion (without prefix)
    pub fn raw_hash(&self) -> &str {
        self.0.strip_prefix("sha256:").unwrap_or(&self.0)
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

/// Content-addressed storage for file content
pub struct ContentStore {
    /// Root directory for content storage
    root: PathBuf,
    /// Whether to compress stored content
    compression: bool,
}

impl ContentStore {
    /// Create or open a content store
    pub fn new(root: PathBuf, compression: bool) -> Result<Self> {
        fs::create_dir_all(&root)?;
        Ok(Self { root, compression })
    }

    /// Get path for a content hash
    fn content_path(&self, hash: &ContentHash) -> PathBuf {
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

    /// Store content and return its hash
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

    /// Store content from a file
    pub fn store_file(&self, file_path: &Path) -> Result<ContentHash> {
        let content = fs::read(file_path)?;
        self.store(&content)
    }

    /// Retrieve content by hash
    pub fn retrieve(&self, hash: &ContentHash) -> Result<Vec<u8>> {
        let path = self.content_path(hash);

        if !path.exists() {
            return Err(JanusError::FileNotFound(hash.to_string()));
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
            return Err(JanusError::ContentIntegrityError {
                expected: hash.to_string(),
                actual: actual.to_string(),
            });
        }

        Ok(content)
    }

    /// Check if content exists
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

    /// Get total size of content store
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
        assert!(hash.verify(content));
        assert!(!hash.verify(b"different content"));
    }

    #[test]
    fn test_store_and_retrieve() {
        let tmp = TempDir::new().unwrap();
        let store = ContentStore::new(tmp.path().to_path_buf(), false).unwrap();

        let content = b"test content";
        let hash = store.store(content).unwrap();

        let retrieved = store.retrieve(&hash).unwrap();
        assert_eq!(content.to_vec(), retrieved);
    }

    #[test]
    fn test_store_compressed() {
        let tmp = TempDir::new().unwrap();
        let store = ContentStore::new(tmp.path().to_path_buf(), true).unwrap();

        let content = b"test content that should compress well when repeated ".repeat(100);
        let hash = store.store(&content).unwrap();

        let retrieved = store.retrieve(&hash).unwrap();
        assert_eq!(content, retrieved.as_slice());
    }

    #[test]
    fn test_deduplication() {
        let tmp = TempDir::new().unwrap();
        let store = ContentStore::new(tmp.path().to_path_buf(), false).unwrap();

        let content = b"duplicate content";
        let hash1 = store.store(content).unwrap();
        let hash2 = store.store(content).unwrap();

        assert_eq!(hash1, hash2);
        assert_eq!(store.count().unwrap(), 1);
    }
}
