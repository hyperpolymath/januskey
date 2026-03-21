// SPDX-License-Identifier: MIT OR PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// Transaction types: Group operations with commit/rollback support
// Theorem 3.4 (Sequential Reversibility) guarantees complete rollback
//
// Note: TransactionExecutor (which performs actual filesystem rollback)
// lives in januskey-cli, not here. This module provides only the data
// types and persistence — no filesystem side effects.

use crate::error::{ReversibleError, Result};
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

/// A transaction grouping multiple operations.
///
/// Per absolute-zero: a committed transaction is a composition of
/// operations. Rolling back applies the inverses in reverse order,
/// yielding a CNO (identity on state).
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
    /// IDs of operations in this transaction (in execution order)
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

/// Manager for transaction lifecycle (data + persistence only).
///
/// Filesystem-level rollback (undoing operations) is handled by
/// the consuming crate's executor, not here. This manager tracks
/// transaction state and persists it.
pub struct TransactionManager {
    /// Path to transaction log file
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
                .map_err(|e| ReversibleError::MetadataCorrupted(e.to_string()))?
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
            return Err(ReversibleError::TransactionActive(active_id.clone()));
        }

        let transaction = Transaction::new(name);
        let id = transaction.id.clone();
        self.log.transactions.push(transaction);
        self.log.active_transaction_id = Some(id);
        self.save()?;

        Ok(self.log.transactions.last().unwrap())
    }

    /// Get current active transaction
    pub fn active(&self) -> Option<&Transaction> {
        self.log
            .active_transaction_id
            .as_ref()
            .and_then(|id| self.log.transactions.iter().find(|t| t.id == *id))
    }

    /// Get mutable active transaction
    pub fn active_mut(&mut self) -> Option<&mut Transaction> {
        let active_id = self.log.active_transaction_id.clone()?;
        self.log
            .transactions
            .iter_mut()
            .find(|t| t.id == active_id)
    }

    /// Add operation to active transaction
    pub fn add_operation(&mut self, operation_id: String) -> Result<()> {
        let transaction = self
            .active_mut()
            .ok_or(ReversibleError::NoActiveTransaction)?;
        transaction.add_operation(operation_id);
        self.save()
    }

    /// Commit the active transaction (marks state only — no filesystem effects)
    pub fn commit(&mut self) -> Result<Transaction> {
        let transaction = self
            .active_mut()
            .ok_or(ReversibleError::NoActiveTransaction)?;
        transaction.commit();
        let result = transaction.clone();
        self.log.active_transaction_id = None;
        self.save()?;
        Ok(result)
    }

    /// Mark the active transaction as rolled back (state only).
    ///
    /// The caller is responsible for actually undoing the operations
    /// via the appropriate executor before calling this.
    pub fn mark_rolled_back(&mut self) -> Result<Transaction> {
        let transaction = self
            .active_mut()
            .ok_or(ReversibleError::NoActiveTransaction)?;
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

/// Preview of pending transaction operations (for display)
#[derive(Debug)]
pub struct TransactionPreview {
    pub transaction_name: Option<String>,
    pub operations: Vec<OperationPreview>,
    pub total_files_affected: usize,
}

/// Single operation preview entry
#[derive(Debug)]
pub struct OperationPreview {
    pub op_type: String,
    pub path: std::path::PathBuf,
    pub secondary_path: Option<std::path::PathBuf>,
}

impl TransactionPreview {
    /// Build a preview from a transaction and its metadata store
    pub fn from_transaction(
        transaction: &Transaction,
        metadata_store: &crate::MetadataStore,
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

    #[test]
    fn test_transaction_lifecycle() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("transactions.json");
        let mut manager = TransactionManager::new(path).unwrap();

        // Begin
        manager.begin(Some("test".to_string())).unwrap();
        assert!(manager.has_active());

        // Add operations
        manager.add_operation("op-1".to_string()).unwrap();
        manager.add_operation("op-2".to_string()).unwrap();

        // Commit
        let tx = manager.commit().unwrap();
        assert_eq!(tx.state, TransactionState::Committed);
        assert_eq!(tx.operation_ids.len(), 2);
        assert!(!manager.has_active());
    }

    #[test]
    fn test_cannot_begin_while_active() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("transactions.json");
        let mut manager = TransactionManager::new(path).unwrap();

        manager.begin(None).unwrap();
        assert!(manager.begin(None).is_err());
    }
}
