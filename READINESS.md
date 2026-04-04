<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->
# Component Readiness Assessment — januskey

**Assessed:** 2026-04-03
**Assessor:** Claude (automated) + Jonathan (review)
**Taxonomy:** standards/testing-and-benchmarking/TESTING-TAXONOMY.adoc v1.0

**Current Grade:** D

## CRG Grade: D (Alpha — Unstable)

**Justification:** Tests exist and pass (67 total), proofs exist (30 Idris2, unchecked in CI), benchmarks exist (5 Criterion groups). But: no fuzz testing, no mutation testing, 225 unwrap() calls, E2E mostly skips, benchmarks measured fake crypto until this session. RSR compliance present. Deep annotation incomplete (TOPOLOGY.md exists but per-directory orientation missing → blocks C).

**Promotion path D→C:** Fix unwrap() calls, wire Idris2 proof check in CI, complete per-directory annotation, add real E2E with built binary, dogfood in at least one real workflow.

## Test Category Matrix

| # | Category | Status | Count | Recipe | Notes |
|---|----------|--------|-------|--------|-------|
| 1 | Unit | ✓ | 36 | `just test` | reversible-core + januskey-cli |
| 2 | P2P | ✓ | 8 | `just test-p2p` | Component interaction across crates |
| 3 | E2E | PARTIAL | 1 script | `just test-e2e` | Mostly skips without pre-built binary |
| 4 | Build | ✓ | CI | `just build` | cargo build --workspace --release |
| 5 | Execution | N/A | — | — | Not an interpreter/VM |
| 6 | Reflexive | ✓ | 1 | `just doctor` | Tool + path checks |
| 7 | Lifecycle | PARTIAL | via P2P | — | Transaction begin/commit/rollback tested; no resource cleanup tests |
| 8 | Smoke | ✓ | 1 | `just smoke` | Build + version + help |
| 9 | Property-based | ✓ | 9 | `just test-property` | proptest: roundtrip, obliteration, key derivation, content store |
| 10 | Mutation | MISSING | 0 | — | cargo-mutants not yet configured |
| 11 | Fuzz | MISSING | 0 | — | Fake placeholder removed. Real fuzz not yet implemented |
| 12 | Contract | ✓ | 3 files | `just test-contracts` | Mustfile + Trustfile + Dustfile validated |
| 13 | Regression | ✓ | 5 | `just test-regressions` | unwrap safety + error handling |
| 14 | Chaos | MISSING | 0 | — | No resilience tests yet |
| 15 | Compatibility | MISSING | 0 | — | No version migration tests |
| 16 | Proof regression | ✓ | 30 proofs | `just test-proofs` | Idris2 --check (requires idris2 binary) |

**Total passing:** 67 tests + 5 benchmark groups + 30 Idris2 proofs
**Total missing:** Fuzz, mutation, chaos, compatibility

## Aspect Matrix

| # | Aspect | Status | Evidence |
|---|--------|--------|----------|
| 1 | Dependability | PARTIAL | Transaction rollback tested. No crash recovery tests. |
| 2 | Security | PARTIAL | Obliteration tests, forbidden pattern checks. No side-channel or memory-after-drop. |
| 3 | Usability | PARTIAL | CLI exists. No user testing. |
| 4 | Interoperability | PARTIAL | Zig FFI tests (15). No cross-language integration test. |
| 5 | Safety | ✓ | 0 believe_me, 0 assert_total. #![forbid(unsafe_code)]. panic-attack assail passes. |
| 6 | Performance | PARTIAL | 5 Criterion benchmark groups. Baseline not yet recorded in VeriSimDB. |
| 7 | Functionality | PARTIAL | Core ops tested. Edge cases via proptest. Feature matrix vs README not audited. |
| 8 | Versability | MISSING | No version migration or semver compliance tests. |
| 9 | Accessibility | N/A | CLI tool. |
| 10 | Maintainability | PARTIAL | TOPOLOGY.md exists. Per-directory annotation incomplete. |
| 11 | Privacy | N/A | Local tool, no network, no telemetry. |
| 12 | Observability | MISSING | No structured logging. No tracing. |
| 13 | Reproducibility | PARTIAL | Cargo.lock committed. Nix not configured. |
| 14 | Portability | PARTIAL | Builds on Linux. macOS/Windows untested. |

## Benchmark Classification

| Benchmark Group | Baseline (this run) | Classification |
|-----------------|--------------------|----|
| Hashing | TBD — run `just bench` | Not yet baselined |
| Content Store | TBD | Not yet baselined |
| Obliteration | TBD | Not yet baselined |
| Transactions | TBD | Not yet baselined |
| Key Derivation | TBD | Not yet baselined |

Benchmarks now use real SHA256 (was DefaultHasher). First baseline needs recording.

## Known Debt

- 225 unwrap() calls (tracked in PROOF-NEEDS.md)
- E2E test needs pre-built binary to exercise full lifecycle
- Idris2 proofs not checked in CI (requires idris2 in CI image)
- No code coverage measurement
- No mutation testing
- Benchmarks not baselined in VeriSimDB

## Recipes

```
just test-all      # Run everything
just test          # Unit tests only
just test-p2p      # Component interaction
just test-e2e      # End-to-end (shell)
just test-aspect   # Cross-cutting checks
just test-property # Property-based (proptest)
just test-regressions # Regression suite
just test-contracts # Contractile validation
just test-proofs   # Idris2 proof regression
just bench         # Criterion benchmarks
just smoke         # Quick sanity check
just doctor        # Self-diagnostic
```
