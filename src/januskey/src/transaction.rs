// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// Transaction Manager: Group operations with commit/rollback support
// Theorem 3.4 (Sequential Reversibility) guarantees complete rollback

use crate::content_store::ContentStore;
use crate::error::{JanusError, Result};
use crate::metadata::{MetadataStore, OperationMetadata};
use crate::operations::{FileOperation, OperationExecutor};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// Transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionState {
    /// Transaction is active and accepting operations
    Active,
    /// Transaction has been committed
    Committed,
    /// Transaction has been rolled back
    RolledBack,
}

/// A transaction grouping multiple operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique transaction ID
    pub id: String,
    /// Human-readable name (optional)
    pub name: Option<String>,
    /// When the transaction was started
    pub started_at: DateTime<Utc>,
    /// When the transaction was completed (commit or rollback)
    pub completed_at: Option<DateTime<Utc>>,
    /// Current state
    pub state: TransactionState,
    /// IDs of operations in this transaction
    pub operation_ids: Vec<String>,
    /// User who started the transaction
    pub user: String,
}

impl Transaction {
    /// Create a new active transaction
    pub fn new(name: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            started_at: Utc::now(),
            completed_at: None,
            state: TransactionState::Active,
            operation_ids: Vec::new(),
            user: whoami::username(),
        }
    }

    /// Check if transaction is active
    pub fn is_active(&self) -> bool {
        self.state == TransactionState::Active
    }

    /// Add an operation to this transaction
    pub fn add_operation(&mut self, operation_id: String) {
        self.operation_ids.push(operation_id);
    }

    /// Mark as committed
    pub fn commit(&mut self) {
        self.state = TransactionState::Committed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark as rolled back
    pub fn rollback(&mut self) {
        self.state = TransactionState::RolledBack;
        self.completed_at = Some(Utc::now());
    }
}

/// Transaction log for persistence
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransactionLog {
    pub version: String,
    pub transactions: Vec<Transaction>,
    pub active_transaction_id: Option<String>,
}

impl TransactionLog {
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            transactions: Vec::new(),
            active_transaction_id: None,
        }
    }
}

/// Manager for transactions
pub struct TransactionManager {
    /// Path to transaction log
    path: PathBuf,
    /// Transaction log
    log: TransactionLog,
}

impl TransactionManager {
    /// Create or open a transaction manager
    pub fn new(path: PathBuf) -> Result<Self> {
        let log = if path.exists() {
            let content = fs::read_to_string(&path)?;
            serde_json::from_str(&content)
                .map_err(|e| JanusError::MetadataCorrupted(e.to_string()))?
        } else {
            TransactionLog::new()
        };

        Ok(Self { path, log })
    }

    /// Save the log to disk
    fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.log)?;
        fs::write(&self.path, content)?;
        Ok(())
    }

    /// Begin a new transaction
    pub fn begin(&mut self, name: Option<String>) -> Result<&Transaction> {
        if let Some(ref active_id) = self.log.active_transaction_id {
            return Err(JanusError::TransactionActive(active_id.clone()));
        }

        let transaction = Transaction::new(name);
        let id = transaction.id.clone();
        self.log.transactions.push(transaction);
        self.log.active_transaction_id = Some(id.clone());
        self.save()?;

        Ok(self.log.transactions.last().unwrap())
    }

    /// Get current active transaction
    pub fn active(&self) -> Option<&Transaction> {
        self.log.active_transaction_id.as_ref().and_then(|id| {
            self.log.transactions.iter().find(|t| t.id == *id)
        })
    }

    /// Get mutable active transaction
    fn active_mut(&mut self) -> Option<&mut Transaction> {
        let active_id = self.log.active_transaction_id.clone()?;
        self.log.transactions.iter_mut().find(|t| t.id == active_id)
    }

    /// Add operation to active transaction
    pub fn add_operation(&mut self, operation_id: String) -> Result<()> {
        let transaction = self
            .active_mut()
            .ok_or(JanusError::NoActiveTransaction)?;
        transaction.add_operation(operation_id);
        self.save()
    }

    /// Commit the active transaction
    pub fn commit(&mut self) -> Result<Transaction> {
        let transaction = self
            .active_mut()
            .ok_or(JanusError::NoActiveTransaction)?;
        transaction.commit();
        let result = transaction.clone();
        self.log.active_transaction_id = None;
        self.save()?;
        Ok(result)
    }

    /// Rollback the active transaction
    pub fn rollback(
        &mut self,
        content_store: &ContentStore,
        metadata_store: &mut MetadataStore,
    ) -> Result<Transaction> {
        let transaction = self.active().ok_or(JanusError::NoActiveTransaction)?.clone();

        // Undo operations in reverse order (Theorem 3.4)
        for op_id in transaction.operation_ids.iter().rev() {
            let mut executor = OperationExecutor::new(content_store, metadata_store);
            executor.undo(op_id)?;
        }

        // Mark transaction as rolled back
        let transaction = self
            .active_mut()
            .ok_or(JanusError::NoActiveTransaction)?;
        transaction.rollback();
        let result = transaction.clone();
        self.log.active_transaction_id = None;
        self.save()?;

        Ok(result)
    }

    /// Get transaction by ID
    pub fn get(&self, id: &str) -> Option<&Transaction> {
        self.log.transactions.iter().find(|t| t.id == id)
    }

    /// Get all transactions
    pub fn all(&self) -> &[Transaction] {
        &self.log.transactions
    }

    /// Check if there's an active transaction
    pub fn has_active(&self) -> bool {
        self.log.active_transaction_id.is_some()
    }

    /// Get active transaction ID
    pub fn active_id(&self) -> Option<&str> {
        self.log.active_transaction_id.as_deref()
    }
}

