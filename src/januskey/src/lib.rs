// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// JanusKey: Provably Reversible File Operations
// Through Maximal Principle Reduction (MPR)

pub mod backend;
pub mod content_store;
pub mod delta;
pub mod error;
pub mod ffi;
pub mod metadata;
pub mod obliteration;
pub mod operations;
pub mod transaction;

// Feature-gated backend implementations
#[cfg(feature = "ssh")]
pub mod backend_ssh;

#[cfg(feature = "s3")]
pub mod backend_s3;

pub use backend::{BackendFactory, FileBackend, LocalBackend, RemoteUri, S3Config, SshConfig};
pub use content_store::{ContentHash, ContentStore};
pub use error::{JanusError, Result};
pub use metadata::{MetadataStore, OperationMetadata, OperationType};
pub use obliteration::{ObliterationManager, ObliterationProof, ObliterationRecord};
pub use operations::{FileOperation, OperationExecutor};
pub use transaction::{Transaction, TransactionManager};

/// JanusKey configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// Path to JanusKey metadata storage
    pub storage_path: std::path::PathBuf,
    /// Enable compression for stored content
    pub compression: bool,
    /// Maximum number of operations to keep in history
    pub max_history: usize,
    /// Auto-confirm dangerous operations
    pub auto_confirm: bool,
    /// Default to dry-run mode
    pub dry_run_default: bool,
    /// Enable audit trail
    pub audit_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        let storage_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("januskey");

        Self {
            storage_path,
            compression: true,
            max_history: 10000,
            auto_confirm: false,
            dry_run_default: false,
            audit_enabled: true,
        }
    }
}

impl Config {
    /// Load config from directory's .januskey/config.json or use defaults
    pub fn load(dir: &std::path::Path) -> Self {
        let config_path = dir.join(".januskey").join("config.json");
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// Save config to directory
    pub fn save(&self, dir: &std::path::Path) -> Result<()> {
        let config_dir = dir.join(".januskey");
        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
}

/// Main JanusKey instance for a directory
pub struct JanusKey {
    /// Working directory
    pub root: std::path::PathBuf,
    /// Configuration
    pub config: Config,
    /// Content-addressed storage
    pub content_store: ContentStore,
    /// Metadata/operation log store
    pub metadata_store: MetadataStore,
    /// Transaction manager
    pub transaction_manager: TransactionManager,
    /// Obliteration manager (RMO primitive)
    pub obliteration_manager: ObliterationManager,
}

impl JanusKey {
    /// Initialize JanusKey for a directory
    pub fn init(root: &std::path::Path) -> Result<Self> {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let jk_dir = root.join(".januskey");
        std::fs::create_dir_all(&jk_dir)?;

        let config = Config::load(&root);
        config.save(&root)?;

        let content_store = ContentStore::new(jk_dir.join("content"), config.compression)?;
        let metadata_store = MetadataStore::new(jk_dir.join("metadata.json"))?;
        let transaction_manager = TransactionManager::new(jk_dir.join("transactions"))?;
        let obliteration_manager = ObliterationManager::new(jk_dir.join("obliterations.json"))?;

        Ok(Self {
            root,
            config,
            content_store,
            metadata_store,
            transaction_manager,
            obliteration_manager,
        })
    }

    /// Open existing JanusKey directory
    pub fn open(root: &std::path::Path) -> Result<Self> {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let jk_dir = root.join(".januskey");

        if !jk_dir.exists() {
            return Err(JanusError::NotInitialized(root.display().to_string()));
        }

        let config = Config::load(&root);
        let content_store = ContentStore::new(jk_dir.join("content"), config.compression)?;
        let metadata_store = MetadataStore::new(jk_dir.join("metadata.json"))?;
        let transaction_manager = TransactionManager::new(jk_dir.join("transactions"))?;
        let obliteration_manager = ObliterationManager::new(jk_dir.join("obliterations.json"))?;

        Ok(Self {
            root,
            config,
            content_store,
            metadata_store,
            transaction_manager,
            obliteration_manager,
        })
    }

    /// Check if directory is initialized
    pub fn is_initialized(root: &std::path::Path) -> bool {
        root.join(".januskey").exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_and_open() {
        let tmp = TempDir::new().unwrap();
        let jk = JanusKey::init(tmp.path()).unwrap();
        assert!(JanusKey::is_initialized(tmp.path()));

        let jk2 = JanusKey::open(tmp.path()).unwrap();
        assert_eq!(jk.root, jk2.root);
    }
}
