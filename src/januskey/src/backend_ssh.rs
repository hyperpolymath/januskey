// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// SSH/SFTP Backend: Remote file operations over SSH
// Requires 'ssh' feature flag

use crate::backend::{FileBackend, SshConfig};
use crate::error::{JanusError, Result};
use crate::metadata::FileMetadata;
use chrono::{DateTime, Utc};
use ssh2::{FileStat, Session, Sftp};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// SSH/SFTP backend for remote file operations
pub struct SshBackend {
    config: SshConfig,
    session: Arc<Mutex<Session>>,
}

impl SshBackend {
    /// Create a new SSH backend with the given configuration
    pub fn new(config: SshConfig) -> Result<Self> {
        let session = Self::connect(&config)?;
        Ok(Self {
            config,
            session: Arc::new(Mutex::new(session)),
        })
    }

    /// Connect to the SSH server
    fn connect(config: &SshConfig) -> Result<Session> {
        let addr = format!("{}:{}", config.host, config.port);
        let tcp = TcpStream::connect(&addr).map_err(|e| {
            JanusError::IoError(format!("Failed to connect to {}: {}", addr, e))
        })?;

        tcp.set_read_timeout(Some(Duration::from_secs(30)))?;
        tcp.set_write_timeout(Some(Duration::from_secs(30)))?;

        let mut session = Session::new()
            .map_err(|e| JanusError::IoError(format!("Failed to create SSH session: {}", e)))?;

        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|e| JanusError::IoError(format!("SSH handshake failed: {}", e)))?;

        // Authenticate
        if let Some(ref key_path) = config.key_path {
            session
                .userauth_pubkey_file(&config.user, None, key_path, config.password.as_deref())
                .map_err(|e| JanusError::IoError(format!("SSH key auth failed: {}", e)))?;
        } else if let Some(ref password) = config.password {
            session
                .userauth_password(&config.user, password)
                .map_err(|e| JanusError::IoError(format!("SSH password auth failed: {}", e)))?;
        } else {
            // Try SSH agent
            let mut agent = session
                .agent()
                .map_err(|e| JanusError::IoError(format!("SSH agent failed: {}", e)))?;
            agent
                .connect()
                .map_err(|e| JanusError::IoError(format!("SSH agent connect failed: {}", e)))?;
            agent
                .list_identities()
                .map_err(|e| JanusError::IoError(format!("SSH agent list failed: {}", e)))?;

            let identities: Vec<_> = agent.identities().collect();
            let mut authenticated = false;
            for identity in identities {
                if let Ok(id) = identity {
                    if agent.userauth(&config.user, &id).is_ok() {
                        authenticated = true;
                        break;
                    }
                }
            }
            if !authenticated {
                return Err(JanusError::IoError(
                    "No SSH authentication method succeeded".to_string(),
                ));
            }
        }

        if !session.authenticated() {
            return Err(JanusError::IoError("SSH authentication failed".to_string()));
        }

        Ok(session)
    }

    /// Get an SFTP session
    fn sftp(&self) -> Result<Sftp> {
        let session = self.session.lock().map_err(|e| {
            JanusError::IoError(format!("Failed to lock SSH session: {}", e))
        })?;

        session
            .sftp()
            .map_err(|e| JanusError::IoError(format!("Failed to create SFTP session: {}", e)))
    }

    /// Convert FileStat to FileMetadata
    fn stat_to_metadata(stat: &FileStat, path: &Path) -> FileMetadata {
        let permissions = stat.perm.unwrap_or(0o644);
        let size = stat.size.unwrap_or(0);
        let mtime = stat.mtime.map(|t| {
            DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_secs(t))
        }).unwrap_or_else(Utc::now);

        let uid = stat.uid.unwrap_or(0);
        let gid = stat.gid.unwrap_or(0);

        FileMetadata {
            permissions,
            owner: uid.to_string(),
            group: gid.to_string(),
            size,
            modified: mtime,
            is_symlink: false, // SFTP stat follows symlinks
            symlink_target: None,
        }
    }
}

