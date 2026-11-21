#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
# Recheck legs of the Agda theory theorems (PROOF-POLICY §2): the Rocq files
# under verification/proofs/rocq and the Lean files under
# verification/proofs/lean must check, every `Print Assumptions` and
# `#print axioms` in them must report no axioms, and every fixture under
# tests/reject-rocq and tests/reject-lean must be refused with the message its
# `expect:` line names. Prover versions are asserted by the caller (CI) and
# printed here. ROCQ, LEAN and LAKE override the binaries.
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
export LC_ALL="${LC_ALL:-C.UTF-8}"
rocq="${ROCQ:-coqc}"
lean="${LEAN:-lean}"
lake="${LAKE:-lake}"
"$rocq" --version | head -1
"$lean" --version
"$lake" --version

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
  # Count the commands as Rocq parses them (any run of whitespace between the
  # two words), and fail outright on any report that lists an axiom.
  want="$(grep -cE '^[[:space:]]*Print[[:space:]]+Assumptions[[:space:]]' "$f" || true)"
  got="$(grep -c '^Closed under the global context$' <<<"$out" || true)"
  if grep -q '^Axioms:' <<<"$out"; then
    echo "ERROR: $f rests on assumptions (a Print Assumptions report lists axioms)" >&2; exit 1
  fi
  if [ "$want" -eq 0 ] || [ "$got" -ne "$want" ]; then
    echo "ERROR: $f rests on assumptions ($got of $want Print Assumptions closed)" >&2; exit 1
  fi
  echo "[PASS] rocq leg checks, no assumptions ($want theorems): $f"
done

# Lean: build the legs as the Lake package in verification/proofs/lean
# (PROOF-POLICY §8.2 names `lean` under `lake` as a pinned kernel run), from a
# copy in the work directory so no .lake/ build output lands in the tree. Lake
# prints each `#print axioms` report as an `info: <file>:<line>:<col>:` line.
cp -- verification/proofs/lean/lakefile.toml verification/proofs/lean/lean-toolchain \
  "${lean_src[@]}" "$work/lean/"
out="$(cd "$work/lean" && "$lake" build 2>&1)" || {
  printf '%s\n' "$out"; echo "ERROR: the Lean legs do not build under lake" >&2; exit 1; }
printf '%s\n' "$out"
if grep -q 'depends on axioms' <<<"$out"; then
  echo "ERROR: a Lean leg depends on axioms (an #print axioms report lists them)" >&2; exit 1
fi
for f in "${lean_src[@]}"; do
  name="$(basename -- "$f")"
  # Every `#print axioms` in the file must report none. Count the commands as
  # Lean parses them, and the reports Lake attributes to this file.
  want="$(grep -cE '^[[:space:]]*#print[[:space:]]+axioms[[:space:]]' "$f" || true)"
  got="$(grep -cE "^info: ${name//./\\.}:[0-9]+:[0-9]+: .* does not depend on any axioms\$" <<<"$out" || true)"
  if [ "$want" -eq 0 ] || [ "$got" -ne "$want" ]; then
    echo "ERROR: $f depends on axioms ($got of $want #print axioms report none)" >&2; exit 1
  fi
  echo "[PASS] lean leg checks under lake, no axioms ($want theorems): $f"
done
# A fixture is checked by the same Lean, with the built legs on its path.
lake_lean() { (cd "$work/lean" && "$lake" env "$lean" "$1"); }

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
  reject "$f" lake_lean "$PWD/$f"
done
echo "[check-recheck] OK"
