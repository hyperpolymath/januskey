# TEST-NEEDS.md — januskey

> Generated 2026-03-29 by punishing audit.

## Current State

| Category     | Count | Notes |
|-------------|-------|-------|
| Unit tests   | ~34   | All inline `#[test]` in source: attestation(4), content_store(4), delta(4), keys(4), metadata(3), obliteration(7), operations(4), transaction(3), lib(1) |
| Integration  | 0     | tests/ dir exists but contains only fuzz targets |
| E2E          | 0     | None |
| Benchmarks   | 0     | None |

**Source modules:** ~26 Rust source files across januskey crate (attestation, content_store, delta, keys, metadata, obliteration, operations, transaction).

## What's Missing

### P2P (Property-Based) Tests
- [ ] Key generation: property tests for uniqueness, format compliance
- [ ] Delta computation: property tests for diff/patch roundtrip
- [ ] Transaction: ACID property verification
- [ ] Attestation: signature verification invariants

### E2E Tests
- [ ] Full key lifecycle: generate -> store -> attest -> retrieve -> obliterate
- [ ] Multi-key transaction: batch operations with rollback
- [ ] Content store: write -> verify -> read -> delete with integrity checks
- [ ] Delta chain: create -> modify -> modify -> verify full history

### Aspect Tests
- **Security:** Obliteration completeness (7 inline tests exist but no verification that obliterated data is truly unrecoverable), key material in memory after drop, side-channel resistance — CRITICAL for a key management tool
- **Performance:** No benchmarks for key generation speed, content store throughput, delta computation cost
- **Concurrency:** No tests for concurrent key operations, transaction isolation, content store contention
- **Error handling:** No tests for corrupted key store, interrupted transactions, invalid attestation chains

### Build & Execution
- [ ] `cargo test` runner
- [ ] Fuzz target execution (`cargo fuzz`)

### Benchmarks Needed
- [ ] Key generation latency (per algorithm)
- [ ] Content store read/write throughput
- [ ] Delta computation time vs data size
- [ ] Obliteration time and verification
- [ ] Transaction commit/rollback overhead

### Self-Tests
- [ ] Key store integrity self-check
- [ ] Attestation chain validation
- [ ] Content store consistency verification

## Priority

**HIGH.** A key management system with zero integration tests and zero benchmarks. 34 inline unit tests for 26 modules is decent unit coverage, but no integration or E2E means the pieces are never tested together. For a security-critical tool, this is insufficient. Fuzz targets exist but need to actually run.

## FAKE-FUZZ ALERT

- `tests/fuzz/placeholder.txt` is a scorecard placeholder inherited from rsr-template-repo — it does NOT provide real fuzz testing
- Replace with an actual fuzz harness (see rsr-template-repo/tests/fuzz/README.adoc) or remove the file
- Priority: P2 — creates false impression of fuzz coverage
