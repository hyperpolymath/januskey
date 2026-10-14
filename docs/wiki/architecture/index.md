<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
<!-- Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk> -->

# Architecture Overview

## System Design

JanusKey follows a layered architecture that separates concerns and enables the formal guarantees described in the theoretical foundations.

        +--------------------------------------------------+
        |                    CLI Layer                      |
        |              (jk command interface)               |
        +--------------------------------------------------+
                               |
        +--------------------------------------------------+
        |                Operation Layer                    |
        |    (FileOperation enum, OperationExecutor)       |
        +--------------------------------------------------+
                               |
        +--------------------------------------------------+
        |              Transaction Manager                  |
        |         (begin, commit, rollback)                |
        +--------------------------------------------------+
                               |
               +---------------+---------------+
               |                               |
        +------v------+                 +------v------+
        |   Content   |                 |  Metadata   |
        |    Store    |                 |    Store    |
        +------+------+                 +------+------+
               |                               |
        +------v------+                 +------v------+
        | SHA256 Hash |                 |  JSON Log   |
        | File System |                 |  Append-Only|
        +-------------+                 +-------------+

## Components

### CLI Layer

Location: `crates/januskey-cli/src/main.rs`

The CLI provides the user interface for JanusKey operations:

``` rust
#[derive(Parser)]
#[command(name = "jk")]
#[command(about = "JanusKey: Provably Reversible File Operations")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

enum Commands {
    Init,           // Initialize .januskey directory
    Delete { files: Vec<PathBuf> },
    Modify { path: PathBuf, content: String },
    Move { source: PathBuf, destination: PathBuf },
    Copy { source: PathBuf, destination: PathBuf },
    Mkdir { path: PathBuf, parents: bool },
    Rmdir { path: PathBuf, recursive: bool },
    // ... more commands
    Undo { count: usize },
    Begin { name: Option<String> },
    Commit,
    Rollback,
    History,
    Status,
}
```

See: [CLI Guide](../guides/cli.md)

### Operation Layer

Location: `crates/januskey-cli/src/operations.rs`

Defines all reversible file operations and their execution logic.

#### FileOperation Enum

``` rust
pub enum FileOperation {
    Delete { path: PathBuf },
    Modify { path: PathBuf, new_content: Vec<u8> },
    Move { source: PathBuf, destination: PathBuf },
    Copy { source: PathBuf, destination: PathBuf },
    Create { path: PathBuf, content: Vec<u8> },
    Chmod { path: PathBuf, new_mode: u32 },
    Mkdir { path: PathBuf, parents: bool },
    Rmdir { path: PathBuf },
    RmdirRecursive { path: PathBuf },
    Symlink { target: PathBuf, link_path: PathBuf },
    Append { path: PathBuf, content: Vec<u8> },
    Truncate { path: PathBuf, new_size: u64 },
    Touch { path: PathBuf, create: bool },
}
```

#### OperationExecutor

Handles the execution of operations with pre-capture and metadata recording:

``` rust
pub struct OperationExecutor<'a> {
    content_store: &'a ContentStore,
    metadata_store: &'a mut MetadataStore,
    transaction_id: Option<String>,
}

impl OperationExecutor {
    pub fn execute(&mut self, operation: FileOperation) -> Result<OperationMetadata>;
    pub fn undo(&mut self, operation_id: &str) -> Result<OperationMetadata>;
}
```

See: [Operations Reference](../operations/index.md)

### Transaction Manager

Location: `crates/januskey-cli/src/transaction.rs`

Provides transaction support for grouping operations.

``` rust
pub struct TransactionManager {
    path: PathBuf,
    log: TransactionLog,
}

impl TransactionManager {
    pub fn begin(&mut self, name: Option<String>) -> Result<&Transaction>;
    pub fn commit(&mut self) -> Result<Transaction>;
    pub fn rollback(
        &mut self,
        content_store: &ContentStore,
        metadata_store: &mut MetadataStore,
    ) -> Result<Transaction>;
    pub fn active(&self) -> Option<&Transaction>;
}
```

See: [Transaction Guide](../guides/transactions.md)

### Content Store

Location: `crates/januskey-cli/src/content_store.rs`

Content-addressed storage with SHA256 hashing and optional compression.

``` rust
pub struct ContentStore {
    root: PathBuf,
    compression: bool,
}

impl ContentStore {
    pub fn store(&self, content: &[u8]) -> Result<ContentHash>;
    pub fn retrieve(&self, hash: &ContentHash) -> Result<Vec<u8>>;
    pub fn exists(&self, hash: &ContentHash) -> bool;
}
```

