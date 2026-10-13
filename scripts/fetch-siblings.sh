#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
# Fetch the proof sibling at a pinned commit (pattern: choreographic-types,
# residual-evidence-types). The Agda module under verification/proofs/agda
# imports resource-algebra's Loss instance; it is consumed by pin, never
# re-declared here. Re-pin deliberately, one commit per pin.
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
RESOURCE_ALGEBRA_COMMIT="${RESOURCE_ALGEBRA_COMMIT:-cf9235c532712a783a404c359a1055f40134794a}"   # Loss/MaxPlus/HWM as read 2026-10-12
mkdir -p _build/dependencies
if [ ! -d _build/dependencies/resource-algebra ]; then
  git clone --quiet https://github.com/hyperpolymath/resource-algebra.git _build/dependencies/resource-algebra
fi
git -C _build/dependencies/resource-algebra fetch --quiet origin
git -C _build/dependencies/resource-algebra checkout --quiet "$RESOURCE_ALGEBRA_COMMIT"
echo "resource-algebra at $(git -C _build/dependencies/resource-algebra rev-parse --short HEAD)"
