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
flags=(--safe --without-K --no-libraries --ignore-interfaces --double-check
       -i verification/proofs/agda -i _build/dependencies/resource-algebra/src)
"$prover" "${flags[@]}" verification/proofs/agda/Reversibility.agda
mkdir -p _build/check-logs
shopt -s nullglob
rejects=(tests/reject-agda/*.agda)
shopt -u nullglob
for file in "${rejects[@]}"; do
  name="$(basename -- "$file" .agda)"
  log="_build/check-logs/$name.log"
  if "$prover" "${flags[@]}" -i tests/reject-agda "$file" >"$log" 2>&1; then
    cat "$log"; echo "ERROR: $file was accepted but must be refused" >&2; exit 1
  fi
  echo "refused as expected: $file ($(grep -m1 -E 'error|Error' "$log" | cut -c1-80))"
done
echo "[check-agda] OK"
