#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
# Comment-aware postulate scan of an Agda import cone (PROOF-POLICY §8.4,
# "Axioms travel with the grade"): --safe already refuses `postulate`, and this
# scan is the second of the three parts that make the absence of axioms visible
# on a receipt. Text inside nested {- -} comments and after a `--` line comment
# is ignored; pragmas ({-# … #-}) are treated as code. A `--` that starts an
# operator (`-->`) is not a comment, so the scan errs towards reporting.
#
# Usage: scan-postulates.sh DIR...   Prints each hit as path:line: text and the
# count; exits 1 if there is any hit, 2 if no .agda file was found.
set -euo pipefail
[ "$#" -gt 0 ] || { echo "usage: $0 DIR..." >&2; exit 2; }
mapfile -t files < <(find "$@" -name '*.agda' -type f | LC_ALL=C sort)
[ "${#files[@]}" -gt 0 ] || { echo "ERROR: no .agda files under: $*" >&2; exit 2; }
hits="$(awk '
  FNR == 1 { depth = 0 }
  {
    line = $0; code = ""; i = 1; n = length(line)
    while (i <= n) {
      two = substr(line, i, 2)
      if (depth == 0 && two == "{-" && substr(line, i + 2, 1) == "#") {
        code = code "{-#"; i += 3; continue
      }
      if (two == "{-") { depth++; i += 2; continue }
      if (depth > 0 && two == "-}") { depth--; i += 2; continue }
      if (depth > 0) { i++; continue }
      if (two == "--" && (i + 2 > n || substr(line, i + 2, 1) ~ /[ \t]/)) break
      code = code substr(line, i, 1); i++
    }
    if (code ~ /(^|[^A-Za-z0-9_-])postulate([^A-Za-z0-9_-]|$)/)
      printf "%s:%d: %s\n", FILENAME, FNR, line
  }
' "${files[@]}")"
count=0
[ -n "$hits" ] && count="$(printf '%s\n' "$hits" | wc -l | tr -d ' ')"
[ -n "$hits" ] && printf '%s\n' "$hits"
echo "postulate scan: ${#files[@]} files, $count postulate(s)"
[ "$count" -eq 0 ]
