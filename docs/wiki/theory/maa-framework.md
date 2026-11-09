<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
<!-- Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk> -->

# Mutually Assured Accountability (MAA) Framework

## Abstract

The MAA Framework is an ethical and engineering paradigm that structurally balances two opposing requirements in data management systems:

1.  **Accountability/Auditability**: Systems must track changes for compliance, debugging, and recovery

2.  **User Autonomy/Privacy**: Users must have verifiable control over their data, including deletion

These requirements are often in tension. MAA proposes a pair of primitives that
address both sides — one reversible, one obliterative.

> **Status.** MAA is a design framework, not a verified theory. RMR and RMO are
> described below at the strength the code supports: RMR's transactional model
> has three proved lemmas and an open obligation for "every operation is
> reversible"; RMO is an engineering primitive that destroys data and records
> that it did, with no proof that the data is unrecoverable. The claims ledger in
> `PROOF-NEEDS.adoc` is the authority for both.

## The Problem Statement

### The Accountability Requirement

Modern systems require auditability for:

- **Regulatory compliance** (SOX, HIPAA, financial regulations)

- **Security forensics** (incident investigation)

- **System reliability** (debugging, recovery)

- **Business continuity** (disaster recovery)

This pushes toward: *never delete anything, log everything, keep all history*.

### The Autonomy Requirement

Users and regulations require data control:

- **GDPR Article 17** (Right to Erasure)

- **CCPA** (California Consumer Privacy Act)

- **User trust** (promise of deletion must be kept)

- **Security** (compromised data should be destroyable)

This pushes toward: *complete, verifiable deletion capability*.

### The Tension

        Accountability <---------> Autonomy
        "Keep everything"          "Delete completely"

        Traditional solutions:
        +------------------+  or  +------------------+
        | Keep all data    |      | Allow deletion   |
        | No real deletion |      | No audit trail   |
        +------------------+      +------------------+

        Result: Always compromise one goal

## The MAA Solution

MAA proposes two primitives, one for each requirement. Neither is formally
verified in full; the status of each part is stated below and in the ledger.

### RMR (Reversible Transaction)

> A primitive for state transitions that records what is needed to undo each change — so that undo is a metadata lookup rather than a restore from backup. For a delete or modify the original content is stored before the change; the operation's metadata is saved to the log after it, so a stop between the two leaves undo unable to find the operation (OPEN, T-9). Whether every operation is *in fact* reversible is an open obligation, not a guarantee; see the claims ledger.

RMR aims at the following. These are goals, not established guarantees:

- Capture of everything undo needs (the log record is written after the
  change; T-9 is OPEN)

- Reversal of every operation ("every operation is reversible" is OPEN, and
  Chown undo is not implemented)

- A full audit trail of all operations

- Accountability without risk of data loss

See: [RMR Primitive Specification](rmr-primitive.md)

### RMO (Obliterative Wipe)

> A primitive for data erasure: best-effort overwrite, removal, store scrub, and, when a JanusKey store exists, an unkeyed record of what was destroyed. The record is not tamper-evident.

RMO provides:

- An overwrite-then-unlink operation, plus scrubbing of the unshared
  content-store blobs and operation-log entries for the path

- A timestamped commitment (`H(content_hash ‖ nonce ‖ timestamp)`) recording
  what was claimed destroyed — reviewable, not independently verifiable from
  outside the store

- A deletion path with no inverse: after RMO there is nothing for undo to restore

- An honest boundary: no proof of non-existence, no physical-erasure guarantee
  on SSD/CoW/journaled media, and no compliance determination. See
  [RMO Primitive Specification](rmo-primitive.md) §"Status, in the
  claims-ledger vocabulary".

See: [RMO Primitive Specification](rmo-primitive.md)

## The MAA Dynamic

The key insight of MAA is that RMR and RMO are not alternatives—they are complementary:

                        +------------------+
                        |   MAA Framework  |
                        +--------+---------+
                                 |
             +-------------------+-------------------+
             |                                       |
        +----v----+                             +----v----+
        |   RMR   |                             |   RMO   |
        +---------+                             +---------+
        | Tracks  |                             | Removes |
        | changes |     Must be balanced        | traces  |
        | fully   |<--------------------------->| fully   |
        +---------+                             +---------+
             |                                       |
        +----v----+                             +----v----+
        |  Audit  |                             | Privacy |
        |  Trail  |                             | Control |
        +---------+                             +---------+

### Balancing Rules

1.  **RMR-by-default**: All operations use RMR for reversibility

2.  **RMO-on-request**: Users can invoke RMO to permanently erase

3.  **Audit of RMO**: The *fact* that RMO was invoked is logged (but not the erased content) when a JanusKey store exists and the record saves; see Property 3 for the cases where it is not

4.  **No silent deletion**: RMO requires explicit user action

## Formal Properties

These are target properties. Each is stated with its status in the claims ledger
(`PROOF-NEEDS.adoc`), and none is a theorem about the Rust as a whole.

