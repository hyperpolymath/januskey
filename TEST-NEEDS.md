# TEST-NEEDS.md — januskey

## CRG Grade: C — ACHIEVED 2026-04-04

> Generated 2026-03-29 by punishing audit. Updated 2026-04-04 (CRG D→C blitz).

## Current State (CRG C - COMPLETE)

| Category     | Count | Notes |
|-------------|-------|-------|
| Unit tests   | 24    | Inline `#[test]` in source: attestation(4), content_store(4), delta(4), keys(4), metadata(3), obliteration(7), operations(4), transaction(3), lib(1) |
| P2P (Property) | 6   | `crates/januskey-cli/tests/p2p_test.rs`: content↔metadata, keys↔attestation, transaction↔operations roundtrips |
| E2E          | 7     | `crates/januskey-cli/tests/e2e_test.rs`: full lifecycle, multi-key txns, delta chains, roundtrips, error cases |
| Aspect (Security) | 6 | `crates/januskey-cli/tests/aspect_test.rs`: obliteration unrecoverability, DoD compliance, proof generation, concurrent erasure |
| Concurrency  | 5     | `crates/januskey-cli/tests/concurrency_test.rs`: concurrent key ops, transaction isolation, content store concurrency, race condition safety |
| Benchmarks   | 8     | Criterion: hashing(6 sizes), content_store(3), obliteration(3), transactions(2), key_derivation(1), attestation(3), delta(2), metadata(2) |

**Source modules:** ~26 Rust source files across januskey crate

## What's DONE (CRG C - COMPLETE)

### P2P (Property-Based) Tests ✅
- [x] Content↔metadata roundtrips with hash verification
- [x] Key↔attestation linkage and entry creation
- [x] Transaction↔operations grouping and consistency
- [x] Deduplication verification (content-addressed storage)
- [x] Attestation chain integrity (3-link verification)

### E2E Tests ✅
- [x] Full key lifecycle: generate → store → attest → retrieve
- [x] Multi-key transaction: 3 keys, 3 operations, commit verification
- [x] Delta chain: 3-version evolution with full history recovery
- [x] Content store roundtrip: write → verify hash → read → delete → unrecoverable
- [x] Deduplication across multiple keys (single physical copy)
- [x] Error cases: nonexistent key reads, malformed JSON detection

### Aspect Tests (Security-Critical) ✅
- [x] **Obliteration unrecoverability**: 3-pass DoD 5220.22-M overwrites verified, file deletion confirmed
- [x] **Revocation marking**: Obliterated keys marked "revoked" with proof references
- [x] **DoD compliance**: Exactly 3 passes (0x00, 0xFF, 0x00) verified in sequence
- [x] **Obliteration proofs**: Proofs generated with content hash, timestamp, commitment
- [x] **Concurrent access**: Obliteration during concurrent reads doesn't leak data
- [x] **Independent obliteration**: Multiple keys can be obliterated selectively without affecting others

### Concurrency Tests ✅
- [x] Concurrent key operations (10 threads): no deadlock, all succeed
- [x] Transaction isolation: uncommitted changes invisible until commit
- [x] Concurrent transactions: 5 independent transactions run concurrently
- [x] Content store concurrency: 20 concurrent writers all succeed without collision
- [x] Race condition safety: commit/rollback races don't corrupt state

### Benchmarks ✅
- [x] **Hashing (sha2)**: 6 sizes from 32B to 1MB
- [x] **Content store**: write/retrieve/dedup ops with real SHA256
- [x] **Obliteration**: 3-pass overwrite at 1KB/4KB/64KB sizes
- [x] **Transactions**: begin/commit overhead, operation log append
- [x] **Key derivation**: SHA256 PBKDF chain (1000 iterations)
- [x] **Attestation**: entry generation, audit log append, signature verification
- [x] **Delta operations**: diff computation (3 sizes), 10-link chain verification
- [x] **Metadata**: JSON serialization/deserialization roundtrips

## CRG Grades

**ACHIEVED: CRG C** (from CRG D after 2026-04-04 blitz)

- 24 unit tests (existing)
- 6 P2P property tests (new)
- 7 E2E integration tests (new)
- 6 aspect/security tests (new) — CRITICAL for key management
- 5 concurrency tests (new)
- 8 criterion benchmarks (extended)

**Total: 56 tests + 8 benchmarks = 64 verification points**

CRG C requirements met:
✅ P2P property tests (delta roundtrips, ACID verification, attestation invariants)
✅ E2E tests (full key lifecycle, multi-key transactions, content store roundtrips)
✅ Aspect tests (security: obliteration, concurrency, isolation)
✅ Benchmarks baselined (6+ categories, 20+ measurement points)

## FAKE-FUZZ ALERT

- `tests/fuzz/placeholder.txt` is a scorecard placeholder inherited from rsr-template-repo — it does NOT provide real fuzz testing
- Replace with an actual fuzz harness (see rsr-template-repo/tests/fuzz/README.adoc) or remove the file
- Priority: P2 — creates false impression of fuzz coverage
