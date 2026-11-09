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
# allowed there are Agda.Primitive's universe-level primitives: exactly Level,
# lzero, lsuc and _⊔_, each bound by its BUILTIN pragma. The gate checks the
# declared names and the bindings, not the file name, so a prover pin that adds
# a postulate to Agda.Primitive fails here; a postulate in any other
# prover-supplied module of the cone fails too.
if [ "${#prover_lib[@]}" -gt 0 ]; then
  status=0; bash scripts/scan-postulates.sh "${prover_lib[@]}" >"$work/prover.log" 2>&1 || status=$?
  [ "$status" -le 1 ] || { cat "$work/prover.log"; exit 1; }
  grep -E '^[^ ]+:[0-9]+: ' "$work/prover.log" >"$work/prover.hits" || true
  prim="$agda_prim/Agda/Primitive.agda"
  # Every hit is in Agda.Primitive and is a bare `postulate` line opening a block,
  # the only shape the name check below reads.
  if grep -vqE -- "^$prim:[0-9]+: postulate[[:space:]]*\$" "$work/prover.hits"; then
    cat "$work/prover.log"; echo "ERROR: a prover-supplied postulate outside the Agda.Primitive level blocks" >&2; exit 1
  fi
  # The names those blocks declare: the indented lines after `postulate`, up to the
  # next line that is not indented; line comments removed.
  declared="$(LC_ALL=C awk '
    { sub(/--.*/, "") }
    /^postulate[ \t]*$/ { inblock = 1; next }
    inblock && /^[ \t]*$/ { next }
    inblock && /^[ \t]/ { split($0, lhs, ":"); n = split(lhs[1], names, /[ \t]+/)
                          for (k = 1; k <= n; k++) if (names[k] != "") print names[k]; next }
    { inblock = 0 }
  ' "$prim" | LC_ALL=C sort | tr '\n' ' ')"
  expected="Level _⊔_ lsuc lzero "
  [ "$declared" = "$expected" ] || {
    echo "ERROR: Agda.Primitive postulates '$declared', expected exactly '$expected'" >&2; exit 1; }
  for binding in "LEVEL Level" "LEVELZERO lzero" "LEVELSUC lsuc" "LEVELMAX _⊔_"; do
    LC_ALL=C grep -qE -- "^\{-#[[:space:]]+BUILTIN[[:space:]]+${binding% *}[[:space:]]+${binding#* }[[:space:]]+#-\}" "$prim" || {
      echo "ERROR: Agda.Primitive does not bind ${binding#* } with BUILTIN ${binding% *}" >&2; exit 1; }
  done
  echo "prover-supplied: Agda.Primitive postulates exactly ${expected% }, each bound by its BUILTIN pragma ($(wc -l <"$work/prover.hits" | tr -d ' ') block(s)); no other postulate"
fi

# The controls are written at run time, so no file in the tree carries a real
# postulate for another scanner to count. Each must give exactly its exit code.
printf '%s\n' 'module CommentsOnly where' '-- postulate in a line comment' \
  '--postulate postulate: no space after the dashes' \
  '{- postulate {- nested postulate -} postulate -}' \
  'x--postulate : Set₁' 'x--postulate = Set' \
  's : String' 's = "a postulate in a string \" still the string"' \
  '{-# FOREIGN GHC postulate = () #-}' \
  > "$work/CommentsOnly.agda"
# One planted postulate per line, 2 to 5: after a block comment; bounded by
# NO-BREAK SPACE (U+00A0), which Agda reads as whitespace; separated from its
# name by a block comment; after a pragma whose payload holds `{- {-`, which Agda
# does not read as a comment there.
printf '%s\n' 'module Planted where' '{- c -} postulate X : Set' \
  $'\302\240postulate\302\240Y : Set' 'postulate{- c -}Z : Set' \
  '{-# FOREIGN GHC {- {- #-} postulate W : Set' > "$work/Planted.agda"
status=0; bash scripts/scan-postulates.sh "$work/CommentsOnly.agda" >"$work/neg.log" 2>&1 || status=$?
[ "$status" -eq 0 ] || { cat "$work/neg.log"; echo "ERROR: the comments-only control did not exit 0 (got $status)" >&2; exit 1; }
status=0; bash scripts/scan-postulates.sh "$work/Planted.agda" >"$work/pos.log" 2>&1 || status=$?
missed=""
for line in 2 3 4 5; do grep -qF -- "Planted.agda:$line:" "$work/pos.log" || missed="$missed $line"; done
if [ "$status" -ne 1 ] || [ -n "$missed" ]; then
  cat "$work/pos.log"; echo "ERROR: the planted control did not exit 1 naming lines 2-5 (got $status; missed:${missed:- none})" >&2; exit 1
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
