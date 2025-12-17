// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// FileBackend: Abstraction for filesystem operations across local and remote targets
// Enables JanusKey to work with SSH/SFTP, S3, WebDAV, and other protocols

use crate::error::{JanusError, Result};
use crate::metadata::FileMetadata;
use std::path::{Path, PathBuf};

/// Abstraction over filesystem operations for local and remote targets
pub trait FileBackend: Send + Sync {
    /// Get the backend type identifier
    fn backend_type(&self) -> &'static str;

    /// Check if a path exists
    fn exists(&self, path: &Path) -> Result<bool>;

    /// Check if path is a file
    fn is_file(&self, path: &Path) -> Result<bool>;

    /// Check if path is a directory
    fn is_dir(&self, path: &Path) -> Result<bool>;

    /// Check if path is a symlink
    fn is_symlink(&self, path: &Path) -> Result<bool>;

    /// Read file contents
    fn read(&self, path: &Path) -> Result<Vec<u8>>;

    /// Write file contents
    fn write(&self, path: &Path, content: &[u8]) -> Result<()>;

    /// Delete a file
    fn remove_file(&self, path: &Path) -> Result<()>;

    /// Delete an empty directory
    fn remove_dir(&self, path: &Path) -> Result<()>;

    /// Delete a directory and all contents
    fn remove_dir_all(&self, path: &Path) -> Result<()>;

    /// Create a directory
    fn create_dir(&self, path: &Path) -> Result<()>;

    /// Create directory and all parents
    fn create_dir_all(&self, path: &Path) -> Result<()>;

    /// Rename/move a file or directory
    fn rename(&self, from: &Path, to: &Path) -> Result<()>;

    /// Copy a file
    fn copy(&self, from: &Path, to: &Path) -> Result<u64>;

    /// Get file metadata
    fn metadata(&self, path: &Path) -> Result<FileMetadata>;

    /// Get symlink metadata (don't follow symlinks)
    fn symlink_metadata(&self, path: &Path) -> Result<FileMetadata>;

    /// Read symlink target
    fn read_link(&self, path: &Path) -> Result<PathBuf>;

    /// Create a symbolic link
    fn symlink(&self, target: &Path, link: &Path) -> Result<()>;

    /// Set file permissions (Unix mode)
    fn set_permissions(&self, path: &Path, mode: u32) -> Result<()>;

    /// Set file modification time
    fn set_mtime(&self, path: &Path, mtime: std::time::SystemTime) -> Result<()>;

    /// Truncate file to specified size
    fn truncate(&self, path: &Path, size: u64) -> Result<()>;

    /// Append content to file
    fn append(&self, path: &Path, content: &[u8]) -> Result<()>;

    /// List directory contents
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;

    /// Walk directory tree recursively
    fn walk_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;

    /// Securely overwrite file content (for RMO obliteration)
    fn secure_overwrite(&self, path: &Path, passes: usize) -> Result<()>;
}

/// Local filesystem backend (default)
pub struct LocalBackend;

