#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
# Agda gate for verification/proofs/agda: kernel flags, sibling on the include
# path, and the expected-rejection fixtures under tests/reject-agda refused.
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
export LC_ALL="${LC_ALL:-C.UTF-8}"
prover="${AGDA:-agda}"
"$prover" --version
bash scripts/fetch-siblings.sh
# PROOF-POLICY §2.1 canonical invocation: --safe --without-K --cubical-compatible
# (standards/3-practice/PROOF-POLICY.adoc). The cubical-compatible flag is
# infectious in principle — direct imports must use it too — but Agda 2.6.4.3
# accepts a --without-K sibling on the include path and reports the pattern
# matches that will not compute under transports as UnsupportedIndexedMatch
# warnings (14 in Reversibility, 16 in TransactionAtomicity, 2 in Epistemic; the
# sibling's ≤-trans, obliterate-absorbs and just-inj). Those are warnings about
# computation under transport, not about the propositions, so the gate keeps
# them visible rather than silencing them with -WnoUnsupportedIndexedMatch.
flags=(--safe --without-K --cubical-compatible --no-libraries --ignore-interfaces --double-check
       -i verification/proofs/agda -i _build/dependencies/resource-algebra/src)
# The working directory for this run: the dependency graphs Agda writes and the
# scan's controls. Removed on exit.
work="$(mktemp -d)"
trap 'rm -rf -- "$work"' EXIT
modules=(Reversibility TransactionAtomicity Epistemic)
for module in "${modules[@]}"; do
  "$prover" "${flags[@]}" --dependency-graph="$work/$module.dot" "verification/proofs/agda/$module.agda"
done

# PROOF-POLICY §8.4: a comment-aware postulate scan of the import cone. The cone
# is the module set Agda itself reports for the three roots (Agda.Primitive and
# the Agda.Builtin modules included); each module is resolved to its source on
# the include path or in Agda's own library, and an unresolved module fails.
agda_prim="$("$prover" --print-agda-dir)/lib/prim"
own=(); prover_lib=()
while IFS= read -r name; do
  found=""
  for root in verification/proofs/agda _build/dependencies/resource-algebra/src "$agda_prim"; do
    candidate="$root/${name//.//}.agda"
    if [ -f "$candidate" ]; then found="$root"; break; fi
  done
  [ -n "$found" ] || { echo "ERROR: cannot resolve module $name of the import cone to a source file" >&2; exit 1; }
  if [ "$found" = "$agda_prim" ]; then prover_lib+=("$candidate"); else own+=("$candidate"); fi
done < <(sed -n 's/.*label="\([^"]*\)".*/\1/p' "$work"/*.dot | LC_ALL=C sort -u)
[ "${#own[@]}" -gt 0 ] || { echo "ERROR: Agda reported no first-party module in the import cone" >&2; exit 1; }
echo "import cone: ${#own[@]} first-party modules, ${#prover_lib[@]} supplied with the prover"
# The proofs and the pinned sibling: no postulate at all.
bash scripts/scan-postulates.sh "${own[@]}"
# Modules supplied with the prover are its trusted base. The only postulates
# allowed there are Agda.Primitive's universe-level primitives (Level, lzero,
# lsuc, _⊔_, each bound by a BUILTIN pragma); they are reported, not hidden, and
# a postulate in any other prover-supplied module of the cone fails the gate.
if [ "${#prover_lib[@]}" -gt 0 ]; then
  status=0; bash scripts/scan-postulates.sh "${prover_lib[@]}" >"$work/prover.log" 2>&1 || status=$?
  [ "$status" -le 1 ] || { cat "$work/prover.log"; exit 1; }
  if grep -E '^[^ ]+:[0-9]+: ' "$work/prover.log" | grep -vqF -- "/Agda/Primitive.agda:"; then
    cat "$work/prover.log"; echo "ERROR: a postulate in a prover-supplied module other than Agda.Primitive" >&2; exit 1
  fi
  echo "prover-supplied: $(grep -cE '^[^ ]+:[0-9]+: ' "$work/prover.log" || true) postulate block(s), all in Agda.Primitive (universe-level primitives)"
fi

# The controls are written at run time, so no file in the tree carries a real
# postulate for another scanner to count. Each must give exactly its exit code.
printf '%s\n' 'module CommentsOnly where' '-- postulate in a line comment' \
  '--postulate with no space after the dashes' \
  '{- postulate {- nested postulate -} postulate -}' \
  'x--postulate : Set₁' 'x--postulate = Set' \
  's : String' 's = "a postulate in a string \" still the string"' \
  > "$work/CommentsOnly.agda"
printf '%s\n' 'module Planted where' '{- c -} postulate X : Set' > "$work/Planted.agda"
status=0; bash scripts/scan-postulates.sh "$work/CommentsOnly.agda" >"$work/neg.log" 2>&1 || status=$?
[ "$status" -eq 0 ] || { cat "$work/neg.log"; echo "ERROR: the comments-only control did not exit 0 (got $status)" >&2; exit 1; }
status=0; bash scripts/scan-postulates.sh "$work/Planted.agda" >"$work/pos.log" 2>&1 || status=$?
if [ "$status" -ne 1 ] || ! grep -qF -- "Planted.agda:2:" "$work/pos.log"; then
  cat "$work/pos.log"; echo "ERROR: the planted control did not exit 1 naming Planted.agda:2 (got $status)" >&2; exit 1
fi
echo "[PASS] postulate scan: cone clean, both controls behave"
mkdir -p _build/check-logs
shopt -s nullglob
rejects=(tests/reject-agda/*.agda)
shopt -u nullglob
[ "${#rejects[@]}" -gt 0 ] || { echo "ERROR: no fixtures under tests/reject-agda" >&2; exit 1; }
# Each fixture names the refusal it exists for in a `-- expect: <text>` line;
# a fixture refused for any other reason (a broken import, a parse error) has
# stopped testing its claim, so it fails the gate too.
for file in "${rejects[@]}"; do
  name="$(basename -- "$file" .agda)"
  log="_build/check-logs/$name.log"
  want="$(sed -n 's/^-- expect: //p' "$file" | head -1)"
  [ -n "$want" ] || { echo "ERROR: $file has no '-- expect: <text>' line" >&2; exit 1; }
  if "$prover" "${flags[@]}" -i tests/reject-agda "$file" >"$log" 2>&1; then
    cat "$log"; echo "ERROR: $file was accepted but must be refused" >&2; exit 1
  fi
  if ! grep -qF -- "$want" "$log"; then
    cat "$log"; echo "ERROR: $file was refused, but not with '$want'" >&2; exit 1
  fi
  echo "[PASS] refused as expected: $file ($want)"
done
echo "[check-agda] OK"
