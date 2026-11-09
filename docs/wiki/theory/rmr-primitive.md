<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
<!-- Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk> -->

# RMR: Reversible Transaction Primitive

> **Status: an engineering mechanism with a partially proved model.**
> JanusKey records the inverse of each operation and can replay it. Three
> lemmas about the transaction *model* are proved in Agda; the step-level
> premise, two failure-path premises and the atomicity of the metadata log are
> not, and the headline claim "every operation is reversible" is an OPEN row in
> `PROOF-NEEDS.adoc`. This page previously stated the property as a theorem
> with a proof sketch and a `∎`; the correction is recorded in
> `../../../docs/estate/2026-11-01-handoff-and-consistency-audit.md` (D1).

## Definition

**RMR (Reversible Transaction)** is the property JanusKey aims at: for every
operation `F` it applies to state `S`, it records enough to apply an inverse
`F⁻¹` with `F⁻¹(F(S)) = S`.

Whether that holds in general is **open**. What holds today, concretely:

- each operation type has a named inverse (`OperationType::inverse`:
  `Delete↔Create`, `Modify↔Modify`, `Move↔Move`, `Copy→Delete`, `Chmod`,
  `Chown` self-inverse);
- the information needed for that inverse is captured *before* the mutation
  (content into the store, file metadata into the log);
- `execute` then `undo` restores the prior state under property testing
  (`property_tests::execute_then_undo_is_identity`);
- rolling back a whole transaction in reverse order restores the state it
  began in, **given** that each step's undo restores its execute. That
  conditional is a proved Agda lemma (T-1); the premise itself is tested, not
  proved (T-6).

## Core properties, at the strength the code supports

### 1. Reversal is determined, not searched

The inverse is a function of the recorded metadata, not of a filesystem scan:
undo is a lookup in the append-only log followed by the operation-specific
inverse in `delta.rs` / `operations.rs`. "Determined" is the honest word — the
metadata is the sole input, and if it is lost the reversal is not merely
ambiguous, it is unavailable.

### 2. What a reversal restores

Capture and restore are **not** the same set, and the difference matters:

| Aspect | Captured? | Restored on undo? |
|---|---|---|
| File content | yes — content store, SHA-256 | yes, verified against the hash on read |
| Permissions | yes (`FileMetadata.permissions`) | yes — `FileMetadata::apply` sets the mode |
| Owner / group | yes (`owner`, `group`) | **no** — no `chown` call exists; values are recorded for the record's sake |
| Modification time | yes (`modified`) | **no** — no `utimensat`/`set_times` call exists |
| Symlink target | yes (`is_symlink`, `symlink_target` via `read_link`) | **no** — no `symlink()` call exists, so a deleted symlink is not recreated as a symlink |
| Directory structure | per-file: a recursive delete logs one operation per entry | recreated only as far as the individual file undos recreate their parents |

So "complete original state, byte-for-byte" is not true of this
implementation. It is true of content and permissions on Unix; the rest is
recorded but not applied.

### 3. Sequential composition

Undo runs in reverse order, and composition is the proved model statement:

```
undo(Oₙ) ∘ ⋯ ∘ undo(O₁) ∘ O₁ ∘ ⋯ ∘ Oₙ(S₀) = S₀
```

This is ledger row **T-1** (`TransactionAtomicity.rollback-restores`, PROVEN in
Agda over the log model) plus the rejected fixture
`tests/reject-agda/ForwardRollback.agda`. It is conditional on T-6; it is not a
statement about bytes on a device.

### 4. Transaction semantics: recorded and resumable, not isolated

| Claim | Reality |
|---|---|
| Commit/rollback settle a group of operations | yes — `TransactionManager`, states `Active`/`Committed`/`RolledBack`; at most one active (T-3, PROVEN) |
| A rollback that fails part-way can be retried | yes (T-4, TESTED — `rollback_resume_test`, driving the real `jk`) |
| A failed save leaves memory and disk agreeing | yes for the transaction log (T-5, TESTED: replaced by rename, never overwritten in place, with permission preservation; the fsync before the rename is by construction, not tested) |
| The **operation metadata** log has the same guarantee | yes for the log itself (T-8, TESTED): an edit whose save fails is not applied and applies exactly once on retry, and an append whose save fails keeps its record for the next save. **Not** for the file change it records: `execute_*` changes the filesystem before appending, and `undo` runs the inverse before `mark_undone`, so a crash or failed save between the two leaves a change the log does not know about (T-9, **OPEN**) |
| A failed execute leaves the filesystem unchanged | assumed, not checked (T-6) |
| Intermediate states are invisible to other processes | **false** — operations are applied to the filesystem one at a time as they are executed. A transaction gives you a recorded, resumable rollback, not isolation. Nothing in this tool locks, stages, or hides the working tree |