### Property 1: Reversibility (RMR), two-sided and relative to an equality

A reversibility claim is an equality *up to* an equivalence $\sim$ on states, and
says nothing about any finer equality. Two are in use:

- $\sim_{tree}$: same relative paths and same bytes for the regular files under
  the working tree. Permissions, owner/group, mtime, symlink identity, inode
  identity and empty directories are not compared. This is what
  `property_tests::execute_then_undo_is_identity` checks. Undo also restores
  permission bits, but no test compares them; owner, group, mtime and symlink
  identity are recorded and not restored ([RMR §2](rmr-primitive.md)).
- $\sim_{model}$: propositional equality on the abstract state of
  `verification/proofs/agda/TransactionAtomicity.agda`, with no mechanical link
  to the filesystem.

Executing an operation produces a state *and* a record. Write $S$ for states
(the working tree, or the model state) and $R$ for records: the operation's
`OperationMetadata` log entry together with the content-store blobs it names.
An operation $F$ with precondition domain $D_F \subseteq S$ is a map
$F : D_F \to S \times R$, with components $F(s) = (F_S(s),\, r_F(s))$. Its
recorded inverse consumes both, $F^{-1} : S \times R \to S$, because undo reads
the record (`OperationExecutor::undo` looks the operation up in the log and
retrieves the stored content by its hash):

$$\text{RMR-L:}\quad \forall s \in D_F.\; F^{-1}(F_S(s),\, r_F(s)) \sim s \qquad\qquad \text{RMR-R:}\quad \forall s \in D_F.\; F_S\big(F^{-1}(F_S(s),\, r_F(s))\big) \sim F_S(s)$$

Together they say that, up to $\sim$, the inverse given the record undoes $F$
on $D_F$, and $F$ undoes the inverse on the image of $D_F$. The post-state
alone does not determine what undo restores: deleting a file from two trees
that differ only in that file's content gives $\sim$-equal post-states with
different records, so no $F^{-1}$ of the post-state alone could satisfy RMR-L
for both. RMR-R compares states only; re-executing $F$ writes a new record (a
new id and timestamp), so it is not stated for records.

RMR-R is over the image only: $\text{Copy}^{-1}$ is a delete of the
destination, and copying again does not restore arbitrary destination content.
RMR-R follows from RMR-L when $F_S$ respects $\sim$ and the restored state is
again in $D_F$; neither is stated or tested.

Both equations assume the record was saved. `execute_*` changes the filesystem
and then appends the record, so a stop between the two leaves a post-state with
no record for undo to consume (T-9, OPEN).

| Part | Status |
|---|---|
| RMR-L, one step, $\sim_{tree}$ | **TESTED** for Delete, Modify, Move, Copy, Create; Chmod is not in the test; Chown undo is unimplemented; the failure path is ASSUMED (T-6) |
| RMR-L, a transaction undone in reverse order, $\sim_{model}$ | **PROVEN** in the model (T-1), given T-6 |
| RMR-R | **OPEN** — no ledger row; neither proved nor tested |
| "Every operation is reversible" | **OPEN** |

### Property 2: Obliteration (RMO) — target, not established

The target: after $\text{RMO}(d)$, no algorithm recovers $d$ from the system
state. That an attacker cannot recover obliterated content is **OPEN** (ledger
E-1; only its model-level shadow E-1m is proved). Hash one-wayness and physical
erasure are **ASSUMED**, external to this repository (X-1). See
[RMO Primitive Specification](rmo-primitive.md).

### Property 3: Accountability Preservation — store-backed path only

The target, stated only for an RMO run in a directory with an initialized
JanusKey store, on the path where each record save succeeds:

$$\forall\, \text{RMO}(d) \text{ store-backed, record saved}:\; \text{log}(\text{timestamp}, \text{user}, \text{hash}(d))$$

The record (`ObliterationRecord`) holds the timestamp, user and content hash,
never the content, and it is not tamper-evident (see RMO above). Outside that
path the property is not claimed:

| Part | Status |
|---|---|
| Store-backed `jk obliterate` of a path with a working file and unshared blobs, every save succeeding: one record for the file and one per shredded blob, persisted | **TESTED** — `obliteration_cas_test::obliterate_scrubs_blobs_log_and_undo` reopens the obliteration log and counts one record for the file plus one per blob (the ledger row "Obliterating a path shreds its unshared blobs and log entries, and undo then has nothing") |
| `jk obliterate` with no initialized store | **OPEN**, false today, no ledger row — `cmd_obliterate` calls `obliterate_file` alone, which overwrites and removes the file and writes no record |
| Store-backed, a record save fails | **OPEN**, no ledger row — `record_proof` and `ObliterationManager::obliterate` remove the content first and save the record after, so a failed save leaves the erasure with no durable record unless a later save in the same run writes the pending one |
| Store-backed, working file absent and every blob shared with another path | **OPEN**, no ledger row, by inspection — `obliterate_path` purges the path's log entries and writes no obliteration record |

