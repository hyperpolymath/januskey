#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
# Recheck legs of the Agda theory theorems (PROOF-POLICY §2): the Rocq files
# under verification/proofs/rocq and the Lean files under
# verification/proofs/lean must check, every `Print Assumptions` and
# `#print axioms` in them must report no axioms, and every fixture under
# tests/reject-rocq and tests/reject-lean must be refused with the message its
# `expect:` line names. Prover versions are asserted by the caller (CI) and
# printed here. ROCQ and LEAN override the binaries.
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
export LC_ALL="${LC_ALL:-C.UTF-8}"
rocq="${ROCQ:-coqc}"
lean="${LEAN:-lean}"
"$rocq" --version | head -1
"$lean" --version

work="$(mktemp -d)"
trap 'rm -rf -- "$work"' EXIT
mkdir -p "$work/rocq" "$work/lean"

shopt -s nullglob
rocq_src=(verification/proofs/rocq/*.v)
lean_src=(verification/proofs/lean/*.lean)
rocq_rej=(tests/reject-rocq/*.v)
lean_rej=(tests/reject-lean/*.lean)
shopt -u nullglob
for set in rocq_src lean_src rocq_rej lean_rej; do
  declare -n arr="$set"
  [ "${#arr[@]}" -gt 0 ] || { echo "ERROR: no files for $set" >&2; exit 1; }
done

# Rocq: compile each leg under the JanusKey prefix; `Print Assumptions` at the
# foot of each file must report a closed theorem.
for f in "${rocq_src[@]}"; do
  cp -- "$f" "$work/rocq/"
done
for f in "${rocq_src[@]}"; do
  name="$(basename -- "$f")"
  out="$(cd "$work/rocq" && "$rocq" -q -Q . JanusKey "$name" 2>&1)" || {
    printf '%s\n' "$out"; echo "ERROR: $f does not check" >&2; exit 1; }
  printf '%s\n' "$out"
  # Every `Print Assumptions` in the file must report a closed theorem: a file
  # with several theorems must not pass on the strength of one of them.
  want="$(grep -c '^Print Assumptions ' "$f" || true)"
  got="$(grep -c '^Closed under the global context$' <<<"$out" || true)"
  if [ "$want" -eq 0 ] || [ "$got" -ne "$want" ]; then
    echo "ERROR: $f rests on assumptions ($got of $want Print Assumptions closed)" >&2; exit 1
  fi
  echo "[PASS] rocq leg checks, no assumptions ($want theorems): $f"
done

# Lean: compile each leg to an .olean so the fixtures can import it; the
# `#print axioms` line must report no axioms.
for f in "${lean_src[@]}"; do
  mod="$(basename -- "$f" .lean)"
  out="$(LEAN_PATH="$work/lean" "$lean" -o "$work/lean/$mod.olean" "$f" 2>&1)" || {
    printf '%s\n' "$out"; echo "ERROR: $f does not check" >&2; exit 1; }
  printf '%s\n' "$out"
  # Every `#print axioms` in the file must report none.
  want="$(grep -c '^#print axioms ' "$f" || true)"
  got="$(grep -c 'does not depend on any axioms' <<<"$out" || true)"
  if [ "$want" -eq 0 ] || [ "$got" -ne "$want" ]; then
    echo "ERROR: $f depends on axioms ($got of $want #print axioms report none)" >&2; exit 1
  fi
  echo "[PASS] lean leg checks, no axioms ($want theorems): $f"
done

# Expected rejections: refused, and refused for the stated reason.
expect_of() { sed -n 's/^.*expect: \(.*\)$/\1/p' "$1" | head -1 | sed 's/ \*)$//'; }
reject() {
  local file="$1"; shift
  local want; want="$(expect_of "$file")"
  [ -n "$want" ] || { echo "ERROR: $file has no 'expect: <text>' line" >&2; exit 1; }
  local log
  if log="$("$@" 2>&1)"; then
    printf '%s\n' "$log"; echo "ERROR: $file was accepted but must be refused" >&2; exit 1
  fi
  grep -qF -- "$want" <<<"$log" || {
    printf '%s\n' "$log"; echo "ERROR: $file was refused, but not with '$want'" >&2; exit 1; }
  echo "[PASS] refused as expected: $file ($want)"
}
# Fixtures are compiled from a copy in the work directory, so no build output
# (.glob, .aux) lands in the tree.
mkdir -p "$work/reject-rocq"
for f in "${rocq_rej[@]}"; do
  cp -- "$f" "$work/reject-rocq/"
  reject "$f" "$rocq" -q -Q "$work/rocq" JanusKey "$work/reject-rocq/$(basename -- "$f")"
done
for f in "${lean_rej[@]}"; do
  reject "$f" env LEAN_PATH="$work/lean" "$lean" "$f"
done
echo "[check-recheck] OK"
