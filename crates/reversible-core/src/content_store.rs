// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// Content-Addressed Storage with SHA256 hashing
// Provides deduplication and integrity verification

use crate::error::{Result, ReversibleError};
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

    /// True when this is exactly `sha256:` followed by 64 lowercase hex digits,
    /// the only form `from_bytes` produces. Anything else is not a hash the
    /// store can hold, and must never be turned into a filesystem path.
    pub fn is_well_formed(&self) -> bool {
        self.0.strip_prefix("sha256:").is_some_and(|hex| {
            hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        })
    }

    /// Verify content matches this hash
    pub fn verify(&self, content: &[u8]) -> bool {
        let computed = Self::from_bytes(content);
        self.0 == computed.0
    }
}

/// Name prefix of in-flight blob writes. A file with this prefix is never a
/// stored blob; a crash can leave one behind.
const TMP_PREFIX: &str = ".tmp-";

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
    ///
    /// Fails closed: a hash that is not well formed (see
    /// [`ContentHash::is_well_formed`]) maps to a fixed path under the root that
    /// no blob ever occupies, never to a path derived from its text. The field
    /// is public and deserialised from metadata, so a hash string is untrusted
    /// input; `Path::join` with an absolute component would otherwise replace
    /// the root.
    pub fn content_path(&self, hash: &ContentHash) -> PathBuf {
        if !hash.is_well_formed() {
            return self.root.join("_invalid_hash");
        }
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
    /// If an intact blob with the same hash already exists, this is a no-op
    /// (deduplication). A blob that is present but damaged, for example
    /// truncated by a crash or by bit rot, is replaced rather than trusted.
    ///
    /// The blob is written to a temporary file in its own directory, flushed to
    /// disk, then renamed into place, so the final path only ever holds a
    /// complete blob. Deduplication reads the existing blob back to check it;
    /// that costs one read of a duplicate, and is what makes "store then
    /// retrieve returns the content" hold after a crash.
    pub fn store(&self, content: &[u8]) -> Result<ContentHash> {
        let hash = ContentHash::from_bytes(content);
        let path = self.content_path(&hash);

        // Skip if an intact copy is already stored (deduplication)
        if path.exists()
            && self
                .read_blob(&path)
                .is_ok_and(|stored| hash.verify(&stored))
        {
            return Ok(hash);
        }

        let parent = path.parent().ok_or_else(|| {
            ReversibleError::FileNotFound(format!("no parent directory for {}", hash))
        })?;
        fs::create_dir_all(parent)?;

        let tmp = parent.join(format!("{TMP_PREFIX}{}", uuid::Uuid::new_v4()));
        if let Err(e) = self.write_blob(&tmp, content) {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }
        if let Err(e) = fs::rename(&tmp, &path) {
            let _ = fs::remove_file(&tmp);
            return Err(e.into());
        }
        // Make the rename itself durable. Best effort: not every platform can
        // open a directory.
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }

        Ok(hash)
    }

    /// Write `content` (optionally compressed) to `path` and flush it to disk.
    fn write_blob(&self, path: &Path, content: &[u8]) -> Result<()> {
        let mut file = File::create(path)?;
        if self.compression {
            let mut encoder = GzEncoder::new(&mut file, Compression::default());
            encoder.write_all(content)?;
            encoder.finish()?;
        } else {
            file.write_all(content)?;
        }
        file.sync_all()?;
        Ok(())
    }

    /// Read a stored blob back (decompressing if needed), without verifying it.
    fn read_blob(&self, path: &Path) -> Result<Vec<u8>> {
        if self.compression {
            let file = File::open(path)?;
            let mut decoder = GzDecoder::new(file);
            let mut content = Vec::new();
            decoder.read_to_end(&mut content)?;
            Ok(content)
        } else {
            Ok(fs::read(path)?)
        }
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

        let content = self.read_blob(&path)?;

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

    /// Count number of stored content blobs (in-flight temp files are not blobs)
    pub fn count(&self) -> Result<usize> {
        let mut count = 0;
        for entry in walkdir::WalkDir::new(&self.root).into_iter().flatten() {
            if entry.file_type().is_file()
                && !entry.file_name().to_string_lossy().starts_with(TMP_PREFIX)
            {
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

    /// Hash strings that are not `sha256:` + 64 lowercase hex digits.
    fn malformed_hashes(victim: &Path) -> Vec<ContentHash> {
        let zeros = "0".repeat(64);
        vec![
            ContentHash(format!("sha256:ab{}", victim.display())), // absolute path after 2 chars
            ContentHash("sha256:ab/../../escape".to_string()),
            ContentHash("sha256:../..".to_string()),
            ContentHash("sha256:".to_string()),
            ContentHash(String::new()),
            ContentHash(format!("sha256:{}", "g".repeat(64))), // not hex
            ContentHash(format!("sha256:{}", zeros.to_uppercase().replace('0', "A"))), // uppercase
            ContentHash(format!("sha256:{}", "0".repeat(63))), // too short
            ContentHash(format!("sha256:{}", "0".repeat(65))), // too long
            ContentHash(zeros),                                // no algorithm prefix
            ContentHash(format!("sha256:{}\0", "0".repeat(63))), // NUL
        ]
    }

    #[test]
    fn malformed_hash_never_leaves_the_store_root() {
        for compression in [false, true] {
            let store_dir = TempDir::new().unwrap();
            let victim_dir = TempDir::new().unwrap();
            let victim = victim_dir.path().join("not-a-blob.txt");
            fs::write(&victim, b"outside the store").unwrap();
            let store = ContentStore::new(store_dir.path().to_path_buf(), compression).unwrap();

            for hash in malformed_hashes(&victim) {
                let path = store.content_path(&hash);
                assert!(
                    path.starts_with(store_dir.path()),
                    "{:?} escaped to {}",
                    hash.0,
                    path.display()
                );
                assert!(!store.exists(&hash), "{:?} must not exist", hash.0);
                store.delete(&hash).unwrap();
                assert!(
                    matches!(store.retrieve(&hash), Err(ReversibleError::FileNotFound(_))),
                    "{:?} must read as not found",
                    hash.0
                );
            }
            assert!(victim.exists(), "a file outside the store was deleted");
        }
    }

    #[test]
    fn well_formed_hash_keeps_the_existing_layout() {
        let tmp = TempDir::new().unwrap();
        let hash = ContentHash::from_bytes(b"layout");
        assert!(hash.is_well_formed());
        let raw = hash.raw_hash().to_string();

        let plain = ContentStore::new(tmp.path().join("plain"), false).unwrap();
        assert_eq!(
            plain.content_path(&hash),
            tmp.path().join("plain").join(&raw[..2]).join(&raw[2..])
        );
        let gz = ContentStore::new(tmp.path().join("gz"), true).unwrap();
        assert_eq!(
            gz.content_path(&hash),
            tmp.path()
                .join("gz")
                .join(&raw[..2])
                .join(format!("{}.gz", &raw[2..]))
        );
    }

    #[test]
    fn store_repairs_a_blob_truncated_by_a_crash() {
        for compression in [false, true] {
            let tmp = TempDir::new().unwrap();
            let store = ContentStore::new(tmp.path().to_path_buf(), compression).unwrap();
            let content = b"important bytes that must survive a crash".repeat(20);
            let hash = store.store(&content).unwrap();

            // What a crash mid-write leaves at the final path.
            let path = store.content_path(&hash);
            let on_disk = fs::read(&path).unwrap();
            fs::write(&path, &on_disk[..on_disk.len() / 2]).unwrap();
            assert!(
                store.retrieve(&hash).is_err(),
                "setup: blob must be damaged"
            );

            assert_eq!(store.store(&content).unwrap(), hash);
            assert_eq!(store.retrieve(&hash).unwrap(), content);
        }
    }

    #[test]
    fn store_leaves_no_temp_files_and_count_ignores_strays() {
        for compression in [false, true] {
            let tmp = TempDir::new().unwrap();
            let store = ContentStore::new(tmp.path().to_path_buf(), compression).unwrap();
            let hash = store.store(b"one blob").unwrap();
            store.store(b"one blob").unwrap();
            assert_eq!(store.count().unwrap(), 1);

            let dir = store.content_path(&hash).parent().unwrap().to_path_buf();
            let leftovers: Vec<_> = fs::read_dir(&dir)
                .unwrap()
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with(TMP_PREFIX))
                .collect();
            assert!(leftovers.is_empty(), "store left temp files behind");

            // A temp file orphaned by a crash is not a stored blob.
            fs::write(dir.join(format!("{TMP_PREFIX}orphan")), b"partial").unwrap();
            assert_eq!(store.count().unwrap(), 1);
            assert_eq!(store.retrieve(&hash).unwrap(), b"one blob");
        }
    }
}
