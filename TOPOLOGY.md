<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->
<!-- TOPOLOGY.md — Project architecture map and completion dashboard -->
<!-- Last updated: 2026-02-19 -->

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

```
COMPONENT                          STATUS              NOTES
─────────────────────────────────  ──────────────────  ─────────────────────────────────
CORE ENGINE (RUST)
  Operation Layer                   ██████████ 100%    Delete/Modify/Move stable
  Inverse Metadata Gen              ██████████ 100%    Perfect inversion verified
  Transaction Manager               ██████████ 100%    Begin/Commit/Rollback active
  Content-Addressed Storage         ██████████ 100%    SHA256 deduplication verified

INTERFACES & RESEARCH
  CLI Interface (jk)                ██████████ 100%    Full command set verified
  MPR Methodology                   ██████████ 100%    Security by construction proven
  Testing Report (SCM)              ██████████ 100%    Audit trail validated

REPO INFRASTRUCTURE
  Justfile Automation               ██████████ 100%    Standard build/lint tasks
  .machine_readable/                ██████████ 100%    STATE tracking active
  0-AI-MANIFEST.a2ml                ██████████ 100%    AI entry point verified

─────────────────────────────────────────────────────────────────────────────
OVERALL:                            ██████████ 100%    v1.0 Production Ready
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
