<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
<!-- Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk> -->

# RMR: Reversible Transaction Primitive

## Definition

**RMR (Reversible Transaction)** is a data primitive that guarantees the existence of a mathematically sound inverse function for every state change.

> For every operation $F$ applied to state $S$, there exists an inverse $F^{-1}$ such that $F^{-1}(F(S)) = S$.

## Core Properties

### Property 1: Deterministic Reversal

Given an operation and its metadata, the reversal is uniquely determined:

$$\text{undo}: (\text{Operation}, \text{Metadata}) \to \text{Operation}^{-1}$$

There is no ambiguity in how to reverse an operation.

### Property 2: State Preservation

The reversal restores *complete* original state, including:

- File content (byte-for-byte identical)

- File metadata (permissions, ownership, timestamps)

- Directory structure

- Symbolic link targets

### Property 3: Sequential Composition

For a sequence of operations, reversals compose correctly:

$$(O_2 \circ O_1)^{-1} = O_1^{-1} \circ O_2^{-1}$$

Operations are undone in reverse order.

### Property 4: Transaction Atomicity

Operations grouped in a transaction satisfy:

- All-or-nothing execution

- All-or-nothing reversal

- Intermediate states are not externally visible

## RMR Protocol

### Phase 1: Pre-Capture

Before any modification, capture everything needed for reversal:

``` rust
// For a DELETE operation
fn pre_capture_delete(path: &Path) -> PreCaptureData {
    PreCaptureData {
        content: fs::read(path),           // Full file content
        metadata: FileMetadata::from_path(path), // All metadata
        content_hash: sha256(content),     // For verification
    }
}
```

### Phase 2: Secure Storage

Store captured data in content-addressed storage:

``` rust
fn secure_store(data: PreCaptureData) -> StorageReceipt {
    let hash = content_store.store(data.content)?;
    // Content is now safely stored with deduplication
    StorageReceipt { hash, verified: true }
}
```

### Phase 3: Metadata Recording

Create complete operation metadata:

``` rust
fn record_metadata(op: Operation, receipt: StorageReceipt) -> OperationMetadata {
    OperationMetadata {
        id: uuid::new_v4(),
        op_type: op.type(),
        timestamp: Utc::now(),
        user: current_user(),
        path: op.path(),
        content_hash: Some(receipt.hash),
        original_metadata: Some(op.original_metadata()),
        // Everything needed for reversal
    }
}
```

### Phase 4: Execute

Only after all safety measures are in place:

``` rust
fn execute(op: Operation) -> Result<()> {
    // At this point, reversal is guaranteed possible
    op.perform()?;
    metadata_store.append(metadata)?;
    Ok(())
}
```

### Phase 5: Reversal

When undo is requested:

``` rust
fn undo(operation_id: &str) -> Result<()> {
    let metadata = metadata_store.get(operation_id)?;

    // Retrieve original content
    let content = content_store.retrieve(&metadata.content_hash)?;

    // Reconstruct inverse operation
    let inverse = construct_inverse(&metadata, content);

    // Execute reversal
    inverse.perform()?;

    // Record the undo
    metadata_store.mark_undone(operation_id)?;
    Ok(())
}
```

## Operation-Specific RMR

### DELETE → CREATE

| Forward (Delete)      | Reverse (Create)            |
|-----------------------|-----------------------------|
| Read content → Store  | Retrieve content from store |
| Read metadata → Store | Apply stored metadata       |
| Remove file           | Write file with content     |
| Log operation         | Log undo operation          |

### MODIFY → MODIFY

| Forward (Modify)              | Reverse (Modify back)       |
|-------------------------------|-----------------------------|
| Read original content → Store | Retrieve original content   |
| Write new content             | Write original content back |
| Log with both hashes          | Log undo                    |

### MOVE → MOVE

| Forward (Move A→B)      | Reverse (Move B→A)               |
|-------------------------|----------------------------------|
| Record source path      | Use stored source as destination |
| Record destination path | Use stored destination as source |
| Perform rename          | Perform rename back              |
| Log operation           | Log undo                         |

### MKDIR → RMDIR

