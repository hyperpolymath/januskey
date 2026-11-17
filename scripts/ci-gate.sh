#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
#
# ci-gate.sh — the verdict behind the required status check `CI/gate`.
#
# Reads the `needs` context of a GitHub Actions job (toJSON(needs)) from
# $NEEDS_JSON and succeeds only if at least one job was aggregated and EVERY
# one ended `success`. `failure`, `cancelled` and `skipped` all fail: a skipped
# prover leg is a leg that proved nothing, and a gate that counts it as green is
# a rubber stamp (doctrine 3, fail loudly). An empty or unparseable context
# fails too, so a gate pointed at zero jobs cannot pass.
#
# Exit: 0 all aggregated jobs succeeded; 1 a job did not, or none were given;
#       2 setup error (no jq, no NEEDS_JSON).
set -euo pipefail

command -v jq >/dev/null 2>&1 || { echo "ci-gate: jq is required" >&2; exit 2; }
[ -n "${NEEDS_JSON:-}" ] || { echo "ci-gate: NEEDS_JSON is not set" >&2; exit 2; }

total="$(jq -e 'length' <<<"$NEEDS_JSON")" || { echo "::error::ci-gate: NEEDS_JSON is not a JSON object"; exit 1; }
if [ "$total" -eq 0 ]; then
  echo "::error::CI/gate aggregates no jobs (denominator = 0); refusing to pass."
  exit 1
fi

echo "CI/gate: ${total} aggregated job(s)"
jq -r 'to_entries[] | "  \(.key): \(.value.result)"' <<<"$NEEDS_JSON"

bad="$(jq -r 'to_entries[] | select(.value.result != "success") | "\(.key)=\(.value.result)"' <<<"$NEEDS_JSON")"
if [ -n "$bad" ]; then
  while IFS= read -r line; do echo "::error::CI/gate: required job did not succeed: ${line}"; done <<<"$bad"
  exit 1
fi
echo "CI/gate: all ${total} required job(s) succeeded"
