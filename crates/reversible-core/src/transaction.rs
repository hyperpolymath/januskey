// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// Transaction types: Group operations with commit/rollback support
// Theorem 3.4 (Sequential Reversibility) guarantees complete rollback
//
// Note: TransactionExecutor (which performs actual filesystem rollback)
// lives in januskey-cli, not here. This module provides only the data
// types and persistence — no filesystem side effects.

use crate::error::{Result, ReversibleError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
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

/// Distinguishes the temporary files of saves made by one process.
static SAVE_SEQ: AtomicU64 = AtomicU64::new(0);

/// A sibling of `path` that no other save, in this or another process, uses.
fn unique_tmp_path(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        SAVE_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    PathBuf::from(tmp)
}

/// Create the temporary file, failing if it exists: never truncate or adopt a
/// file another writer may be using.
fn create_tmp(tmp: &Path) -> std::io::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(tmp)
}

/// Replace `path` with `content` so a reader sees the old file or the new one,
/// never a torn write: write a uniquely named sibling temporary file, flush it
/// to disk, then rename it over `path`. The file keeps the permissions of the
/// one it replaces.
fn write_atomic(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let tmp = unique_tmp_path(path);

    let written = (|| {
        let mut file = create_tmp(&tmp)?;
        file.write_all(content)?;
        if let Ok(existing) = fs::metadata(path) {
            file.set_permissions(existing.permissions())?;
        }
        file.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if written.is_err() {
        // Best effort: nothing reads a stray temporary file, but there is no
        // reason to leave one behind.
        let _ = fs::remove_file(&tmp);
    }
    written?;

    // Make the rename itself durable. This is best effort, deliberately: the
    // rename has already happened, so failing the save here would leave the
    // disk on the new log and memory on the old one, the disagreement this
    // function exists to prevent. Not every platform can open a directory, and
    // some filesystems refuse to sync one.
    if let Ok(dir) = fs::File::open(containing_dir(path)) {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// The directory that holds `path`. A bare filename has the empty path as its
/// parent, which cannot be opened, so it is the current directory.
fn containing_dir(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
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
            let content = ({
                use std::io::Read;
                std::fs::File::open(&path).and_then(|f| {
                    let mut buf = String::new();
                    f.take(10 * 1024 * 1024).read_to_string(&mut buf)?;
                    Ok(buf)
                })
            })?;
            serde_json::from_str(&content)
                .map_err(|e| ReversibleError::MetadataCorrupted(e.to_string()))?
        } else {
            TransactionLog::new()
        };

        Ok(Self { path, log })
    }

    /// Persist `next` and, only once it is on disk, make it the in-memory log.
    ///
    /// Every mutator builds its successor log on a copy and ends here, so a
    /// failed save leaves memory and disk both at the previous log (T-5).
    fn persist(&mut self, next: TransactionLog) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&next)?;
        write_atomic(&self.path, content.as_bytes())?;
        self.log = next;
        Ok(())
    }

    /// Apply `f` to the active transaction of a copy of the log, optionally
    /// clear the active pointer, persist, and return the updated transaction.
    fn update_active(
        &mut self,
        settle: bool,
        f: impl FnOnce(&mut Transaction),
    ) -> Result<Transaction> {
        let active_id = self
            .log
            .active_transaction_id
            .clone()
            .ok_or(ReversibleError::NoActiveTransaction)?;
        let mut next = self.log.clone();
        let transaction = next
            .transactions
            .iter_mut()
            .find(|t| t.id == active_id)
            .ok_or(ReversibleError::NoActiveTransaction)?;
        f(transaction);
        let result = transaction.clone();
        if settle {
            next.active_transaction_id = None;
        }
        self.persist(next)?;
        Ok(result)
    }

    /// Begin a new transaction
    pub fn begin(&mut self, name: Option<String>) -> Result<&Transaction> {
        if let Some(ref active_id) = self.log.active_transaction_id {
            return Err(ReversibleError::TransactionActive(active_id.clone()));
        }

        let transaction = Transaction::new(name);
        let id = transaction.id.clone();
        let mut next = self.log.clone();
        next.transactions.push(transaction);
        next.active_transaction_id = Some(id);
        self.persist(next)?;

        // SAFETY: We just pushed a transaction above, so last() is guaranteed Some
        Ok(self
            .log
            .transactions
            .last()
            .expect("transaction was just pushed"))
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
        self.log.transactions.iter_mut().find(|t| t.id == active_id)
    }

    /// Add operation to active transaction
    pub fn add_operation(&mut self, operation_id: String) -> Result<()> {
        self.update_active(false, |t| t.add_operation(operation_id))
            .map(|_| ())
    }

    /// Commit the active transaction (marks state only — no filesystem effects)
    pub fn commit(&mut self) -> Result<Transaction> {
        self.update_active(true, Transaction::commit)
    }

    /// Mark the active transaction as rolled back (state only).
    ///
    /// The caller is responsible for actually undoing the operations
    /// via the appropriate executor before calling this.
    pub fn mark_rolled_back(&mut self) -> Result<Transaction> {
        self.update_active(true, Transaction::rollback)
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
        let tmp = TempDir::new().expect("failed to create temp dir");
        let path = tmp.path().join("transactions.json");
        let mut manager =
            TransactionManager::new(path).expect("failed to create transaction manager");

        // Begin
        manager
            .begin(Some("test".to_string()))
            .expect("failed to begin transaction");
        assert!(manager.has_active());

        // Add operations
        manager
            .add_operation("op-1".to_string())
            .expect("failed to add operation op-1");
        manager
            .add_operation("op-2".to_string())
            .expect("failed to add operation op-2");

        // Commit
        let tx = manager.commit().expect("failed to commit transaction");
        assert_eq!(tx.state, TransactionState::Committed);
        assert_eq!(tx.operation_ids.len(), 2);
        assert!(!manager.has_active());
    }

    /// Make the save fail at its last step, on every platform and for root too:
    /// plant a non-empty directory where the log file goes, so the temporary
    /// file is written but the rename over it is refused.
    fn break_log_file(path: &std::path::Path) {
        let _ = fs::remove_file(path);
        fs::create_dir_all(path).expect("failed to plant a directory over the log");
        fs::write(path.join("blocker"), b"x").expect("failed to fill the planted directory");
    }

    /// The temporary files in `dir`: anything a save could have left behind.
    fn temporary_files(dir: &std::path::Path) -> Vec<String> {
        fs::read_dir(dir)
            .expect("failed to list the log directory")
            .map(|entry| {
                entry
                    .expect("dir entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| name.ends_with(".tmp"))
            .collect()
    }

    fn repair_log_file(path: &std::path::Path) {
        assert_eq!(
            temporary_files(path.parent().expect("log has a parent")),
            Vec::<String>::new(),
            "a failed save must not leave its temporary file behind"
        );
        fs::remove_dir_all(path).expect("failed to remove the planted directory");
    }

    /// A bare filename's directory is `.`, not the unopenable empty path.
    #[test]
    fn containing_dir_of_a_bare_filename_is_the_current_directory() {
        assert_eq!(
            containing_dir(Path::new("transactions.json")),
            Path::new(".")
        );
        assert_eq!(
            containing_dir(Path::new("a/transactions.json")),
            Path::new("a")
        );
        assert_eq!(
            containing_dir(Path::new("/transactions.json")),
            Path::new("/")
        );
        assert!(fs::File::open(containing_dir(Path::new("transactions.json"))).is_ok());
    }

    /// T-5: a begin whose save fails changes nothing in memory and can be retried.
    #[test]
    fn failed_save_leaves_begin_unapplied() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let dir = tmp.path().join("state");
        let mut manager = TransactionManager::new(dir.join("transactions.json"))
            .expect("failed to create transaction manager");

        break_log_file(&dir.join("transactions.json"));
        assert!(manager.begin(Some("t".to_string())).is_err());
        assert!(!manager.has_active(), "a failed begin must not activate");
        assert!(manager.all().is_empty(), "a failed begin must not record");

        repair_log_file(&dir.join("transactions.json"));
        manager.begin(Some("t".to_string())).expect("retry begin");
        assert_eq!(manager.all().len(), 1);
    }

    /// T-5: an add_operation whose save fails is not recorded in memory, and the
    /// retry records it exactly once, in memory and on disk.
    #[test]
    fn failed_save_leaves_add_operation_unapplied() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let dir = tmp.path().join("state");
        let path = dir.join("transactions.json");
        let mut manager = TransactionManager::new(path.clone()).expect("new manager");
        manager.begin(None).expect("begin");
        manager.add_operation("op-1".to_string()).expect("op-1");

        break_log_file(&dir.join("transactions.json"));
        assert!(manager.add_operation("op-2".to_string()).is_err());
        let ops = &manager.active().expect("still active").operation_ids;
        assert_eq!(
            ops,
            &["op-1".to_string()],
            "a failed save must not record op-2"
        );

        repair_log_file(&dir.join("transactions.json"));
        manager
            .add_operation("op-2".to_string())
            .expect("retry op-2");
        let reloaded = TransactionManager::new(path).expect("reload");
        assert_eq!(
            reloaded.active().expect("active on disk").operation_ids,
            ["op-1".to_string(), "op-2".to_string()],
            "memory and disk agree, op-2 recorded exactly once"
        );
    }

    /// T-5: a commit or rollback whose save fails leaves the transaction Active,
    /// and the retry settles it.
    #[test]
    fn failed_save_leaves_commit_and_rollback_unapplied() {
        for commit in [true, false] {
            let tmp = TempDir::new().expect("failed to create temp dir");
            let dir = tmp.path().join("state");
            let mut manager =
                TransactionManager::new(dir.join("transactions.json")).expect("new manager");
            manager.begin(None).expect("begin");
            manager.add_operation("op-1".to_string()).expect("op-1");

            break_log_file(&dir.join("transactions.json"));
            let settled = if commit {
                manager.commit()
            } else {
                manager.mark_rolled_back()
            };
            assert!(settled.is_err());
            assert!(manager.has_active(), "a failed save must leave it Active");
            assert_eq!(
                manager.active().expect("active").state,
                TransactionState::Active
            );

            repair_log_file(&dir.join("transactions.json"));
            let tx = if commit {
                manager.commit()
            } else {
                manager.mark_rolled_back()
            }
            .expect("retry settles");
            assert_eq!(
                tx.state,
                if commit {
                    TransactionState::Committed
                } else {
                    TransactionState::RolledBack
                }
            );
            assert!(!manager.has_active());
        }
    }

    /// T-5: a save replaces the log by rename, leaves no temporary file, and is
    /// not confused by a torn temporary file left by an earlier crash.
    #[test]
    fn save_replaces_atomically_and_leaves_no_temporary_file() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let path = tmp.path().join("transactions.json");
        // A torn temporary file left by an earlier crash must not matter, and
        // must not be touched: it may belong to a save still in flight.
        let stale = "transactions.json.99999.0.tmp";
        fs::write(tmp.path().join(stale), b"{ torn").expect("plant stale tmp");

        let mut manager = TransactionManager::new(path.clone()).expect("new manager");
        manager.begin(None).expect("begin");
        manager.add_operation("op-1".to_string()).expect("op-1");

        assert_eq!(
            temporary_files(tmp.path()),
            [stale.to_string()],
            "every save's own temporary file is renamed away"
        );
        assert_eq!(
            fs::read(tmp.path().join(stale)).expect("stale file"),
            b"{ torn"
        );
        let reloaded = TransactionManager::new(path).expect("log parses after saves");
        assert_eq!(
            reloaded.active().expect("active").operation_ids,
            ["op-1".to_string()]
        );
    }

    /// T-5: two saves never share a temporary file, so one writer cannot
    /// truncate, rename or delete the file another is still writing.
    #[test]
    fn each_save_uses_its_own_temporary_file() {
        let path = std::path::Path::new("dir/transactions.json");
        let (a, b) = (unique_tmp_path(path), unique_tmp_path(path));
        assert_ne!(a, b);
        for tmp in [&a, &b] {
            assert_eq!(
                tmp.parent(),
                path.parent(),
                "a sibling, so the rename is atomic"
            );
            assert!(tmp.to_string_lossy().ends_with(".tmp"));
        }

        // And the open refuses to adopt a file that is already there.
        let dir = TempDir::new().expect("failed to create temp dir");
        let taken = dir.path().join("taken.tmp");
        fs::write(&taken, b"another writer's bytes").expect("plant");
        assert!(create_tmp(&taken).is_err());
        assert_eq!(fs::read(&taken).expect("read"), b"another writer's bytes");
    }

    /// T-5: replacing the log by rename keeps the permissions of the log it
    /// replaces; an in-place write did, and a fresh file would silently widen them.
    #[cfg(unix)]
    #[test]
    fn save_preserves_the_logs_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().expect("failed to create temp dir");
        let path = tmp.path().join("transactions.json");
        let mut manager = TransactionManager::new(path.clone()).expect("new manager");
        manager.begin(None).expect("begin");

        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("chmod");
        manager.add_operation("op-1".to_string()).expect("op-1");
        manager.commit().expect("commit");

        let mode = fs::metadata(&path).expect("stat").permissions().mode() & 0o777;
        assert_eq!(mode, 0o640, "saves keep the log's mode");
    }

    #[test]
    fn test_cannot_begin_while_active() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let path = tmp.path().join("transactions.json");
        let mut manager =
            TransactionManager::new(path).expect("failed to create transaction manager");

        manager
            .begin(None)
            .expect("failed to begin first transaction");
        assert!(manager.begin(None).is_err());
    }
}