## Protocol, as a sketch

The phases below are the intended shape, not the call surface: `PreCaptureData`
and `StorageReceipt` are illustrative types, and the real API is
`ReversibleExecutor` in `crates/reversible-core/src/lib.rs` plus
`OperationExecutor` in `crates/januskey-cli/src/operations.rs`. Read them as
prose with braces.

### Phase 1: Pre-capture

```rust
// For a DELETE operation
fn pre_capture_delete(path: &Path) -> PreCaptureData {
    PreCaptureData {
        content: fs::read(path),
        metadata: FileMetadata::from_path(path),
        content_hash: sha256(content),
    }
}
```

### Phase 2: Content-addressed storage

```rust
fn secure_store(data: PreCaptureData) -> StorageReceipt {
    let hash = content_store.store(data.content)?;
    StorageReceipt { hash, verified: true }
}
```

### Phase 3: Metadata recording

```rust
fn record_metadata(op: Operation, receipt: StorageReceipt) -> OperationMetadata {
    OperationMetadata {
        id: uuid::new_v4(),
        op_type: op.type(),
        timestamp: Utc::now(),
        user: current_user(),
        path: op.path(),
        content_hash: Some(receipt.hash),
        original_metadata: Some(op.original_metadata()),
    }
}
```

### Phase 4: Execute

```rust
fn execute(op: Operation) -> Result<()> {
    // The recorded metadata is now the only thing between this operation
    // and unrecoverability. If the append below fails, the inverse is lost.
    op.perform()?;
    metadata_store.append(metadata)?;
    Ok(())
}
```

Ordering note, because this is where the guarantee is thinnest: in
`execute_delete` the content is stored **before** the unlink and the metadata is
appended **after** it (`operations.rs`). A crash between the unlink and the
append loses the ability to undo that delete. T-6 and T-9 are the ledger rows
that would have to close to remove that window.

### Phase 5: Reversal

```rust
fn undo(operation_id: &str) -> Result<()> {
    let metadata = metadata_store.get(operation_id)?;
    let content = content_store.retrieve(&metadata.content_hash)?;
    let inverse = construct_inverse(&metadata, content);
    inverse.perform()?;
    metadata_store.mark_undone(operation_id)?;   // T-9: runs after the inverse (OPEN)
    Ok(())
}
```

## Operation-specific inverses

Taken from `OperationType::inverse` (`crates/reversible-core/src/metadata.rs`).
There is no `Mkdir`/`Rmdir` operation type: a directory is deleted by walking it
and logging one `Delete` per entry.

| Forward | Inverse | What is stored for the inverse |
|---|---|---|
| `Delete` | `Create` | content hash + original metadata |
| `Create` | `Delete` | nothing (the file did not exist) |
| `Modify` | `Modify` | original hash + new hash; `delta.rs` stores a full copy or a line/block delta |
| `Move` | `Move` | source and destination paths; the inverse swaps them |
| `Copy` | `Delete` | destination path; the inverse removes the copy |
| `Chmod` | `Chmod` | original mode |
| `Chown` | `Chown` | original owner/group |

`Chmod` and `Chown` exist in the model and the operation layer but are **not**
exposed as `jk` subcommands; the CLI surface is `init, delete, modify, move,
copy, rename, obliterate, undo, begin, commit, rollback, preview, history,
status, gc`.

## Formal status

There is no single "RMR correctness" theorem here, and this page should not
present one until it exists. What exists, by ledger row:

| Row | Statement | Status |
|---|---|---|
| T-1 | Undoing a fully executed transaction in reverse order restores the state it began in, given each step's undo restores its execute | **PROVEN** (Agda, model) |
| T-2 | In any single-active log, the transitions preserve the invariant and rewrite only the active record (januskey's transactions are an instance) | **PROVEN** (Agda, abstract) |
| T-3 | In any single-active log, at most one record is active | **PROVEN** (Agda, abstract) |
| T-4 | A partially failed rollback can be retried to completion | **TESTED** |
| T-5 | A failed transaction-log save leaves memory unchanged; the log is replaced by rename with permissions preserved (fsync by construction) | **TESTED** |
| T-6 | A failed execute leaves the filesystem unchanged; each undo restores its execute's pre-state | **ASSUMED** |
| T-7 | Transaction ids never collide | **ASSUMED** |
| T-8 | The operation metadata log is replaced by rename; a failed edit is unapplied and retries once; a failed append keeps its record | **TESTED** |
| T-9 | A file change whose log record fails to persist is recorded or reverted | **OPEN** — needs a write-ahead intent record |
| T-10 | The Rust `TransactionManager` refines the abstract single-active log | **OPEN** — engineering, for Creusot or Verus |
| — | "Every operation is reversible" as a theorem linked to the Rust | **OPEN** |
| G-2 | The Idris2 and Agda presentations correspond | **ASSUMED** — "by transcription, no mechanical link claimed" |

## Implementation details

### Content store

```rust
pub struct ContentStore { root: PathBuf, compression: bool }

impl ContentStore {
    pub fn store(&self, content: &[u8]) -> Result<ContentHash> {
        let hash = ContentHash::from_bytes(content);   // sha256:<hex>
        let path = self.content_path(&hash);
        if path.exists() { return Ok(hash); }          // dedup
        self.write_content(&path, content)?;
        Ok(hash)
    }

    pub fn retrieve(&self, hash: &ContentHash) -> Result<Vec<u8>> {
        let content = self.read_content(&self.content_path(hash))?;
        if !hash.verify(&content) { return Err(JanusError::ContentIntegrityError); }
        Ok(content)
    }
}
```

Deduplication has a consequence worth knowing: a blob can back several paths, so
"delete the blob" is not the same as "delete this file's history" — see the
shared-blob rule in [RMO](rmo-primitive.md).

### Metadata store

```rust
impl MetadataStore {
    pub fn append(&mut self, metadata: OperationMetadata) -> Result<()> {
        self.log.operations.push(metadata);
        self.save()?;                                  // T-8: rename; a failed save keeps the record
        Ok(())
    }

    pub fn mark_undone(&mut self, id: &str) -> Result<()> {
        if let Some(op) = self.get_mut(id) { op.undone = true; self.save()?; }
        Ok(())
    }
}
```

The transaction log (`TransactionLog::save`) uses the atomic path in
`transaction.rs`: a per-save temporary file, mode preservation, fsync and
rename. The operation metadata log calls the same helper, and row T-8 tests it.
The sketch above simplifies the metadata store: an edit is saved before it
becomes the in-memory log, and an append whose save fails keeps its record for
the next save.

## Performance

The table below is *expected* cost derived from the algorithm, not a
measurement. No baseline has been recorded; the five Criterion groups exist but
their results are unbaselined (`READINESS.adoc` §Benchmark Classification).

| Metric | Expected | Notes |
|---|---|---|
| Storage overhead | O(content size) | with deduplication |
| Capture latency | O(file size) | one read |
| Undo latency | O(file size) | one write |
| Metadata overhead | ~500 bytes/operation | JSON serialized |

## Limitations

1. **The store is the whole guarantee.** Lose `.januskey/` and undo is gone.
2. **Metadata consistency is weaker than it looks.** The log itself saves
   safely (T-8, TESTED), but a file change and its log record are not written
   together: a crash between them leaves a change the log does not know about
   (T-9, OPEN).
3. **Not everything captured is restored** — ownership, mtime and symlink
   identity are recorded but not applied (§2).
4. **No isolation.** Operations touch the live filesystem as they execute.
5. **Local filesystem only.** No network or external side effects are reversed.
6. **Not proved against the Rust** — G-2 is ASSUMED, so the Agda model and the
   implementation are linked by review, not by machine.

## Further reading

- [RMO: Obliterative Wipe Primitive](rmo-primitive.md)
- [Formal Model of File System State](formal-model.md)
- `../../../PROOF-NEEDS.adoc` — the claims ledger, rows T-1..T-10
- [Architecture](../architecture/index.md)
