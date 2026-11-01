<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
<!-- Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk> -->

# RMO: Obliterative Wipe Primitive

> **Status: engineering primitive, not a compliance mechanism and not a theorem.**
> RMO describes what `jk obliterate` does. It overwrites a file's bytes and
> removes it, and — when a JanusKey store exists — shreds the unshared
> content-store blobs and purges the operation-log entries for that path.
> It does **not** establish that the data is unrecoverable, and it is not a
> GDPR Article 17 determination. This page previously claimed both; the
> correction is recorded in `PROOF-NEEDS.adoc` (rows C-3, X-1) and in the
> audit at `docs/estate/2026-11-01-handoff-and-consistency-audit.md` (D1).

## Definition

**RMO (Obliterative Wipe)** is the deliberate, documented exception to
JanusKey's reversibility. Everything else in the tool stores the information
needed to invert itself; `jk obliterate` destroys that information instead.

It is best-effort by construction:

- Overwrite-in-place cannot reach physical media through wear levelling
  (SSD), copy-on-write (ZFS, Btrfs), journaling, or a hypervisor's snapshot.
- The threat model for overwriting assumes rotating magnetic media.
- On modern media, physical destruction of the device is the only
  guaranteed obliteration.

The same caveat appears in `README.adoc` §"The secure obliteration exception"
and `GLOSSARY.adoc` §"Secure obliteration". If this page and those disagree,
they win.

## What the operation actually does

1. **Overwrite.** Three passes over the file's current length — `0x00`, then
   `0xFF`, then random bytes — with `sync_all()` after each pass
   (`crates/januskey-cli/src/obliteration.rs`, `secure_overwrite`, `OVERWRITE_PASSES = 3`).
   The pattern set has the shape of DoD 5220.22-M. That is a description of
   the passes, **not** a claim of conformance to that standard.
2. **Unlink.** `fs::remove_file` on the path.
3. **Scrub the store** (only when a `.januskey/` store exists,
   `obliterate_path`): shred every content-store blob referenced by an
   operation-log entry whose primary or secondary path is this one — **except**
   a blob that a surviving entry for another path still references, which is
   kept so the other path's history stays undoable.
4. **Purge the log entries.** `MetadataStore::purge_path`, so `jk undo` and
   `jk history` no longer mention the path. Blobs are shredded *before* the
   log is purged: if a shred fails, the entries survive and the command can be
   re-run.
5. **Record.** Each shred appends a record to `obliterations.json`
   (`OBLITERATION_LOG_FILE`), including the working file's proof.

## The obliteration record

```rust
pub struct ObliterationProof {
    pub id: String,
    pub content_hash: ContentHash,   // what was destroyed
    pub timestamp: DateTime<Utc>,
    pub user: String,
    pub nonce: String,
    pub commitment: String,          // H(content_hash ‖ nonce ‖ timestamp)
    pub overwrite_passes: usize,
    pub storage_cleared: bool,       // set by the tool; not independently verified
}
```

What the commitment is: a SHA-256 binding of *the claim being made* — which
content hash, at what time, with which nonce. It detects accidental changes
to the record. It does not resist a writer who edits the record, because the
commitment has no secret key. It is **not** a proof that the content
is gone, and `storage_cleared` is an assertion written by the same code path
that performed the overwrite, not an independent check.

What can be checked in-process today: `ObliterationProof::verify_commitment`
(recompute the hash from the record), `ObliterationManager::verify_proof(id)`,
and `verify_obliteration(path, original_hash)` (the path no longer holds that
content). There is **no** `jk verify-obliteration` and no
`jk obliteration-history` subcommand; the CLI surface is `jk obliterate <paths>…`
with the global `--dry-run` and `--yes` flags, and it asks for confirmation
when non-interactive without `--yes`.

## Status, in the claims-ledger vocabulary

