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
modules=(Reversibility TransactionAtomicity Epistemic)
for module in "${modules[@]}"; do
  "$prover" "${flags[@]}" "verification/proofs/agda/$module.agda"
done
mkdir -p _build/check-logs
# PWG-3 receipt fields beyond the kernel run (PROOF-POLICY §8.2, §8.4 rule 6):
# the import cone of each module, a comment-aware scan of it for postulates
# and check-disabling pragmas, and the input digests. Printed into this log.
AGDA_FLAGS="${flags[*]}" bash scripts/agda-cone.sh "$prover" _build/check-logs/agda-receipt.txt "${modules[@]}"
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