/// Helper struct for executing operations within a transaction
pub struct TransactionExecutor<'a> {
    content_store: &'a ContentStore,
    metadata_store: &'a mut MetadataStore,
    transaction_manager: &'a mut TransactionManager,
}

impl<'a> TransactionExecutor<'a> {
    pub fn new(
        content_store: &'a ContentStore,
        metadata_store: &'a mut MetadataStore,
        transaction_manager: &'a mut TransactionManager,
    ) -> Self {
        Self {
            content_store,
            metadata_store,
            transaction_manager,
        }
    }

    /// Execute an operation within the current transaction (if any)
    pub fn execute(&mut self, operation: FileOperation) -> Result<OperationMetadata> {
        let transaction_id = self.transaction_manager.active_id().map(String::from);

        let mut executor = OperationExecutor::new(self.content_store, self.metadata_store);
        if let Some(ref tid) = transaction_id {
            executor = executor.with_transaction(tid.clone());
        }

        let metadata = executor.execute(operation)?;

        // If in a transaction, record the operation ID
        if transaction_id.is_some() {
            self.transaction_manager.add_operation(metadata.id.clone())?;
        }

        Ok(metadata)
    }
}

/// Preview of pending transaction operations
#[derive(Debug)]
pub struct TransactionPreview {
    pub transaction_name: Option<String>,
    pub operations: Vec<OperationPreview>,
    pub total_files_affected: usize,
}

#[derive(Debug)]
pub struct OperationPreview {
    pub op_type: String,
    pub path: PathBuf,
    pub secondary_path: Option<PathBuf>,
}

impl TransactionPreview {
    pub fn from_transaction(
        transaction: &Transaction,
        metadata_store: &MetadataStore,
    ) -> Self {
        let mut operations = Vec::new();
        let mut paths_seen = std::collections::HashSet::new();

        for op_id in &transaction.operation_ids {
            if let Some(meta) = metadata_store.get(op_id) {
                paths_seen.insert(meta.path.clone());
                if let Some(ref secondary) = meta.path_secondary {
                    paths_seen.insert(secondary.clone());
                }

                operations.push(OperationPreview {
                    op_type: meta.op_type.to_string(),
                    path: meta.path.clone(),
                    secondary_path: meta.path_secondary.clone(),
                });
            }
        }

        Self {
            transaction_name: transaction.name.clone(),
            operations,
            total_files_affected: paths_seen.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, ContentStore, MetadataStore, TransactionManager) {
        let tmp = TempDir::new().unwrap();
        let content_store = ContentStore::new(tmp.path().join("content"), false).unwrap();
        let metadata_store = MetadataStore::new(tmp.path().join("metadata.json")).unwrap();
        let transaction_manager =
            TransactionManager::new(tmp.path().join("transactions.json")).unwrap();
        (tmp, content_store, metadata_store, transaction_manager)
    }

    #[test]
    fn test_transaction_lifecycle() {
        let (tmp, content_store, mut metadata_store, mut tx_manager) = setup();

        // Begin transaction
        tx_manager.begin(Some("test-tx".to_string())).unwrap();
        assert!(tx_manager.has_active());

        // Create test files
        let file1 = tmp.path().join("file1.txt");
        let file2 = tmp.path().join("file2.txt");
        fs::write(&file1, "content1").unwrap();
        fs::write(&file2, "content2").unwrap();

        // Execute operations in transaction
        {
            let mut executor =
                TransactionExecutor::new(&content_store, &mut metadata_store, &mut tx_manager);
            executor
                .execute(FileOperation::Delete { path: file1.clone() })
                .unwrap();
            executor
                .execute(FileOperation::Delete { path: file2.clone() })
                .unwrap();
        }

        assert!(!file1.exists());
        assert!(!file2.exists());

        // Rollback
        tx_manager
            .rollback(&content_store, &mut metadata_store)
            .unwrap();

        // Files should be restored
        assert!(file1.exists());
        assert!(file2.exists());
        assert!(!tx_manager.has_active());
    }

    #[test]
    fn test_transaction_commit() {
        let (tmp, content_store, mut metadata_store, mut tx_manager) = setup();

        // Begin transaction
        tx_manager.begin(None).unwrap();

        // Create and delete a file
        let file = tmp.path().join("test.txt");
        fs::write(&file, "content").unwrap();

        {
            let mut executor =
                TransactionExecutor::new(&content_store, &mut metadata_store, &mut tx_manager);
            executor
                .execute(FileOperation::Delete { path: file.clone() })
                .unwrap();
        }

        // Commit
        let tx = tx_manager.commit().unwrap();
        assert_eq!(tx.state, TransactionState::Committed);
        assert!(!file.exists());
        assert!(!tx_manager.has_active());
    }

    #[test]
    fn test_cannot_begin_while_active() {
        let (_tmp, _content_store, _metadata_store, mut tx_manager) = setup();

        tx_manager.begin(None).unwrap();
        let result = tx_manager.begin(None);
        assert!(result.is_err());
    }
}