| Statement | Status | Where |
|---|---|---|
| A transaction containing an obliteration is not all-reversible | **PROVEN** (model) | `Layout.idr::obliterateNotReversible` (ledger C-3) |
| Obliterating a path shreds its unshared blobs and log entries, and undo then has nothing | **TESTED** | `obliteration_cas_test::obliterate_scrubs_blobs_log_and_undo` |
| A blob another path still references is kept | **TESTED** | `obliteration_cas_test::obliterate_keeps_blob_shared_with_another_path` |
| `jk obliterate` scrubs the store end to end | **TESTED** | `obliteration_cas_test::cli_obliterate_scrubs_store_and_undo_has_nothing` |
| Data erasure is complete and irreversible (obliteration correctness) | **OPEN** | `PROOF-NEEDS.adoc` §What Needs Proving |
| Physical erasure; hash one-wayness | **ASSUMED** (external) | ledger X-1 |
| The obliteration record is chained into the keyed, tamper-evident audit log | **DESIGNED, not built** | `obliterations.json` is an unkeyed JSON ledger; chaining deferred (J2-3) |

Nothing in RMO is proved. The one PROVEN row above is a *model* statement
(C-3) about the operation log, not about bytes on a device.

## Erasure requests: what this does and does not establish

RMO is useful to a controller handling an erasure request. It is not a
compliance determination, and this repository makes none.

| Requirement (e.g. GDPR Art. 17) | What JanusKey supplies | What it does not supply |
|---|---|---|
| Erase without undue delay | Overwrite + unlink in one command; store scrub where a store exists | Any guarantee against media-level retention, journaling, snapshots, or backups |
| Evidence that erasure was carried out | A timestamped commitment record with the content hash, user, pass count — logged without the content | Proof of non-existence; independent verification |
| Records of the request | `obliterations.json`, plus the audit chain for other operations | Retention policy, legal-basis handling, controller/processor analysis |
| No retention of erased content | Proof records hold the hash, not the bytes | Erased copies outside JanusKey's reach (backups, replicas, caches) |

Whether a given deployment is compliant is a determination for the
controller, on the facts of that deployment — including media type, backups,
replicas, and retention policy. A local file tool cannot make it.

## Relationship with RMR

RMR (reversible transactions) and RMO (obliterative wipe) are complementary,
and the composition is intentional:

```
        +---------------+                    +---------------+
        |     RMR       |                    |     RMO       |
        | Reversibility |                    | Obliteration  |
        +-------+-------+                    +-------+-------+
                |        normal operation            |
                +------------------------------------>|
                |        can undo via RMR            |
                <------------------------------------+
                |        erasure request             |
                +------------------------------------>|
                |        after RMO: no undo path     |
```

The difference that matters for design, not marketing:

| Aspect | RMR | RMO |
|---|---|---|
| Purpose | Track and reverse changes | Destroy the record of one path |
| Reversibility | Reversible by metadata lookup | No inverse exists; undo has nothing to restore |
| Content storage | Preserved in the content store | Shredded, unless another path shares the blob |
| Audit | Operation log entries | Fact of deletion only, in an unkeyed ledger |
| Failure mode | Store loss makes undo impossible | Shared blobs and media-level retention leave traces |

## Security considerations

- **Media.** See the caveat at the top. A successful `jk obliterate` on an
  SSD does not mean the bytes are gone.
- **Shared blobs.** Deduplication means one blob can back several paths.
  Obliterating one path keeps a blob another path still references — correct
  for reversibility, but it means "obliterated" is not "gone from the store".
  `--dry-run` lists only the target paths. It does not report the shared
  blobs that will be kept or the history entries that will be purged.
- **The ledger is unkeyed.** `obliterations.json` is not HMAC-chained. An
  attacker with write access to the store can edit it. The keyed `AuditLog`
  covers other operations and requires an unlocked key; joining the two is
  J2-3, not done.
- **Interruption.** Blobs are shredded before the log is purged, so a
  partially completed obliteration can be re-run. It cannot be *undone*.

## Formal status

There is no theorem here, and there should not be a `∎` on this page until
one exists. The nearest statement in the kernel is G-1
(`verification/proofs/agda/Reversibility.agda`): reversibility is a `Loss`
grade over traces, with an obliterating step absorbing — a statement about the
grade algebra, not about storage media. `X-1` keeps physical erasure and hash
one-wayness explicitly outside the theorem set.

## Further reading

- [RMR: Reversible Transaction Primitive](rmr-primitive.md)
- [MAA Framework Overview](maa-framework.md)
- `../../../PROOF-NEEDS.adoc` — the claims ledger this page reconciles against
- `../../security/THREAT_MODEL.adoc` — limitations and the compliance mapping