| Forward (Mkdir)       | Reverse (Rmdir)         |
|-----------------------|-------------------------|
| Create directory      | Remove directory        |
| Log path              | Use logged path         |
| (No content to store) | (No content to restore) |

### RMDIR (recursive) → Restore

| Forward (Rmdir -r)      | Reverse (Full restore) |
|-------------------------|------------------------|
| Walk directory tree     | Read manifest          |
| Store each file content | Restore each file      |
| Store manifest          | Recreate directories   |
| Remove directory tree   | Apply metadata         |

## Formal Verification

### Theorem: RMR Correctness

For all operations $O$ and states $S$:

$$\text{undo}(\text{execute}(O, S)) = S$$

#### Proof Sketch

1.  **Pre-capture completeness**: By construction, pre-capture stores all information needed to reconstruct original state.

2.  **Storage integrity**: Content-addressed storage with SHA256 verification ensures stored content is retrieved exactly.

3.  **Metadata sufficiency**: Operation metadata contains path, original metadata, and content hash—sufficient for reconstruction.

4.  **Inverse construction**: Each operation type has a defined inverse that uses stored information.

5.  **Composition**: Undo operations are themselves operations that can be tracked. ∎

### Theorem: Sequential Reversibility (Theorem 3.4)

For operations $O_1, O_2, \ldots, O_n$:

$$\text{undo}(O_n) \circ \cdots \circ \text{undo}(O_1) \circ O_1 \circ \cdots \circ O_n(S) = S$$

#### Proof

By induction on $n$:

**Base case** ($n=1$): Direct application of RMR correctness.

**Inductive step**: Assume true for $n-1$. Then:

$$\text{undo}(O_n)(O_n(O_{n-1}(\cdots O_1(S)))) = O_{n-1}(\cdots O_1(S))$$

By inductive hypothesis, the remaining undos restore $S$. ∎

## Implementation Details

### Content Store

``` rust
pub struct ContentStore {
    root: PathBuf,
    compression: bool,
}

impl ContentStore {
    // Store content, return hash
    pub fn store(&self, content: &[u8]) -> Result<ContentHash> {
        let hash = ContentHash::from_bytes(content);
        let path = self.content_path(&hash);

        // Deduplication: skip if already stored
        if path.exists() {
            return Ok(hash);
        }

        // Write (optionally compressed)
        self.write_content(&path, content)?;
        Ok(hash)
    }

    // Retrieve with integrity verification
    pub fn retrieve(&self, hash: &ContentHash) -> Result<Vec<u8>> {
        let content = self.read_content(&self.content_path(hash))?;

        // Verify integrity
        if !hash.verify(&content) {
            return Err(JanusError::ContentIntegrityError);
        }

        Ok(content)
    }
}
```

### Metadata Store

``` rust
pub struct MetadataStore {
    path: PathBuf,
    log: OperationLog,
}

impl MetadataStore {
    // Append-only operation logging
    pub fn append(&mut self, metadata: OperationMetadata) -> Result<()> {
        self.log.operations.push(metadata);
        self.save()?;
        Ok(())
    }

    // Mark operation as undone
    pub fn mark_undone(&mut self, id: &str, undo_id: &str) -> Result<()> {
        if let Some(op) = self.get_mut(id) {
            op.undone = true;
            op.undo_operation_id = Some(undo_id.to_string());
            self.save()?;
        }
        Ok(())
    }
}
```

## Performance Characteristics

| Metric            | Value                | Notes                  |
|-------------------|----------------------|------------------------|
| Storage overhead  | O(content size)      | With deduplication     |
| Capture latency   | O(file size)         | Single read operation  |
| Undo latency      | O(file size)         | Single write operation |
| Metadata overhead | ~500 bytes/operation | JSON serialized        |

## Limitations

1.  **Requires disk space**: Original content must be stored

2.  **Not instantaneous**: Pre-capture adds latency to operations

3.  **Single-system**: RMR operates on local file system only

4.  **No external side effects**: Cannot reverse network operations, etc.

## Further Reading

- [RMO: Obliterative Wipe Primitive](rmo-primitive.md)

- [Formal Model of File System State](formal-model.md)

- [Content Store Implementation](../architecture/content-store.md)
