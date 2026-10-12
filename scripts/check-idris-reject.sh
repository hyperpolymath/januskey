#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
# Expected-rejection fixtures for the Idris2 ABI. Each file under
# tests/reject-idris is appended to a scratch copy of the package as
# JanusKey.ABI.Reject.<Name> and must FAIL to typecheck; a fixture that
# typechecks means a guarantee has been lost. Requires idris2 (0.8.0 in CI).
set -euo pipefail
cd "$(dirname "$0")/.."
command -v idris2 >/dev/null 2>&1 || { echo "FAIL: idris2 not installed" >&2; exit 1; }
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
cp -r src/abi "$tmp/abi"
mkdir -p "$tmp/abi/JanusKey/ABI/Reject" "$tmp/logs"
status=0
for f in tests/reject-idris/*.idr; do
  name=$(basename "$f" .idr)
  cp "$f" "$tmp/abi/JanusKey/ABI/Reject/$name.idr"
  { cat src/abi/januskey-abi.ipkg; echo "        , JanusKey.ABI.Reject.$name"; } > "$tmp/abi/reject-$name.ipkg"
  if (cd "$tmp/abi" && idris2 --typecheck "reject-$name.ipkg" > "$tmp/logs/$name.log" 2>&1); then
    echo "[check-idris-reject] FAIL: $f typechecked but must be refused"; status=1
  else
    echo "[check-idris-reject] refused as expected: $f ($(grep -m1 -E 'Error|error' "$tmp/logs/$name.log" | cut -c1-90))"
  fi
  rm -f "$tmp/abi/JanusKey/ABI/Reject/$name.idr" "$tmp/abi/reject-$name.ipkg"
done
exit $status
