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

MAA introduces two formally verified primitives that satisfy both requirements:

### RMR (Reversible Transaction)

> A primitive for state transitions that records, before each change, what is needed to undo it — so that undo is a metadata lookup rather than a restore from backup. Whether every operation is *in fact* reversible is an open obligation, not a guarantee; see the claims ledger.

RMR provides:

- Complete state capture before changes

- Guaranteed reversal capability

- Full audit trail of all operations

- Accountability without data loss risk

See: [RMR Primitive Specification](rmr-primitive.md)

### RMO (Obliterative Wipe)

> A primitive for data erasure: best-effort overwrite, removal, store scrub, and a tamper-evident record of what was destroyed.

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

3.  **Audit of RMO**: The *fact* that RMO was invoked is logged (but not the erased content)

4.  **No silent deletion**: RMO requires explicit user action

## Formal Properties

### Property 1: Complete Reversibility (RMR)

For any sequence of operations $O_1, O_2, \ldots, O_n$:

$$O_n^{-1}(O_{n-1}^{-1}(\ldots O_1^{-1}(O_1(O_2(\ldots O_n(S)))))) = S$$

The system can always return to any previous state.

### Property 2: Formal Obliteration (RMO)

After $\text{RMO}(d)$ for data $d$:

$$\nexists \text{ algorithm } A: A(\text{system\_state}) \to d$$

No algorithm can recover the obliterated data from system state.

### Property 3: Accountability Preservation

$$\forall \text{RMO}(d): \text{log}(\text{timestamp}, \text{user}, \text{hash}(d))$$

The fact of deletion is logged, maintaining accountability without content preservation.

## Implementation in JanusKey

JanusKey currently implements RMR fully. RMO support is planned.

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
- **RMO:** nothing is proved. Overwriting is best effort; the record is
  tamper-evident, not a certificate of absence. There is no mathematical proof
  of non-existence, and there cannot be one from inside this process: the tool
  cannot observe the media, the journal, or the backups.

The claims ledger in `PROOF-NEEDS.adoc` is the authority for both, and
`dashboard-check` fails the build when a dashboard's counts disagree with it.

## Further Reading

- [RMR Primitive Specification](rmr-primitive.md)

- [RMO Primitive Specification](rmo-primitive.md)

- [Formal Proofs](../formal-proofs/index.md)

- [Maximal Principle Reduction](mpr.md)
