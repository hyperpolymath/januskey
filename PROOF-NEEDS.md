<!--
SPDX-License-Identifier: CC-BY-SA-4.0
Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
-->
# PROOF-NEEDS.md — januskey

## Current State

_Re-verified 2026-06-29 (idris2 0.8.0)._

- **src/abi/*.idr**: PRESENT but **placeholder** — `src/abi/Proofs.idr` typechecks, but its
  theorems are trivial/tautological (e.g. `memoryDefeatsGPU : So (65536 >= 65536)`,
  `timeCostMonotonic : So (a >= b) -> So (a >= b)` returns its own hypothesis). NOT the
  security proofs listed below.
- **generated/idrisiser/idris2/Januskey/Verified/*.idr**: **DO NOT typecheck** —
  `idris2 --check` fails ("Expected a capitalised identifier, got: key") because the
  generated module names are lowercase. Despite the `Verified/` name, nothing there is
  currently verified.
- **Dangerous patterns**: **265** `unwrap()` calls across the Rust codebase.
- **LOC**: ~12,200 (Rust)
- **ABI layer**: present (`src/abi/{Types,Foreign,Layout,Proofs}.idr`) but carries no
  load-bearing security proof yet — the real obligations below are still open.

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
