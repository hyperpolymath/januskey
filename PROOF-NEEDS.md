# PROOF-NEEDS.md — januskey

## Current State

- **src/abi/*.idr**: NO
- **Dangerous patterns**: 225 `unwrap()` calls across Rust codebase
- **LOC**: ~12,200 (Rust)
- **ABI layer**: Missing

## What Needs Proving

| Component | What | Why |
|-----------|------|-----|
| Obliteration correctness | Data erasure is complete and irreversible | Core feature: must guarantee data is unrecoverable |
| Key derivation | Key generation produces cryptographically sound keys | Weak keys break entire security model |
| Attestation chain | Chain of custody proofs are unforgeable | Tampered attestations undermine trust |
| Content store integrity | Content-addressed storage never returns wrong content | Hash collisions or bugs corrupt stored data |
| Transaction atomicity | Delta application is atomic and reversible | Partial transactions corrupt state |
| reversible-core | Undo/redo preserves data integrity | Core crate used by januskey-cli |

## Recommended Prover

**Idris2** — Create `src/abi/` with dependent type proofs for obliteration completeness, attestation chain integrity, and transaction atomicity. The 225 `unwrap()` calls should be systematically replaced with proven error handling.

## Priority

**HIGH** — JanusKey handles cryptographic key management, attestation chains, and data obliteration. Any correctness bug in obliteration or key derivation is a critical security vulnerability. Missing ABI layer entirely.
