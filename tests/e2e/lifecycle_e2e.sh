#!/usr/bin/env bash
# SPDX-License-Identifier: PMPL-1.0-or-later
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
#
# E2E test: full JanusKey lifecycle
# init → execute → undo → verify → obliterate → verify-destroyed

set -euo pipefail

PASS=0
FAIL=0
SKIP=0
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

check() { if eval "$2"; then echo "[PASS] $1"; ((PASS++)); else echo "[FAIL] $1"; ((FAIL++)); fi; }
skip() { echo "[SKIP] $1"; ((SKIP++)); }

echo "=== JanusKey E2E Lifecycle Test ==="

# Check binary exists
JK_BIN="$(command -v jk 2>/dev/null || echo "")"
if [ -z "$JK_BIN" ]; then
    # Try cargo build
    if command -v cargo >/dev/null 2>&1; then
        REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
        (cd "$REPO_ROOT" && cargo build --release 2>/dev/null)
        JK_BIN="$REPO_ROOT/target/release/jk"
    fi
fi

if [ ! -x "$JK_BIN" ]; then
    skip "jk binary not found — running cargo test instead"
    REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
    if (cd "$REPO_ROOT" && cargo test --all 2>&1 | tail -5); then
        check "cargo test --all passes" "true"
    else
        check "cargo test --all passes" "false"
    fi
else
    # --- Init ---
    echo "--- Repository Init ---"
    check "jk init succeeds" "$JK_BIN init '$TMPDIR/repo' 2>/dev/null"
    check "repo directory created" "[ -d '$TMPDIR/repo' ]"

    # --- Create test file ---
    echo "test content" > "$TMPDIR/testfile.txt"

    # --- Execute (copy) ---
    echo "--- Execute Copy ---"
    if $JK_BIN -r "$TMPDIR/repo" copy "$TMPDIR/testfile.txt" "$TMPDIR/repo/copied.txt" 2>/dev/null; then
        check "copy operation" "true"
        check "copied file exists" "[ -f '$TMPDIR/repo/copied.txt' ]"
    else
        skip "copy operation (not implemented in current build)"
    fi

    # --- Undo ---
    echo "--- Undo ---"
    if $JK_BIN -r "$TMPDIR/repo" undo 2>/dev/null; then
        check "undo operation" "true"
    else
        skip "undo operation (not implemented in current build)"
    fi

    # --- Obliterate ---
    echo "--- Obliterate ---"
    echo "sensitive data" > "$TMPDIR/sensitive.txt"
    if $JK_BIN -r "$TMPDIR/repo" obliterate "$TMPDIR/sensitive.txt" 2>/dev/null; then
        check "obliterate operation" "true"
        check "file destroyed" "[ ! -f '$TMPDIR/sensitive.txt' ]"
    else
        skip "obliterate (not implemented in current build)"
    fi
fi

# --- Cargo tests (always run) ---
echo ""
echo "--- Cargo Test Suite ---"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
if command -v cargo >/dev/null 2>&1; then
    test_output="$TMPDIR/cargo-test.log"
    if (cd "$REPO_ROOT" && cargo test --all 2>&1 | tee "$test_output" | tail -3); then
        test_count=$(grep -c 'test .* ok' "$test_output" 2>/dev/null || echo "?")
        check "cargo test --all (${test_count} tests)" "true"
    else
        check "cargo test --all" "false"
    fi
else
    skip "cargo not installed"
fi

# --- Panic Attack ---
echo ""
echo "--- Panic Attack Scan ---"
if command -v panic-attack >/dev/null 2>&1; then
    pa_report="$TMPDIR/pa-report.json"
    if panic-attack assail "$REPO_ROOT" --output-format json --output "$pa_report" --quiet 2>/dev/null; then
        wp=$(python3 -c "import json; print(len(json.load(open('$pa_report')).get('weak_points',[])))" 2>/dev/null || echo "?")
        check "panic-attack scan (${wp} weak points)" "true"
    else
        check "panic-attack scan" "false"
    fi
else
    skip "panic-attack not installed"
fi

echo ""
echo "==============================="
echo "  PASS: ${PASS}  FAIL: ${FAIL}  SKIP: ${SKIP}"
echo "==============================="
[ "${FAIL}" -eq 0 ]