impl LocalBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl FileBackend for LocalBackend {
    fn backend_type(&self) -> &'static str {
        "local"
    }

    fn exists(&self, path: &Path) -> Result<bool> {
        Ok(path.exists())
    }

    fn is_file(&self, path: &Path) -> Result<bool> {
        Ok(path.is_file())
    }

    fn is_dir(&self, path: &Path) -> Result<bool> {
        Ok(path.is_dir())
    }

    fn is_symlink(&self, path: &Path) -> Result<bool> {
        Ok(path.symlink_metadata()?.file_type().is_symlink())
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>> {
        std::fs::read(path).map_err(Into::into)
    }

    fn write(&self, path: &Path, content: &[u8]) -> Result<()> {
        std::fs::write(path, content).map_err(Into::into)
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        std::fs::remove_file(path).map_err(Into::into)
    }

    fn remove_dir(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir(path).map_err(Into::into)
    }

    fn remove_dir_all(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path).map_err(Into::into)
    }

    fn create_dir(&self, path: &Path) -> Result<()> {
        std::fs::create_dir(path).map_err(Into::into)
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(path).map_err(Into::into)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        std::fs::rename(from, to).map_err(Into::into)
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<u64> {
        std::fs::copy(from, to).map_err(Into::into)
    }

    fn metadata(&self, path: &Path) -> Result<FileMetadata> {
        FileMetadata::from_path(path)
    }

    fn symlink_metadata(&self, path: &Path) -> Result<FileMetadata> {
        FileMetadata::from_path(path)
    }

    fn read_link(&self, path: &Path) -> Result<PathBuf> {
        std::fs::read_link(path).map_err(Into::into)
    }

    #[cfg(unix)]
    fn symlink(&self, target: &Path, link: &Path) -> Result<()> {
        std::os::unix::fs::symlink(target, link).map_err(Into::into)
    }

    #[cfg(not(unix))]
    fn symlink(&self, _target: &Path, _link: &Path) -> Result<()> {
        Err(JanusError::OperationFailed(
            "Symlinks not supported on this platform".to_string(),
        ))
    }

    #[cfg(unix)]
    fn set_permissions(&self, path: &Path, mode: u32) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path, perms).map_err(Into::into)
    }

    #[cfg(not(unix))]
    fn set_permissions(&self, _path: &Path, _mode: u32) -> Result<()> {
        // Windows doesn't use Unix permissions
        Ok(())
    }

    fn set_mtime(&self, path: &Path, mtime: std::time::SystemTime) -> Result<()> {
        let ft = filetime::FileTime::from_system_time(mtime);
        filetime::set_file_mtime(path, ft).map_err(|e| JanusError::IoError(e.to_string()))
    }

    fn truncate(&self, path: &Path, size: u64) -> Result<()> {
        let file = std::fs::OpenOptions::new().write(true).open(path)?;
        file.set_len(size).map_err(Into::into)
    }

    fn append(&self, path: &Path, content: &[u8]) -> Result<()> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
        file.write_all(content).map_err(Into::into)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let entries: std::io::Result<Vec<_>> = std::fs::read_dir(path)?
            .map(|e| e.map(|e| e.path()))
            .collect();
        entries.map_err(Into::into)
    }

    fn walk_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        for entry in walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            paths.push(entry.path().to_path_buf());
        }
        Ok(paths)
    }

    fn secure_overwrite(&self, path: &Path, passes: usize) -> Result<()> {
        use rand::RngCore;
        use std::io::{Seek, SeekFrom, Write};

        let metadata = std::fs::metadata(path)?;
        let size = metadata.len() as usize;

        let mut file = std::fs::OpenOptions::new().write(true).open(path)?;

        for pass in 0..passes {
            file.seek(SeekFrom::Start(0))?;

            let pattern: Vec<u8> = match pass % 3 {
                0 => vec![0x00; size], // All zeros
                1 => vec![0xFF; size], // All ones
                _ => {
                    let mut buf = vec![0u8; size];
                    rand::thread_rng().fill_bytes(&mut buf);
                    buf
                }
            };

            file.write_all(&pattern)?;
            file.sync_all()?;
        }

        Ok(())
    }
}

/// URI parser for remote backends
#[derive(Debug, Clone)]
pub struct RemoteUri {
    pub protocol: String,
    pub user: Option<String>,
    pub host: String,
    pub port: Option<u16>,
    pub path: PathBuf,
}

impl RemoteUri {
    /// Parse a URI string like "ssh://user@host:port/path" or "s3://bucket/key"
    pub fn parse(uri: &str) -> Option<Self> {
        // Handle protocol prefix
        let (protocol, rest) = uri.split_once("://")?;

        // Handle user@host:port/path
        let (authority, path) = if let Some(idx) = rest.find('/') {
            (&rest[..idx], &rest[idx..])
        } else {
            (rest, "/")
        };

        let (user_host, port) = if let Some(idx) = authority.rfind(':') {
            let port_str = &authority[idx + 1..];
            if let Ok(p) = port_str.parse::<u16>() {
                (&authority[..idx], Some(p))
            } else {
                (authority, None)
            }
        } else {
            (authority, None)
        };

        let (user, host) = if let Some(idx) = user_host.find('@') {
            (Some(user_host[..idx].to_string()), &user_host[idx + 1..])
        } else {
            (None, user_host)
        };

        Some(Self {
            protocol: protocol.to_string(),
            user,
            host: host.to_string(),
            port,
            path: PathBuf::from(path),
        })
    }
}

/// Configuration for SSH backend
#[derive(Debug, Clone)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: Option<PathBuf>,
    pub password: Option<String>,
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 22,
            user: whoami::username(),
            key_path: None,
            password: None,
        }
    }
}

/// Configuration for S3 backend
#[derive(Debug, Clone)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            region: "us-east-1".to_string(),
            endpoint: None,
            access_key: None,
            secret_key: None,
        }
    }
}

/// Factory for creating backends from URIs
pub struct BackendFactory;

