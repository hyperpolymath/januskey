#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
# Expected-rejection fixtures for the Idris2 ABI. Each file under
# tests/reject-idris is appended to a scratch copy of the package as
# JanusKey.ABI.Reject.<Name> and must FAIL to typecheck; a fixture that
# typechecks means a guarantee has been lost. Each fixture names the refusal
# it exists for in a `-- expect: <text>` line; a fixture refused for another
# reason (a renamed import, a parse error) has stopped testing its claim and
# fails too. Requires idris2 (0.8.0 in CI).
# Usage: check-idris-reject.sh [Name ...]   (default: every fixture)
set -euo pipefail
cd "$(dirname "$0")/.."
command -v idris2 >/dev/null 2>&1 || { echo "FAIL: idris2 not installed" >&2; exit 1; }
if [ "$#" -gt 0 ]; then
  fixtures=(); for n in "$@"; do fixtures+=("tests/reject-idris/$n.idr"); done
else
  shopt -s nullglob; fixtures=(tests/reject-idris/*.idr); shopt -u nullglob
fi
[ "${#fixtures[@]}" -gt 0 ] || { echo "FAIL: no fixtures under tests/reject-idris" >&2; exit 1; }
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
cp -r src/abi "$tmp/abi"
mkdir -p "$tmp/abi/JanusKey/ABI/Reject" "$tmp/logs"
status=0
for f in "${fixtures[@]}"; do
  name=$(basename "$f" .idr)
  if [ ! -f "$f" ]; then echo "[check-idris-reject] FAIL: $f does not exist"; status=1; continue; fi
  want=$(sed -n 's/^-- expect: //p' "$f" | head -1)
  if [ -z "$want" ]; then echo "[check-idris-reject] FAIL: $f has no '-- expect: <text>' line"; status=1; continue; fi
  cp "$f" "$tmp/abi/JanusKey/ABI/Reject/$name.idr"
  { cat src/abi/januskey-abi.ipkg; echo "        , JanusKey.ABI.Reject.$name"; } > "$tmp/abi/reject-$name.ipkg"
  if (cd "$tmp/abi" && idris2 --typecheck "reject-$name.ipkg" > "$tmp/logs/$name.log" 2>&1); then
    echo "[check-idris-reject] FAIL: $f typechecked but must be refused"; status=1
  # Match the message only: Idris echoes source lines (` NN | ...`), and the
  # fixture's own `-- expect:` comment must not count as the refusal.
  elif ! grep -vE '^ *[0-9]+ \| ' "$tmp/logs/$name.log" | grep -qF -- "$want"; then
    cat "$tmp/logs/$name.log"
    echo "[check-idris-reject] FAIL: $f was refused, but not with '$want'"; status=1
  else
    echo "[check-idris-reject] PASS refused as expected: $f ($want)"
  fi
  rm -f "$tmp/abi/JanusKey/ABI/Reject/$name.idr" "$tmp/abi/reject-$name.ipkg"
done
exit $status
