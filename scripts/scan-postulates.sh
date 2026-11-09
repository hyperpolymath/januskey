#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
# Comment-aware postulate scan of Agda source files (PROOF-POLICY §8.4,
# "Axioms travel with the grade"): --safe already refuses `postulate`, and this
# scan is the second of the three parts that make the absence of axioms visible
# on a receipt. check-agda.sh passes it the exact import cone that Agda itself
# reports, so the files scanned are the files checked.
#
# Lexing follows Agda closely enough for one keyword: text inside nested
# {- -} comments, after a `--` that starts a token (with or without a space
# after it), and inside string or character literals is ignored; pragmas
# ({-# … #-}) are code. A `--` inside a name such as `x--y` is part of the name.
#
# Usage: scan-postulates.sh FILE...   Prints each hit as path:line: text and the
# count; exits 1 if there is any hit, 2 on a usage error or an unreadable file.
set -euo pipefail
[ "$#" -gt 0 ] || { echo "usage: $0 FILE..." >&2; exit 2; }
for f in "$@"; do
  [ -f "$f" ] && [ -r "$f" ] || { echo "ERROR: not a readable file: $f" >&2; exit 2; }
done
hits="$(awk '
  # Agda name characters are everything except whitespace and .;{}()@"
  function namechar(c) { return c != "" && c !~ /[ \t.;{}()@"]/ }
  FNR == 1 { depth = 0 }
  {
    line = $0; code = ""; i = 1; n = length(line)
    while (i <= n) {
      c = substr(line, i, 1); two = substr(line, i, 2)
      prev = (i > 1) ? substr(line, i - 1, 1) : ""
      if (depth == 0 && two == "{-" && substr(line, i + 2, 1) == "#") {
        code = code "{-#"; i += 3; continue
      }
      if (two == "{-") { depth++; i += 2; continue }
      if (depth > 0 && two == "-}") { depth--; i += 2; continue }
      if (depth > 0) { i++; continue }
      if (c == "\"") {                     # string literal: skip to the closing quote
        i++
        while (i <= n && substr(line, i, 1) != "\"") i += (substr(line, i, 1) == "\\") ? 2 : 1
        i++; code = code " "; continue
      }
      if (c == "'\''" && !namechar(prev)) { # character literal at a token start
        i++
        while (i <= n && substr(line, i, 1) != "'\''") i += (substr(line, i, 1) == "\\") ? 2 : 1
        i++; code = code " "; continue
      }
      if (two == "--" && !namechar(prev)) break
      code = code c; i++
    }
    if (code ~ /(^|[ \t.;{}()@])postulate([ \t.;{}()@]|$)/)
      printf "%s:%d: %s\n", FILENAME, FNR, line
  }
' "$@")"
count=0
[ -n "$hits" ] && count="$(printf '%s\n' "$hits" | wc -l | tr -d ' ')"
[ -n "$hits" ] && printf '%s\n' "$hits"
echo "postulate scan: $# files, $count postulate(s)"
[ "$count" -eq 0 ] || exit 1
