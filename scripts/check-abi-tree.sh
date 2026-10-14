#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
# One ABI tree. Fails when the Idris2 ABI surface is duplicated, symlinked,
# mis-cased or inconsistent with its package file. Pure shell: runs without
# idris2, so it can gate PRs on hosts that cannot typecheck.
set -euo pipefail
cd "$(dirname "$0")/.."
fail=0
# say MSG...: print MSG prefixed with the script's tag to stdout.
say() { echo "[check-abi-tree] $*"; }

# 1. The symlink farm (PR #140, src/idris-abi/) is retired.
if [ -e src/idris-abi ]; then say "FAIL: src/idris-abi exists (retired symlink farm)"; fail=1; fi

# 2. Exactly one package file, and it is the one the gates name.
n=$(find src/abi -name '*.ipkg' | wc -l)
if [ "$n" -ne 1 ]; then say "FAIL: expected 1 .ipkg under src/abi, found $n"; fail=1; fi
if [ ! -f src/abi/januskey-abi.ipkg ]; then say "FAIL: src/abi/januskey-abi.ipkg missing"; fail=1; fi

# 3. Exactly one Foreign.idr under src/ (the FFI surface is declared once).
n=$(find src -name 'Foreign.idr' | wc -l)
if [ "$n" -ne 1 ]; then say "FAIL: expected 1 Foreign.idr under src/, found $n"; fail=1; fi

# 4. No symlinks under src/abi.
n=$(find src/abi -type l | wc -l)
if [ "$n" -ne 0 ]; then say "FAIL: $n symlink(s) under src/abi"; fail=1; fi

# 5. Every module declaration matches its path (this is what caught the
#    Januskey/JanusKey casing split).
while IFS= read -r f; do
  rel=${f#src/abi/}; want=${rel%.idr}; want=${want//\//.}
  have=$(grep -m1 -E '^module ' "$f" | awk '{print $2}')
  if [ "$have" != "$want" ]; then say "FAIL: $f declares module '$have'; path says '$want'"; fail=1; fi
done < <(find src/abi -name '*.idr' | sort)

# 6. Every module the package lists has a file; every file is listed.
listed=$(sed -n '/^modules *=/,/^[^ ,]/p' src/abi/januskey-abi.ipkg | tr ',' '\n' | sed 's/^modules *=//; s/^[ ,]*//; s/[ ,]*$//' | grep -E '^[A-Z]' || true)
for m in $listed; do
  p="src/abi/${m//./\/}.idr"
  if [ ! -f "$p" ]; then say "FAIL: ipkg lists $m but $p is missing"; fail=1; fi
done
while IFS= read -r f; do
  rel=${f#src/abi/}; mod=${rel%.idr}; mod=${mod//\//.}
  if ! grep -qx "$mod" <<< "$listed"; then say "FAIL: $f is not listed in src/abi/januskey-abi.ipkg"; fail=1; fi
done < <(find src/abi -name '*.idr' | sort)

# 7. Report, do not fail, on generated trees: they are not part of this gate.
if [ -d generated/idrisiser ]; then
  say "note: generated/idrisiser is present and is NOT part of the ABI gate (its modules do not typecheck; see PROOF-NEEDS.adoc)"
fi

if [ "$fail" -ne 0 ]; then say "FAIL"; exit 1; fi
say "OK: one ABI tree ($(find src/abi -name '*.idr' | wc -l) modules, 1 package)"
