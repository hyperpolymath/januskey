# ADR-0001 — Repository identity: reversible file operations, and what the key store is for

- **Status:** accepted, partially applied
- **Date:** 2026-11-01
- **Supersedes:** nothing formal. Corrects the identity claims in `README.adoc`,
  `PROOF-NEEDS.adoc` §Priority, `ARCHITECTURE.adoc`, `crates/januskey-cli/src/lib.rs`
  and `.machine_readable/contractiles/Trustfile.a2ml` (the last needs a human; see §6).
- **Related:** estate audit `docs/estate/2026-11-01-handoff-and-consistency-audit.md`,
  invariant 7 ("januskey is reversible file operations with inversion metadata; it is
  **not** a cryptographic key system").

## 1. Context

The estate rule is that semantic boundaries follow implementation, not repository
names. `epistemic-types@5422cd5` is the current example of that rule being applied to
another repo; this ADR applies it here.

JanusKey's name contains "Key", its README describes reversible file operations, its
workspace builds a second binary called `jk-keys`, and its Trustfile asserts
"Januskey is a key-management tool". At least one of those four is wrong, and the
name is the least trustworthy of them.

So: measure the coupling rather than argue about it.

## 2. What the code actually contains

| Component | Lines | Binary | Used by `jk`? | Evidence |
|---|---:|---|---|---|
| `operations.rs`, `delta.rs`, `obliteration.rs` | 1,900 | `jk` | yes | `main.rs` dispatch |
| `reversible-core` (store, log, transaction, manifest) | 2,202 | — | yes | `lib.rs` re-exports |
| `main.rs` + `lib.rs` | 1,220 | `jk` | — | — |
| `attestation.rs` (HMAC-chained audit log) | 627 | — | **no** | `main.rs` contains no `AuditLog` or `attestation::` reference |
| `keys.rs` (key store: Argon2id, AES-256-GCM wrap, rotate, revoke, backup) | 735 | **`jk-keys`** | **no** | `main.rs` contains no `keys::`, `KeyManager` or `keys_cli` reference |
| `keys_cli.rs` | 701 | **`jk-keys`** | **no** | separate `[[bin]]` |
| `docs/security/KEY_LIFECYCLE.adoc` | 554 | — | — | documents `jk-keys` only |

Dependency direction, from `use` statements:

```
main.rs (jk)  →  operations / delta / obliteration  →  reversible-core
keys.rs       →  attestation.rs (AuditLog)          →  hmac, sha2
keys_cli.rs   →  keys.rs, attestation.rs
```

There is **no edge in either direction** between the reversibility path and the key
subsystem. `AttestationKey` is an opaque `[u8; 32]` handed to `AuditLog::init`; it is
not derived from `KeyManager` anywhere in the tree, and nothing in `jk` ever sets one.
`KeyManager` owns an `AuditLog`, not the other way round.

Two further facts that the identity statement has to survive:

1. **The audit chain is not wired into the CLI.** The `jk` binary never constructs an
   `AuditLog`, so "every change is tracked automatically" was never true of the CLI.
   The chain is exercised by unit tests only, and obliteration records go to an
   unkeyed JSON ledger (`obliterations.json`).
2. **The key store protects nothing on the reversibility path.** Content-store blobs
   are plaintext, gzip-optionally-compressed files addressed by SHA-256. AES-256-GCM
   is used to wrap *key material inside the key store*, not file content.

## 3. Decision

**JanusKey's identity is reversible file operations.** Concretely:

- The product is the `jk` binary: `init, delete, modify, move, copy, rename,
  obliterate, undo, begin, commit, rollback, preview, history, status, gc`.
- SHA-256 content addressing and the append-only operation log are part of that
  product. They are not "crypto" in the identifying sense; they are storage.
- **The key lifecycle manager is not part of the product.** `jk-keys` is a separate
  binary with its own store format, its own lifecycle state machine (generated /
  active / rotating / suspended / revoked / obliterated), and zero coupling to
  reversibility. It stays in this repository *for now* behind an opt-in feature
  (`key-lifecycle`), so nothing is lost, and it leaves the default binary set and the
  default identity.
  Be precise about what that gate does and does not do: it stops `cargo build
  --workspace` from producing `jk-keys`; it does not stop `keys.rs` from compiling
  into the library, and it does not remove `aes-gcm`/`argon2` from the dependency
  graph. Those go with the module move in §5. A feature gate is a statement about
  the product, not a security boundary.
- The HMAC-chained audit log stays in the library and is honestly described as
  **library-only, not wired into the CLI**. Wiring it is a separate decision and
  requires a key-provisioning story that does not exist yet.

The invariant "not a cryptographic key system" is therefore treated as **mandatory
for the product, and aspirational for the module**: the shipped default build no
longer offers key custody, and the remaining module is recorded here as a candidate
for extraction rather than being described as the repo's purpose.

Why not simply move the module today: this sandbox has no `rustc`, no `cargo` and no
crates.io access, so a crate move could not be built or tested. The estate rule is
that a claim must be checkable; "the split compiles" is not checkable from here.
§5 gives the exact steps for whoever has a toolchain.

Why not keep it and rewrite the identity as "a key-management tool that also does
reversibility": because the measurement in §2 says the reverse — reversibility is the
product, the key manager is a detached appendage with no callers. Rewriting the
identity to match the Trustfile's sentence would be inferring purpose from a name and
a comment, which is the error this ADR exists to correct.

## 4. Valence Shell: what may and may not be claimed

`reversible-core` was extracted on 2026-03-21 (Phase 1) so that three downstream
projects could consume it. **None of them does.** There is no `Cargo.toml` in this
tree depending on it from outside the workspace, no test that exercises a downstream
consumer, and no way from here to see another repo's dependency graph.

Therefore, from now on:

- **May be claimed:** the crate exists, builds in this workspace, is `#![forbid(unsafe_code)]`, and is *intended* to be the shared basis for reversible operation data.
- **May not be claimed:** that valence-shell, ochrance or aletheia use it, partly use it, or are "wired" to it. `ARCHITECTURE.adoc` Phase 2/3 are unstarted; the Merkle/hash mismatch (SHA-256 here vs BLAKE3 believed there) is unresolved, and is itself a reason a consumer cannot yet be wired.
- **Evidence that would make it claimable:** a dependency edge in the consuming repo's manifest, a passing test there that exercises `ReversibleExecutor`, and — for ochrance — a settled hash algorithm at the manifest boundary.

The same rule applies to the A2ML manifest: `ManifestEmitter` produces text that
ochrance *is intended* to parse. Ochrance has never parsed it. The format doc comment
now says so.

## 5. Consequences and the remaining split

Applied now (verifiable by inspection, no build required):

- `crates/januskey-cli/Cargo.toml`: `[features] key-lifecycle = []`; `[[bin]] jk-keys`
  gains `required-features = ["key-lifecycle"]`. Default `cargo build --workspace`
  therefore produces `jk` and `dashboard-check`, and omits `jk-keys`.
- Identity corrected in `README.adoc`, `PROOF-NEEDS.adoc`, `ARCHITECTURE.adoc`,
  `crates/januskey-cli/src/lib.rs`, `docs/security/KEY_LIFECYCLE.adoc`,
  `.machine_readable/6a2/STATE.a2ml`.
- Consumption status pinned in `ARCHITECTURE.adoc` and as ledger rows `V-1`, `V-2` in
  `PROOF-NEEDS.adoc`, so "valence-shell consumes reversible-core" cannot drift back
  into prose without a row changing status.

Not applied — needs a toolchain:

1. Create `crates/januskey-keys` with `keys.rs`, `keys_cli.rs` and their dependencies
   (`aes-gcm`, `argon2`, `rand`, `zeroize`, `uuid`, `chrono`, `base64`, `subtle`),
   depending on `januskey` for `attestation::AuditLog`.
2. Cut the two import edges: `use crate::attestation::AuditLog` → `use januskey::attestation::AuditLog`;
   `use januskey::keys::…` → `use januskey_keys::…`.
3. Remove the three `KeyAlgorithm` references in `attestation.rs`'s test module (lines
   ~524, ~568, ~611) to avoid a dev-dependency cycle, or make them a dev-dependency of
   the key crate.