impl FileBackend for SshBackend {
    fn backend_type(&self) -> &'static str {
        "ssh"
    }

    fn exists(&self, path: &Path) -> Result<bool> {
        let sftp = self.sftp()?;
        match sftp.stat(path) {
            Ok(_) => Ok(true),
            Err(e) if e.code() == ssh2::ErrorCode::SFTP(2) => Ok(false), // No such file
            Err(e) => Err(JanusError::IoError(format!("SFTP stat failed: {}", e))),
        }
    }

    fn is_file(&self, path: &Path) -> Result<bool> {
        let sftp = self.sftp()?;
        match sftp.stat(path) {
            Ok(stat) => Ok(stat.is_file()),
            Err(e) if e.code() == ssh2::ErrorCode::SFTP(2) => Ok(false),
            Err(e) => Err(JanusError::IoError(format!("SFTP stat failed: {}", e))),
        }
    }

    fn is_dir(&self, path: &Path) -> Result<bool> {
        let sftp = self.sftp()?;
        match sftp.stat(path) {
            Ok(stat) => Ok(stat.is_dir()),
            Err(e) if e.code() == ssh2::ErrorCode::SFTP(2) => Ok(false),
            Err(e) => Err(JanusError::IoError(format!("SFTP stat failed: {}", e))),
        }
    }

    fn is_symlink(&self, path: &Path) -> Result<bool> {
        let sftp = self.sftp()?;
        match sftp.lstat(path) {
            Ok(stat) => {
                // Check if it's a symlink by comparing stat and lstat
                if let Ok(target_stat) = sftp.stat(path) {
                    // If they differ, it's likely a symlink
                    Ok(stat.size != target_stat.size || stat.mtime != target_stat.mtime)
                } else {
                    // If stat fails but lstat succeeds, it's a broken symlink
                    Ok(true)
                }
            }
            Err(e) if e.code() == ssh2::ErrorCode::SFTP(2) => Ok(false),
            Err(e) => Err(JanusError::IoError(format!("SFTP lstat failed: {}", e))),
        }
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>> {
        let sftp = self.sftp()?;
        let mut file = sftp.open(path).map_err(|e| {
            JanusError::IoError(format!("SFTP open failed for {}: {}", path.display(), e))
        })?;

        let mut content = Vec::new();
        file.read_to_end(&mut content).map_err(|e| {
            JanusError::IoError(format!("SFTP read failed: {}", e))
        })?;

        Ok(content)
    }

    fn write(&self, path: &Path, content: &[u8]) -> Result<()> {
        let sftp = self.sftp()?;
        let mut file = sftp
            .create(path)
            .map_err(|e| JanusError::IoError(format!("SFTP create failed: {}", e)))?;

        file.write_all(content)
            .map_err(|e| JanusError::IoError(format!("SFTP write failed: {}", e)))?;

        Ok(())
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        let sftp = self.sftp()?;
        sftp.unlink(path)
            .map_err(|e| JanusError::IoError(format!("SFTP unlink failed: {}", e)))
    }

    fn remove_dir(&self, path: &Path) -> Result<()> {
        let sftp = self.sftp()?;
        sftp.rmdir(path)
            .map_err(|e| JanusError::IoError(format!("SFTP rmdir failed: {}", e)))
    }

    fn remove_dir_all(&self, path: &Path) -> Result<()> {
        // Recursively remove directory contents
        let entries = self.read_dir(path)?;

        for entry in entries {
            if self.is_dir(&entry)? {
                self.remove_dir_all(&entry)?;
            } else {
                self.remove_file(&entry)?;
            }
        }

        self.remove_dir(path)
    }

    fn create_dir(&self, path: &Path) -> Result<()> {
        let sftp = self.sftp()?;
        sftp.mkdir(path, 0o755)
            .map_err(|e| JanusError::IoError(format!("SFTP mkdir failed: {}", e)))
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component);
            if !self.exists(&current)? {
                self.create_dir(&current)?;
            }
        }
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        let sftp = self.sftp()?;
        sftp.rename(from, to, None)
            .map_err(|e| JanusError::IoError(format!("SFTP rename failed: {}", e)))
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<u64> {
        // SFTP doesn't have a native copy, so we read and write
        let content = self.read(from)?;
        let size = content.len() as u64;
        self.write(to, &content)?;
        Ok(size)
    }

    fn metadata(&self, path: &Path) -> Result<FileMetadata> {
        let sftp = self.sftp()?;
        let stat = sftp
            .stat(path)
            .map_err(|e| JanusError::IoError(format!("SFTP stat failed: {}", e)))?;

        Ok(Self::stat_to_metadata(&stat, path))
    }

    fn symlink_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let sftp = self.sftp()?;
        let stat = sftp
            .lstat(path)
            .map_err(|e| JanusError::IoError(format!("SFTP lstat failed: {}", e)))?;

        Ok(Self::stat_to_metadata(&stat, path))
    }

    fn read_link(&self, path: &Path) -> Result<PathBuf> {
        let sftp = self.sftp()?;
        sftp.readlink(path)
            .map_err(|e| JanusError::IoError(format!("SFTP readlink failed: {}", e)))
    }

    fn symlink(&self, target: &Path, link: &Path) -> Result<()> {
        let sftp = self.sftp()?;
        sftp.symlink(target, link)
            .map_err(|e| JanusError::IoError(format!("SFTP symlink failed: {}", e)))
    }

    fn set_permissions(&self, path: &Path, mode: u32) -> Result<()> {
        let sftp = self.sftp()?;
        let mut stat = FileStat::default();
        stat.perm = Some(mode);

        sftp.setstat(path, stat)
            .map_err(|e| JanusError::IoError(format!("SFTP setstat failed: {}", e)))
    }

    fn set_mtime(&self, path: &Path, mtime: SystemTime) -> Result<()> {
        let sftp = self.sftp()?;
        let duration = mtime
            .duration_since(UNIX_EPOCH)
            .map_err(|e| JanusError::IoError(format!("Invalid mtime: {}", e)))?;

        let mut stat = FileStat::default();
        stat.mtime = Some(duration.as_secs());

        sftp.setstat(path, stat)
            .map_err(|e| JanusError::IoError(format!("SFTP setstat failed: {}", e)))
    }

    fn truncate(&self, path: &Path, size: u64) -> Result<()> {
        // SFTP doesn't have truncate, so we read, truncate in memory, and write
        let content = self.read(path)?;
        let truncated: Vec<u8> = content.into_iter().take(size as usize).collect();
        self.write(path, &truncated)
    }

    fn append(&self, path: &Path, content: &[u8]) -> Result<()> {
        let sftp = self.sftp()?;
        let mut file = sftp
            .open_mode(
                path,
                ssh2::OpenFlags::WRITE | ssh2::OpenFlags::APPEND,
                0o644,
                ssh2::OpenType::File,
            )
            .map_err(|e| JanusError::IoError(format!("SFTP open for append failed: {}", e)))?;

        file.write_all(content)
            .map_err(|e| JanusError::IoError(format!("SFTP append failed: {}", e)))
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let sftp = self.sftp()?;
        let dir = sftp
            .readdir(path)
            .map_err(|e| JanusError::IoError(format!("SFTP readdir failed: {}", e)))?;

        Ok(dir.into_iter().map(|(p, _)| p).collect())
    }

    fn walk_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut paths = vec![path.to_path_buf()];
        let mut result = Vec::new();

        while let Some(current) = paths.pop() {
            result.push(current.clone());
            if self.is_dir(&current)? {
                for entry in self.read_dir(&current)? {
                    paths.push(entry);
                }
            }
        }

        Ok(result)
    }

    fn secure_overwrite(&self, path: &Path, passes: usize) -> Result<()> {
        use rand::RngCore;

        // Get file size
        let metadata = self.metadata(path)?;
        let size = metadata.size as usize;

        for pass in 0..passes {
            let pattern: Vec<u8> = match pass % 3 {
                0 => vec![0x00; size],
                1 => vec![0xFF; size],
                _ => {
                    let mut buf = vec![0u8; size];
                    rand::thread_rng().fill_bytes(&mut buf);
                    buf
                }
            };

            self.write(path, &pattern)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // SSH tests require a running SSH server, so they're disabled by default
    // Run with: cargo test --features ssh -- --ignored

    #[test]
    #[ignore]
    fn test_ssh_backend_requires_server() {
        // This test is a placeholder - real tests need an SSH server
        println!("SSH backend tests require a running SSH server");
    }
}
