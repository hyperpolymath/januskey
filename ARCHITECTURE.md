<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->
# JanusKey Architecture — Reversibility Stack Junction Point

## Lineage

```
maa-framework (policy vision)
  → absolute-zero (certified null operations — formal theory)
    → januskey (development proof-of-concept — this repo)
      → THREE downstream applications:
         ├── ochrance    — neurosymbolic filesystem verification (Idris2)
         ├── valence-shell — formally verified reversible shell (Rust + 6 proof systems)
         └── aletheia    — reversible OS operations (early research)
```

JanusKey is the **junction point** where absolute-zero's theoretical work on Certified
Null Operations (CNOs) was first applied to practical file operations. The three
downstream applications independently implemented reversibility, but shared no code —
until the `reversible-core` extraction described below.

## Workspace Structure

```
januskey/
├── crates/
│   ├── reversible-core/    ← SHARED LIBRARY (the integration surface)
│   │   ├── content_store   — SHA256 content-addressed storage
│   │   ├── metadata        — OperationMetadata + MetadataStore (append-only log)
│   │   ├── transaction     — Transaction lifecycle (begin/commit/rollback)
│   │   ├── manifest        — A2ML emitter (bridge to ochrance verification)
│   │   ├── error           — ReversibleError types
│   │   └── lib             — ReversibleExecutor trait
│   │
│   └── januskey-cli/       ← CLI TOOL (depends on reversible-core)
│       ├── operations      — FileOperation executor (actual filesystem ops)
│       ├── keys            — Key management (AES-GCM, Argon2)
│       ├── attestation     — Audit trail
│       ├── obliteration    — Secure deletion
│       ├── delta           — Differential operations
│       ├── main            — jk CLI binary
│       └── keys_cli        — jk-keys CLI binary
│
└── src/januskey/           ← LEGACY (pre-extraction, superseded by crates/)
```

## reversible-core: The Shared Foundation

`reversible-core` is a lean Rust library crate (no CLI deps) that provides the types
all three downstream applications share:

### ReversibleExecutor Trait

```rust
pub trait ReversibleExecutor {
    type Op;
    type Metadata;
    type Error;

    fn execute(&mut self, op: Self::Op) -> Result<Self::Metadata, Self::Error>;
    fn undo(&mut self, metadata_id: &str) -> Result<Self::Metadata, Self::Error>;
    fn generate_manifest(&self) -> Result<String, Self::Error>;
}
```

This is the **Rust-side mirror** of ochrance's `VerifiedSubsystem` interface (Idris2).
The `generate_manifest` method emits A2ML that ochrance can parse and verify.

### CNO Correspondence

Per absolute-zero, every `OperationType` has a known inverse:

| Operation | Inverse | Property |
|-----------|---------|----------|
| Delete | Create | `delete ; create ≡ CNO` |
| Create | Delete | `create ; delete ≡ CNO` |
| Modify | Modify | Self-inverse (stores old+new content) |
| Move | Move | Self-inverse (swap src/dst) |
| Copy | Delete | `copy ; delete_copy ≡ CNO` |
| Chmod | Chmod | Self-inverse (stores old mode) |
| Chown | Chown | Self-inverse (stores old uid:gid) |

### A2ML Bridge to Ochrance

`ManifestEmitter::generate()` produces A2ML manifests containing:
- Manifest header (version, subsystem, timestamp, Merkle root)
- Refs (one per operation, with content hash)
- Policy (verification mode)

Ochrance parses these and produces `VerificationProof` witnesses:
- `LaxProof` — manifest is well-formed
- `CheckedProof` — all content hashes verified via BLAKE3
- `AttestedProof` — manifest is signed (Ed25519)

## Integration Status

### Phase 1: reversible-core extraction ✅ DONE (2026-03-21)

Extracted core types from januskey into `reversible-core`. Workspace builds,
44 tests pass. januskey-cli re-exports all types for backward compatibility.

### Phase 2: valence-shell integration — NEXT

Wire valence-shell (`impl/rust-cli/`) to depend on `reversible-core`:
- Add `ContentStore` to `ShellState` for content-addressed undo data
- Replace inline `undo_data: Option<Vec<u8>>` for large file content
- Add `a2ml_emitter.rs` for manifest generation
- Replace `verification.rs` stubs with `ReversibleExecutor` implementation

### Phase 3: ochrance ABI extension — PENDING

Add reversibility types to ochrance's Idris2 ABI:
- `src/abi/Ochrance/ABI/Reversibility.idr` — `ReversibleOp`, `ReversibilityProof`
- `ochrance-core/Ochrance/Subsystem/OperationLog.idr` — `VerifiedSubsystem` for op logs

### Phase 4: Cross-repo documentation — PENDING

Update ARCHITECTURE.md and ECOSYSTEM.a2ml in all three repos with cross-references.

### Deferred

- **aletheia**: Too early (Phase 0 research, no operation types yet)
- **Merkle tree compatibility**: ochrance uses BLAKE3 height-indexed trees; Rust side
  needs compatible implementation
