<!--
SPDX-License-Identifier: CC-BY-SA-4.0
Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
-->
<!-- TOPOLOGY.md — Project architecture map and completion dashboard -->
<!-- Last updated: 2026-07-02 (completion dashboard reconciled to STATE.a2ml / READINESS.md) -->

# JanusKey — Project Topology

## System Architecture

```
                        ┌─────────────────────────────────────────┐
                        │              OPERATOR / CLI             │
                        │        (jk delete, jk modify, jk undo)  │
                        └───────────────────┬─────────────────────┘
                                            │
                                            ▼
                        ┌─────────────────────────────────────────┐
                        │           JANUSKEY CORE (RUST)          │
                        │    (Operation Layer / Inverse Meta)     │
                        └──────────┬───────────────────┬──────────┘
                                   │                   │
                                   ▼                   ▼
                        ┌───────────────────────┐  ┌────────────────────────────────┐
                        │ TRANSACTION MANAGER   │  │ REVERSIBILITY ENGINE           │
                        │ - begin / commit      │  │ - Maximal Principle Reduction   │
                        │ - rollback logic      │  │ - Inverse Metadata Gen         │
                        └──────────┬────────────┘  └──────────┬─────────────────────┘
                                   │                          │
                                   └────────────┬─────────────┘
                                                ▼
                        ┌─────────────────────────────────────────┐
                        │             DATA LAYER                  │
                        │  ┌───────────┐  ┌───────────────────┐  │
                        │  │ Metadata  │  │ Content-Addressed │  │
                        │  │ Store     │  │ Storage (SHA256)  │  │
                        │  └───────────┘  └───────────────────┘  │
                        └───────────────────┬─────────────────────┘
                                            │
                                            ▼
                        ┌─────────────────────────────────────────┐
                        │           TARGET FILESYSTEM             │
                        │      (Provably Reversible State)        │
                        └─────────────────────────────────────────┘

                        ┌─────────────────────────────────────────┐
                        │          REPO INFRASTRUCTURE            │
                        │  Justfile / Cargo   .machine_readable/  │
                        │  MAAF Integration   0-AI-MANIFEST.a2ml  │
                        └─────────────────────────────────────────┘
```

## Completion Dashboard

> **Source of truth:** `.machine_readable/6a2/STATE.a2ml` (completion 60%,
> CRG grade **D**) and `READINESS.md` (Grade **D — Alpha, Unstable**). This
> dashboard is a human-readable summary of those files; if they disagree,
> they win. Percentages below are qualitative, not measured coverage.

```
COMPONENT                          STATUS              NOTES
─────────────────────────────────  ──────────────────  ─────────────────────────────────
CORE ENGINE (RUST)
  Operation Layer                   ████████░░  ~85%   Delete/Modify/Move implemented; unit-tested
  Inverse Metadata Gen              ████████░░  ~80%   execute∘undo roundtrip property-tested (proptest)
  Transaction Manager               ████████░░  ~80%   Begin/Commit/Rollback tested via P2P suite
  Content-Addressed Storage         ████████░░  ~85%   Real SHA256 content store + dedup

SECURITY (honest)
  Attestation / audit chain         ███░░░░░░░  ~30%   Homerolled SHA256(key||data||prev) MAC +
                                                        zero-key fallback — flagged by threat model
                                                        (STATE.a2ml homerolled-hmac); needs real HMAC
  Asymmetric crypto                 █░░░░░░░░░  ~10%   Ed25519/X25519 are enum labels, not implemented
  Secure obliteration               ████░░░░░░  ~40%   Best-effort; NOT guaranteed on SSD/CoW (threat-
                                                        model dependent — see obliteration caveat)

INTERFACES & RESEARCH
  CLI Interface (jk)                ████████░░  ~85%   Full command set; no user testing yet
  MPR Methodology                   ████░░░░░░  ~40%   Design documented; FORMAL PROOFS PENDING
                                                        (30 Idris2 proofs unchecked in CI; not linked
                                                        to the Rust)
  Testing (READINESS matrix)        ██████░░░░  ~60%   67 tests + 5 benches; missing fuzz, mutation,
                                                        chaos, compatibility (Grade D)

REPO INFRASTRUCTURE
  Justfile Automation               █████████░  ~90%   Build/lint/test recipes present
  .machine_readable/                █████████░  ~90%   STATE tracking active
  0-AI-MANIFEST.a2ml                █████████░  ~90%   AI entry point present

─────────────────────────────────────────────────────────────────────────────
OVERALL:                            ██████░░░░  ~60%   Grade D — Alpha, Unstable (not v1.0)
```

## Key Dependencies

```
jk command ──────► Inverse Meta ──────► Transaction Mgr ──────► Commit
     │                 │                   │                    │
     ▼                 ▼                   ▼                    ▼
Source File ─────► SHA256 Store ─────► Metadata Log ───────► Rollback
```

## Update Protocol

This file is maintained by both humans and AI agents. When updating:

1. **After completing a component**: Change its bar and percentage
2. **After adding a component**: Add a new row in the appropriate section
3. **After architectural changes**: Update the ASCII diagram
4. **Date**: Update the `Last updated` comment at the top of this file

Progress bars use: `█` (filled) and `░` (empty), 10 characters wide.
Percentages: 0%, 10%, 20%, ... 100% (in 10% increments).
