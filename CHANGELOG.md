<!--
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath)
-->

# Changelog

All notable changes to `januskey` will be documented in this file.

This file is generated from conventional commits by the
[`changelog-reusable.yml`](https://github.com/hyperpolymath/standards/blob/main/.github/workflows/changelog-reusable.yml)
workflow (`hyperpolymath/standards#206`). Adopt the workflow in this repo's CI to keep this file in sync automatically — see
[`templates/cliff.toml`](https://github.com/hyperpolymath/standards/blob/main/templates/cliff.toml)
for the canonical config.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project aims to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- feat(crg): add crg-grade and crg-badge justfile recipes
- feat(crg): add Current Grade badge anchor to READINESS.md
- feat: add idrisiser Idris2 proof wrappers for JanusKey cryptographic core
- feat: blitz — wire all tests, add property-based + regression, fix benchmarks, READINESS.md
- feat: add E2E, P2P, aspect tests + criterion benchmarks
- feat: add Zig FFI implementation + C header + integration tests
- feat: complete Idris2 ABI — Foreign.idr + Proofs.idr
- feat: add Idris2 ABI proofs — TypeLL Levels 1-12
- feat: add stapeln.toml container definition
- feat: deploy UX Manifesto infrastructure

### Fixed

- fix(ci): bump a2ml/k9-validate-action pins to canonical (#33)
- fix(ci): sync hypatia-scan.yml to canonical (#32)
- fix(ci): adopt canonical hypatia-scan.yml (#31)
- fix(ci): Phase-2 fleet submission must not fail the security gate (#30)
- fix(ci): hypatia-scan workdir (${{ env.HOME }} resolves empty) (#29)
- fix(januskey): sweep .expect("TODO: handle error") — 166 sites cleared
- fix: replace 60 unwrap() calls with expect() in security-critical modules
- fix: quote $$ and use printf in setup.sh
- fix: correct 'Provably Reversible' claim — proofs are pending, not done
- fix(scorecard): enforce granular permissions and add fuzzing placeholder

### Changed

- refactor: migrate 6SCM → 6A2 (.scm → .a2ml format)

### Documentation

- docs(security): draft MCP-exposure threat model (AI-authored, pending human sign-off)
- docs: add M2 estate audit report (2026-04-04)
- docs: substantive CRG C annotation (EXPLAINME.adoc)
- docs: add EXPLAINME.adoc — prove-it file backing README claims
- docs: add ARCHITECTURE.md — reversibility stack junction point
- docs: update SCM files with project information
- docs: add CONTRIBUTING.md
- docs: add checkpoint files for state tracking

### CI

- ci(rust): convert rust-ci.yml to thin wrapper (standards#174) (#39)
- ci: redistribute concurrency-cancel guard to read-only check workflows (#35)
- ci: bump actions/upload-artifact SHA to current v4 (#27)
- ci: SHA-pin hyperpolymath validate-actions in dogfood-gate
- ci: restore Dependabot security path + wire auto-merge

## Pre-history

Prior commits to this file's introduction are recorded in git history but not formally classified into Keep-a-Changelog sections. To backfill, run `git cliff -o CHANGELOG.md` locally using the canonical [`cliff.toml`](https://github.com/hyperpolymath/standards/blob/main/templates/cliff.toml) — this is one-shot mechanical work.

---

<!-- This file was seeded by the 2026-05-26 estate tech-debt audit follow-up (Row-2 Phase 3); see [`hyperpolymath/standards/docs/audits/2026-05-26-estate-documentation-debt.md`](https://github.com/hyperpolymath/standards/blob/main/docs/audits/2026-05-26-estate-documentation-debt.md). -->
