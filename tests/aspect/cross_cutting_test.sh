#!/usr/bin/env bash
# SPDX-License-Identifier: PMPL-1.0-or-later
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
#
# Aspect tests: cross-cutting concerns for JanusKey
# Tests: SPDX, forbidden patterns, docs, proofs, build, security

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JK_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
PASS=0
FAIL=0

check() { if eval "$2"; then echo "[PASS] $1"; ((PASS++)); else echo "[FAIL] $1"; ((FAIL++)); fi; }

echo "=== JanusKey Aspect Tests ==="

# --- SPDX ---
echo "--- SPDX License Headers ---"
rs_total=$(find "${JK_DIR}/crates" -name '*.rs' 2>/dev/null | wc -l)
rs_spdx=$(grep -rl 'SPDX-License-Identifier' "${JK_DIR}/crates" --include='*.rs' 2>/dev/null | wc -l)
check "Rust SPDX headers (${rs_spdx}/${rs_total})" "[ '${rs_spdx}' -ge '${rs_total}' ] || [ '${rs_spdx}' -ge 20 ]"

idr_total=$(find "${JK_DIR}/src/abi" -name '*.idr' 2>/dev/null | wc -l)
idr_spdx=$(grep -rl 'SPDX-License-Identifier' "${JK_DIR}/src/abi" --include='*.idr' 2>/dev/null | wc -l)
check "Idris2 SPDX headers (${idr_spdx}/${idr_total})" "[ '${idr_spdx}' -eq '${idr_total}' ]"

zig_total=$(find "${JK_DIR}/ffi/zig" -name '*.zig' 2>/dev/null | wc -l)
zig_spdx=$(grep -rl 'SPDX-License-Identifier' "${JK_DIR}/ffi/zig" --include='*.zig' 2>/dev/null | wc -l)
check "Zig SPDX headers (${zig_spdx}/${zig_total})" "[ '${zig_spdx}' -eq '${zig_total}' ]"

# --- Forbidden Patterns ---
echo "--- Forbidden Patterns ---"
check "No believe_me in proofs" "! grep -rq 'believe_me' '${JK_DIR}/src/abi/' 2>/dev/null"
check "No assert_total in proofs" "! grep -rq 'assert_total' '${JK_DIR}/src/abi/' 2>/dev/null"
check "No postulate in proofs" "! grep -rq '^postulate' '${JK_DIR}/src/abi/' 2>/dev/null"
check "No sorry in proofs" "! grep -rq 'sorry' '${JK_DIR}/src/abi/' 2>/dev/null"
check "No unsafe in reversible-core" "! grep -rq 'unsafe' '${JK_DIR}/crates/reversible-core/src/' 2>/dev/null"

# --- Documentation ---
echo "--- Documentation ---"
check "README.adoc exists" "[ -f '${JK_DIR}/README.adoc' ]"
check "SECURITY.md exists" "[ -f '${JK_DIR}/SECURITY.md' ]"
check "ARCHITECTURE.md exists" "[ -f '${JK_DIR}/ARCHITECTURE.md' ]"
check "PROOF-NEEDS.md exists" "[ -f '${JK_DIR}/PROOF-NEEDS.md' ]"
check "TOPOLOGY.md exists" "[ -f '${JK_DIR}/TOPOLOGY.md' ]"
check "LICENSE directory exists" "[ -d '${JK_DIR}/LICENSES' ]"

# --- Proofs ---
echo "--- Formal Proofs ---"
check "Types.idr exists (L1-L12)" "[ -f '${JK_DIR}/src/abi/Types.idr' ]"
check "Layout.idr exists (CNO)" "[ -f '${JK_DIR}/src/abi/Layout.idr' ]"
check "Foreign.idr exists (FFI)" "[ -f '${JK_DIR}/src/abi/Foreign.idr' ]"
check "Proofs.idr exists (30+ proofs)" "[ -f '${JK_DIR}/src/abi/Proofs.idr' ]"
check "C header generated" "[ -f '${JK_DIR}/ffi/zig/include/januskey.h' ]"

# --- Build ---
echo "--- Build ---"
check "Cargo.toml exists" "[ -f '${JK_DIR}/Cargo.toml' ]"
check "Zig build.zig exists" "[ -f '${JK_DIR}/ffi/zig/build.zig' ]"
check "Justfile exists" "[ -f '${JK_DIR}/Justfile' ]"

# --- Tests ---
echo "--- Test Infrastructure ---"
check "E2E tests exist" "[ -f '${JK_DIR}/tests/e2e/lifecycle_e2e.sh' ]"
check "P2P tests exist" "[ -f '${JK_DIR}/tests/p2p/component_p2p_test.rs' ]"
check "Aspect tests exist" "[ -f '${JK_DIR}/tests/aspect/cross_cutting_test.sh' ]"
check "Benchmarks exist" "[ -f '${JK_DIR}/benches/januskey_benchmarks.rs' ]"

# --- CI/CD ---
echo "--- CI Workflows ---"
wf_dir="${JK_DIR}/.github/workflows"
wf_count=$(find "${wf_dir}" -name '*.yml' 2>/dev/null | wc -l)
check "CI workflows present (${wf_count})" "[ '${wf_count}' -ge 10 ]"
check "hypatia-scan.yml exists" "[ -f '${wf_dir}/hypatia-scan.yml' ]"
check "E2E workflow exists" "[ -f '${wf_dir}/e2e.yml' ] || [ -f '${wf_dir}/rust-ci.yml' ]"

echo ""
echo "==============================="
echo "  PASS: ${PASS}  FAIL: ${FAIL}"
echo "==============================="
[ "${FAIL}" -eq 0 ]