impl BackendFactory {
    /// Create a backend from a path or URI
    ///
    /// Supported formats:
    /// - Local path: `/path/to/file` or `./relative`
    /// - SSH/SFTP: `ssh://user@host:port/path` or `sftp://...`
    /// - S3: `s3://bucket/key`
    pub fn from_uri(uri: &str) -> Result<Box<dyn FileBackend>> {
        if uri.contains("://") {
            let parsed = RemoteUri::parse(uri)
                .ok_or_else(|| JanusError::OperationFailed(format!("Invalid URI: {}", uri)))?;

            match parsed.protocol.as_str() {
                "ssh" | "sftp" => {
                    // SSH backend would be instantiated here
                    // For now, return an error indicating it needs to be enabled
                    Err(JanusError::OperationFailed(
                        "SSH backend not yet implemented. Use 'ssh' feature flag.".to_string(),
                    ))
                }
                "s3" => {
                    // S3 backend would be instantiated here
                    Err(JanusError::OperationFailed(
                        "S3 backend not yet implemented. Use 's3' feature flag.".to_string(),
                    ))
                }
                proto => Err(JanusError::OperationFailed(format!(
                    "Unknown protocol: {}",
                    proto
                ))),
            }
        } else {
            // Local path
            Ok(Box::new(LocalBackend::new()))
        }
    }

    /// Create a local backend
    pub fn local() -> Box<dyn FileBackend> {
        Box::new(LocalBackend::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_local_backend_basic_ops() {
        let tmp = TempDir::new().unwrap();
        let backend = LocalBackend::new();

        let test_file = tmp.path().join("test.txt");

        // Write
        backend.write(&test_file, b"hello world").unwrap();
        assert!(backend.exists(&test_file).unwrap());
        assert!(backend.is_file(&test_file).unwrap());

        // Read
        let content = backend.read(&test_file).unwrap();
        assert_eq!(content, b"hello world");

        // Append
        backend.append(&test_file, b" appended").unwrap();
        let content = backend.read(&test_file).unwrap();
        assert_eq!(content, b"hello world appended");

        // Truncate
        backend.truncate(&test_file, 5).unwrap();
        let content = backend.read(&test_file).unwrap();
        assert_eq!(content, b"hello");

        // Delete
        backend.remove_file(&test_file).unwrap();
        assert!(!backend.exists(&test_file).unwrap());
    }

    #[test]
    fn test_local_backend_directory_ops() {
        let tmp = TempDir::new().unwrap();
        let backend = LocalBackend::new();

        let test_dir = tmp.path().join("subdir");

        // Create directory
        backend.create_dir(&test_dir).unwrap();
        assert!(backend.is_dir(&test_dir).unwrap());

        // Create nested directories
        let nested = test_dir.join("a/b/c");
        backend.create_dir_all(&nested).unwrap();
        assert!(backend.is_dir(&nested).unwrap());

        // Remove directory
        backend.remove_dir(&nested).unwrap();
        assert!(!backend.exists(&nested).unwrap());
    }

    #[test]
    fn test_uri_parsing() {
        let uri = RemoteUri::parse("ssh://user@example.com:2222/home/user/file.txt").unwrap();
        assert_eq!(uri.protocol, "ssh");
        assert_eq!(uri.user, Some("user".to_string()));
        assert_eq!(uri.host, "example.com");
        assert_eq!(uri.port, Some(2222));
        assert_eq!(uri.path, PathBuf::from("/home/user/file.txt"));

        let uri = RemoteUri::parse("s3://my-bucket/path/to/object").unwrap();
        assert_eq!(uri.protocol, "s3");
        assert_eq!(uri.host, "my-bucket");
        assert_eq!(uri.path, PathBuf::from("/path/to/object"));
    }

    #[test]
    fn test_backend_factory() {
        // Local path should work
        let backend = BackendFactory::from_uri("/tmp/test").unwrap();
        assert_eq!(backend.backend_type(), "local");

        // SSH should fail gracefully (not implemented)
        let result = BackendFactory::from_uri("ssh://user@host/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_secure_overwrite() {
        let tmp = TempDir::new().unwrap();
        let backend = LocalBackend::new();

        let test_file = tmp.path().join("secret.txt");
        backend.write(&test_file, b"sensitive data here").unwrap();

        // Secure overwrite with 3 passes
        backend.secure_overwrite(&test_file, 3).unwrap();

        // File should still exist but content should be overwritten
        assert!(backend.exists(&test_file).unwrap());
        let content = backend.read(&test_file).unwrap();
        assert_ne!(content, b"sensitive data here");
    }
}
