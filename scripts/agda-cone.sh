#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
#
# Import cone and escape-hatch scan for the Agda theory modules: the receipt
# fields PROOF-POLICY §8.2 (PWG-3) and §8.4 rule 6 ask for beyond the kernel
# run itself.
#
#   * the import cone of each module, as Agda reports it (--dependency-graph);
#   * a comment-aware scan of every cone file that is not bound by the checker
#     (first-party and pinned sibling) for `postulate` and for the pragmas and
#     options that switch a check off;
#   * a SHA-256 digest of each scanned file and of the whole input set.
#
# Agda.Primitive and Agda.Builtin.* ship with the pinned checker and bind its
# BUILTINs (Agda.Primitive declares Level by `postulate`, which is how a
# BUILTIN is introduced); they are recorded in the cone and attributed to the
# checker, not scanned.
#
# Usage: agda-cone.sh AGDA RECEIPT_FILE MODULE...   (called by check-agda.sh)
#        agda-cone.sh --self-test
set -euo pipefail
export LC_ALL="${LC_ALL:-C.UTF-8}"

# Print the escape hatches in one Agda source, one "file:line: token" each,
# ignoring `--` line comments, nested `{- -}` block comments and string
# literals. A pragma `{-# ... #-}` is code, not a comment, so it is scanned.
scan_file() {
  awk -v F="$1" '
    function flag(tok) { printf "%s:%d: %s\n", F, NR, tok; hits++ }
    {
      line = $0; out = ""; i = 1; n = length(line)
      while (i <= n) {
        c2 = substr(line, i, 2); c3 = substr(line, i, 3)
        if (depth > 0) {
          if (c2 == "{-" && c3 != "{-#") { depth++; i += 2; continue }
          if (c2 == "-}") { depth--; i += 2; continue }
          i++; continue
        }
        if (inpragma) {
          if (c3 == "#-}") { out = out c3; inpragma = 0; i += 3; continue }
          out = out substr(line, i, 1); i++; continue
        }
        if (instr) {
          ch = substr(line, i, 1)
          if (ch == "\\") { i += 2; continue }
          if (ch == "\"") instr = 0
          i++; continue
        }
        if (c3 == "{-#") { out = out c3; inpragma = 1; i += 3; continue }
        if (c2 == "{-") { depth = 1; i += 2; continue }
        if (c2 == "--" && (i == 1 || substr(line, i - 1, 1) ~ /[ \t(){};]/)) break
        ch = substr(line, i, 1)
        if (ch == "\"") { instr = 1; i++; continue }
        out = out ch; i++
      }
      if (out ~ /(^|[^A-Za-z0-9_-])postulate([^A-Za-z0-9_-]|$)/) flag("postulate")
      if (out ~ /primTrustMe|primEraseEquality/) flag("trust primitive")
      if (out ~ /\{-#[ \t]*(TERMINATING|NON_TERMINATING|NO_POSITIVITY_CHECK|NO_UNIVERSE_CHECK|INJECTIVE|ETA|REWRITE|BUILTIN[ \t]+REWRITE)/) flag("pragma")
      if (out ~ /\{-#[ \t]*OPTIONS/ && out ~ /--(type-in-type|allow-unsolved-metas|allow-incomplete-matches|no-positivity-check|no-termination-check|rewriting|guardedness|sized-types|cubical([ \t]|#|$)|with-K|omega-in-omega|injective-type-constructors)/) flag("option")
    }
    END { exit (hits > 0) }
  ' "$1"
}

self_test() {
  local tmp; tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  printf '%s\n' '-- postulate in a line comment' '{- postulate {- nested -} postulate -}' \
    'x = "postulate"  -- in a string' 'f : Set -- trailing postulate' > "$tmp/clean.agda"
  printf '%s\n' 'module M where' 'postulate' '  A : Set' > "$tmp/post.agda"
  printf '%s\n' '{-# TERMINATING #-}' 'f : Set' > "$tmp/pragma.agda"
  printf '%s\n' '{-# OPTIONS --safe --type-in-type #-}' > "$tmp/option.agda"
  printf '%s\n' '{- a comment -} postulate B : Set' > "$tmp/after.agda"
  scan_file "$tmp/clean.agda" >/dev/null || { echo "self-test: commented postulates flagged" >&2; return 1; }
  local f
  for f in post pragma option after; do
    if scan_file "$tmp/$f.agda" >/dev/null; then echo "self-test: $f.agda not flagged" >&2; return 1; fi
  done
  echo "[agda-cone] scanner self-test: 1 clean control passed, 4 planted hatches flagged"
}

if [ "${1:-}" = "--self-test" ]; then self_test; exit; fi

prover="$1"; receipt="$2"; shift 2
self_test
agda_dir="$("$prover" --print-agda-dir)"
flags=(--safe --without-K --cubical-compatible --no-libraries
       -i verification/proofs/agda -i _build/dependencies/resource-algebra/src)
mkdir -p _build/check-logs
: > "$receipt"
{
  echo "checker: $("$prover" --version)"
  echo "checker-prim-dir: $agda_dir/lib/prim"
  echo "flags: ${AGDA_FLAGS:?check-agda.sh passes the flags in force}"
  echo "standard-library: none (--no-libraries)"
  echo "sibling: hyperpolymath/resource-algebra $(git -C _build/dependencies/resource-algebra rev-parse HEAD)"
  echo "source: $(git rev-parse HEAD 2>/dev/null || echo unknown)"
} >> "$receipt"

declare -A seen=()
scanned=()
for module in "$@"; do
  dot="_build/check-logs/$module.deps.dot"
  "$prover" "${flags[@]}" --dependency-graph="$dot" "verification/proofs/agda/$module.agda" >/dev/null 2>&1 \
    || { echo "ERROR: could not compute the import cone of $module" >&2; exit 1; }
  mapfile -t cone < <(grep -o 'label="[^"]*"' "$dot" | sed 's/^label="//; s/"$//' | sort -u)
  [ "${#cone[@]}" -gt 1 ] || { echo "ERROR: empty import cone for $module" >&2; exit 1; }
  echo "cone($module): ${cone[*]}" >> "$receipt"
  for m in "${cone[@]}"; do
    case "$m" in Agda.Primitive|Agda.Builtin.*) continue ;; esac
    rel="${m//.//}.agda"; file=""
    for dir in verification/proofs/agda _build/dependencies/resource-algebra/src; do
      [ -f "$dir/$rel" ] && { file="$dir/$rel"; break; }
    done
    [ -n "$file" ] || { echo "ERROR: $m is in the cone of $module but no source file was found" >&2; exit 1; }
    [ -n "${seen[$file]:-}" ] && continue
    seen[$file]=1; scanned+=("$file")
  done
done

hits=0
for file in "${scanned[@]}"; do
  echo "input: $(sha256sum "$file" | cut -d' ' -f1)  $file" >> "$receipt"
  scan_file "$file" >> "$receipt" || hits=$((hits + 1))
done
echo "input-digest: $(for f in "${scanned[@]}"; do sha256sum "$f"; done | sha256sum | cut -d' ' -f1) (sha256 over the per-file lines above)" >> "$receipt"
if [ "$hits" -gt 0 ]; then
  echo "escape-hatch-scan: FAILED in $hits of ${#scanned[@]} files" >> "$receipt"
  cat "$receipt"; echo "ERROR: escape hatch in the import cone (see above)" >&2; exit 1
fi
echo "escape-hatch-scan: 0 postulates, trust primitives, check-disabling pragmas or options in ${#scanned[@]} first-party and sibling files; Agda.Primitive and Agda.Builtin.* bound by the checker" >> "$receipt"
cat "$receipt"