4. Drop `pub mod keys;` and the `pub use keys::{…}` re-export from `januskey`, and the
   key dependencies from `crates/januskey-cli/Cargo.toml`.
5. **Acceptance:** `cargo build -p januskey` does not compile `keys.rs`; `cargo tree -p januskey`
   lists neither `aes-gcm` nor `argon2`; `cargo test --workspace` green; the Trustfile
   rule `no-metadata-only-key-types` passes in the repo that ends up owning the module.

After that, `jk-keys` can move to whichever repo needs key custody. That is not this one.

## 6. What a human has to do

Two files contradict this ADR and are **agent-denied** by the Trustfile
(`@trust-deny: [modify-trustfile, modify-threat-model]`). An agent must not edit them,
and this ADR does not:

| File | Line | Wrong statement | Needed |
|---|---|---|---|
| `.machine_readable/contractiles/Trustfile.a2ml` | 8–10 | "Januskey is a key-management tool: any LLM/agent interaction with it must treat every key, audit entry…" | One sentence from the owner; the crypto-specific deny-list is still correct if the key module stays |
| `.machine_readable/threat-model.a2ml` | `[primitives.mac]` | Still describes the home-rolled `SHA256(key‖data‖prev)` as the live construction | Mark resolved: `attestation.rs` now uses real HMAC-SHA256 |

Also owner-decided, out of agent scope: the crate version `1.0.0` against README's
alpha/D-phase status, and whether `jk-keys` should be published at all once split.

## 7. Rejected alternatives

- **Rename the repo.** Names do not fix coupling; `jk-keys` in a repo called
  `reversible-fs` is the same defect.
- **Delete `keys.rs` outright.** It is 1,436 lines of working, tested code with a
  legitimate future owner, and deletion here would be unverifiable (no build) and
  irreversible in the estate's sense.
- **Keep it and call the repo a key-management tool.** Contradicted by §2: no caller,
  no content encryption, no CLI wiring.
- **Declare the invariant satisfied today.** It is not: `keys.rs` is still in the
  library's public API (`pub mod keys`). §3 says so rather than claiming the win.
