#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
# CI/gate must be a verdict, not a rubber stamp. Positive control: all green
# passes. Negative cases: failure, cancelled, skipped, empty and malformed
# input each fail. Drift: the gate's `needs:` must list every other job of
# ci.yml; every workflow ci.yml calls must be reusable and must have no
# push/pull_request trigger of its own (else it would run twice).
# shellcheck disable=SC2015,SC2016
set -uo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." || exit 2
fail=0
chk() { # chk NAME WANT-RC JSON
  NEEDS_JSON="$3" bash scripts/ci-gate.sh >/dev/null 2>&1; rc=$?
  if [ "$rc" -eq "$2" ]; then echo "ok   $1"; else echo "FAIL $1: rc=$rc want $2"; fail=1; fi
}
chk "all success passes"      0 '{"a":{"result":"success"},"b":{"result":"success"}}'
chk "one failure fails"       1 '{"a":{"result":"success"},"b":{"result":"failure"}}'
chk "one cancelled fails"     1 '{"a":{"result":"success"},"b":{"result":"cancelled"}}'
chk "one skipped fails"       1 '{"a":{"result":"success"},"b":{"result":"skipped"}}'
chk "empty needs fails"       1 '{}'
chk "malformed fails"         1 'not json'
chk "array input fails"       1 '[{"result":"success"}]'
chk "string input fails"      1 '"x"'
chk "number input fails"      1 '3'
chk "null input fails"        1 'null'
chk "underscore job passes"   0 '{"integration_tests":{"result":"success"}}'

wf=.github/workflows/ci.yml
# drift WORKFLOW: prints "jobs|needs"; equal and non-empty means no drift.
drift() {
  local j n
  j="$(sed -n '/^jobs:/,$ s/^  \([A-Za-z_][A-Za-z0-9_-]*\):[[:space:]]*\(#.*\)\{0,1\}$/\1/p' "$1" | grep -vx 'gate' | sort | tr '\n' ' ')"
  n="$(sed -n 's/^    needs: \[\(.*\)\]$/\1/p' "$1" | tr -d ',' | tr ' ' '\n' | sort | tr '\n' ' ')"
  echo "$j|$n"
}
d="$(drift "$wf")"; jobs="${d%%|*}"; needs="${d#*|}"
if [ -n "$jobs" ] && [ "$jobs" = "$needs" ]; then echo "ok   gate needs every other job ($needs)"; else echo "FAIL needs drift: jobs=[$jobs] needs=[$needs]"; fail=1; fi
# Mutation: an underscore-named job absent from gate.needs must be detected.
mut="$(mktemp)"
sed 's/^  gate:$/  integration_tests:\n    runs-on: ubuntu-latest\n  gate:/' "$wf" > "$mut"
d="$(drift "$mut")"; rm -f "$mut"
if [ "${d%%|*}" != "${d#*|}" ]; then echo "ok   drift check catches an underscore job missing from needs"; else echo "FAIL drift check missed integration_tests"; fail=1; fi
# Mutation: a job header with an inline comment (valid YAML) must be detected too.
mut="$(mktemp)"
sed 's/^  gate:$/  smoke: # quick suite\n    runs-on: ubuntu-latest\n  gate:/' "$wf" > "$mut"
d="$(drift "$mut")"; rm -f "$mut"
if [ "${d%%|*}" != "${d#*|}" ]; then echo "ok   drift check catches a commented job header missing from needs"; else echo "FAIL drift check missed a commented job header"; fail=1; fi
grep -q '^    name: CI/gate$' "$wf" && echo "ok   job name is exactly CI/gate" || { echo "FAIL job name"; fail=1; }
grep -q '^    if: \${{ always() }}$' "$wf" && echo "ok   gate runs always()" || { echo "FAIL gate lacks always()"; fail=1; }

# Every workflow ci.yml calls: reusable, and no direct push/pull_request trigger.
called="$(sed -n 's/^    uses: \(\.\/\.github\/workflows\/[^ ]*\)$/\1/p' "$wf")"
[ -n "$called" ] || { echo "FAIL ci.yml calls no local workflow"; fail=1; }
for f in $called; do
  f="${f#./}"
  # the top-level `on:` block: from `on:` to the next unindented key
  on_block="$(sed -n '/^on:/,/^[a-z]/p' "$f" | sed '$d')"
  grep -q '^  workflow_call:' <<<"$on_block" && echo "ok   $f is reusable" || { echo "FAIL $f lacks workflow_call"; fail=1; }
  if grep -qE '^  (push|pull_request|pull_request_target):' <<<"$on_block"; then
    echo "FAIL $f has its own push/pull_request trigger: it would run twice"; fail=1
  else echo "ok   $f has no push/pull_request trigger of its own"; fi
done
# The gated jobs the owner named must still be reachable from the called workflows.
for j in proofs recheck receipt; do
  grep -q "^  $j:" .github/workflows/agda-proofs.yml && echo "ok   agda-proofs.yml still has job $j" || { echo "FAIL agda-proofs.yml lost job $j"; fail=1; }
done
grep -q '^  dashboard-check:' .github/workflows/dashboard-check.yml && echo "ok   dashboard-check job present" || { echo "FAIL dashboard-check job lost"; fail=1; }
grep -q '^  rust-ci:' .github/workflows/rust-ci.yml && echo "ok   rust-ci job present" || { echo "FAIL rust-ci job lost"; fail=1; }
exit $fail