## Implementation in JanusKey

JanusKey implements RMR at the strength stated under Property 1, and RMO as
`jk obliterate` (`crates/januskey-cli/src/obliteration.rs`). The "Planned RMO
Implementation" sketch below predates that module; its step 4 ("non-existence
proof") was never implemented and is not claimed (E-1, X-1).

### Current RMR Implementation

``` rust
// Every operation stores complete reversal information
pub struct OperationMetadata {
    pub id: String,                           // Unique identifier
    pub op_type: OperationType,               // What operation
    pub timestamp: DateTime<Utc>,             // When
    pub user: String,                         // Who
    pub path: PathBuf,                        // What file
    pub content_hash: Option<ContentHash>,    // Original content
    pub original_metadata: Option<FileMetadata>, // Original metadata
    // ... sufficient for complete reversal
}
```

### Planned RMO Implementation

``` rust
// Future RMO primitive
pub struct ObliterationRecord {
    pub timestamp: DateTime<Utc>,
    pub user: String,
    pub content_hash: ContentHash,  // Hash of obliterated content
    pub obliteration_proof: ObliterationProof,
}

// Secure deletion with overwrite
pub fn obliterate(content_hash: &ContentHash) -> Result<ObliterationProof> {
    // 1. Locate content in store
    // 2. Overwrite with cryptographic randomness
    // 3. Remove from content index
    // 4. Generate non-existence proof
    // 5. Log obliteration record (without content)
}
```

## Requirements Mapping (not a compliance claim)

This table says which primitive is *relevant* to each requirement. It is not a
statement that any of them is satisfied: compliance is a determination for the
controller, on the facts of a deployment, and no assessment of JanusKey has been
performed.

| Regulation        | Requirement           | Relevant primitive | What that does **not** establish |
|-------------------|-----------------------|--------------------|----------------------------------|
| GDPR Art. 17      | Right to erasure      | RMO (`jk obliterate`), for files the tool can reach | That the data is gone from backups, replicas, journals or the media |
| GDPR Art. 5(1)(e) | Storage limitation    | RMR history + `jk gc` pruning | Any retention policy; none is implemented or enforced |
| GDPR Art. 30      | Records of processing | RMR operation log, `obliterations.json` | That the record is complete or that the ledger is trustworthy on its own (the obliteration ledger is unkeyed) |
| SOX Section 802   | Record retention      | RMR history, while the store is intact | Immutability against an attacker with write access |
| HIPAA             | Audit controls        | Attestation chain (HMAC-SHA256) over covered operations | That every relevant action is covered; obliteration records are not yet chained into the keyed audit log |
| CCPA              | Right to delete       | RMO, as above | Any of the above |

## Security Considerations

### Threat Model

| Threat                         | RMR Protection           | RMO Protection                 |
|--------------------------------|--------------------------|--------------------------------|
| Accidental deletion            | Full reversal capability | N/A (RMO is intentional)       |
| Malicious modification         | Detect via audit trail   | N/A                            |
| Data breach (compromised data) | N/A                      | Obliterate compromised content |
| Compliance audit failure       | Complete audit trail     | Obliteration records           |

### Attack Surface

The MAA framework’s security relies on:

1.  **Content store integrity** (SHA256 verification)

2.  **Metadata store integrity** (append-only log)

3.  **RMO implementation correctness** (secure overwrite)

## Theoretical Foundations

MAA draws from:

### Bennett’s Reversible Computing (1973)

Charles Bennett proved that all computation can be made reversible without significant overhead. JanusKey applies this to file operations.

### Landauer’s Principle (1961)

Rolf Landauer established that erasing information has a minimum thermodynamic cost. RMO acknowledges that true erasure is an irreversible physical process.

### What is proved, and what is not

Traditional logging provides *evidence* of state changes. JanusKey provides
smaller, differently-shaped things, and the difference matters:

- **RMR:** three proved lemmas over a transaction *model* — a rollback restores
  the state it began in given each step's undo restores its execute; the
  lifecycle preserves the log invariant; at most one transaction is active
  (`verification/proofs/agda/TransactionAtomicity.agda`, ledger rows T-1..T-3).
  The claim "every operation is reversible" is **OPEN**, as is the mechanical
  correspondence between that model and the Rust.
- **RMO:** nothing is proved. Overwriting is best effort; the record is an
  unkeyed ledger that a writer can alter without detection, kept only when a
  store exists, and it is not a certificate of absence. There is no
  mathematical proof of non-existence, and there cannot be one from inside this
  process: the tool cannot observe the media, the journal, or the backups.

The claims ledger in `PROOF-NEEDS.adoc` is the authority for both, and
`dashboard-check` fails the build when a dashboard's counts disagree with it.

## Further Reading

- [RMR Primitive Specification](rmr-primitive.md)

- [RMO Primitive Specification](rmo-primitive.md)

- [Formal Proofs](../formal-proofs/index.md)

- [Maximal Principle Reduction](mpr.md)