#### Storage Layout

    .januskey/content/
    ├── ab/
    │   ├── cdef1234...    # First 2 chars of hash as directory
    │   └── cdef5678.gz    # Compressed if enabled
    ├── cd/
    │   └── ef123456...
    └── ...

See: [Content Store Details](content-store.md)

### Metadata Store

Location: `crates/januskey-cli/src/metadata.rs`

Append-only operation log in JSON format.

``` rust
pub struct MetadataStore {
    path: PathBuf,
    log: OperationLog,
}

pub struct OperationMetadata {
    pub id: String,
    pub op_type: OperationType,
    pub timestamp: DateTime<Utc>,
    pub user: String,
    pub path: PathBuf,
    pub path_secondary: Option<PathBuf>,
    pub content_hash: Option<ContentHash>,
    pub original_metadata: Option<FileMetadata>,
    pub undone: bool,
    // ...
}
```

See: [Metadata Store Details](metadata-store.md)

## Data Flow

### Execute Operation Flow

        User Request
             |
             v
        +----+----+
        |   CLI   |
        +----+----+
             |
             v
        +----+--------+
        | Validate    |
        | Arguments   |
        +----+--------+
             |
             v
        +----+--------+     +-------------+
        | Pre-capture |---->| Content     |
        | Content     |     | Store       |
        +----+--------+     +-------------+
             |
             v
        +----+--------+     +-------------+
        | Record      |---->| Metadata    |
        | Metadata    |     | Store       |
        +----+--------+     +-------------+
             |
             v
        +----+--------+
        | Execute     |
        | File Op     |
        +----+--------+
             |
             v
        Success Response

### Undo Operation Flow

        Undo Request
             |
             v
        +----+--------+     +-------------+
        | Load        |<----| Metadata    |
        | Metadata    |     | Store       |
        +----+--------+     +-------------+
             |
             v
        +----+--------+     +-------------+
        | Retrieve    |<----| Content     |
        | Content     |     | Store       |
        +----+--------+     +-------------+
             |
             v
        +----+--------+
        | Construct   |
        | Inverse Op  |
        +----+--------+
             |
             v
        +----+--------+
        | Execute     |
        | Inverse     |
        +----+--------+
             |
             v
        +----+--------+     +-------------+
        | Mark        |---->| Metadata    |
        | Undone      |     | Store       |
        +----+--------+     +-------------+
             |
             v
        Success Response

## Directory Structure

    project/
    ├── .januskey/
    │   ├── config.json       # Configuration
    │   ├── content/          # Content-addressed storage
    │   │   ├── ab/
    │   │   ├── cd/
    │   │   └── ...
    │   ├── metadata.json     # Operation log
    │   └── transactions      # Transaction log
    └── ... (project files)

## Configuration

Located at `.januskey/config.json`:

``` json
{
  "storage_path": "/home/user/.local/share/januskey",
  "compression": true,
  "max_history": 10000,
  "auto_confirm": false,
  "dry_run_default": false,
  "audit_enabled": true
}
```

| Option          | Default | Description                                |
|-----------------|---------|--------------------------------------------|
| compression     | true    | Enable gzip compression for stored content |
| max_history     | 10000   | Maximum operations to keep in history      |
| auto_confirm    | false   | Skip confirmation for dangerous operations |
| dry_run_default | false   | Default to dry-run mode                    |
| audit_enabled   | true    | Enable audit logging                       |

## Error Handling

JanusKey uses a custom error type for comprehensive error handling:

``` rust
pub enum JanusError {
    FileNotFound(String),
    PathExists(String),
    DirectoryNotFound(String),
    NotInitialized(String),
    ContentIntegrityError { expected: String, actual: String },
    MetadataCorrupted(String),
    TransactionActive(String),
    NoActiveTransaction,
    InvalidOperationId(String),
    OperationFailed(String),
    Io(std::io::Error),
    // ...
}
```

## Security Model

### Integrity

- Content verified on retrieval via SHA256

- Metadata stored atomically

- Operations fail-safe (fail before corrupting state)

### Confidentiality

- Content stored locally (no network transmission)

- File permissions preserved

- No secrets in metadata

### Availability

- Self-contained (no external dependencies)

- Deduplication prevents storage exhaustion

- Garbage collection for old content

## Further Reading

- [Content Store Deep Dive](content-store.md)

- [Metadata Store Deep Dive](metadata-store.md)

- [Theoretical Foundations](../theory/index.md)
