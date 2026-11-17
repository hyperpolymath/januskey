#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
#
# proof-receipt.sh: the proof receipt of this repository,
# verification/receipts/januskey_praxis.deed (PROOF-POLICY §8.5.1, profile
# proof-receipt), written and checked by the estate's shared generator
# (standards scripts/proof-receipt.sh).  This file holds only what is
# januskey's own: the claim list, the run ledger, the run scopes, and the
# hooks that name each leg's invocation and input set.  The generator, the
# reader, the reference digest and the §8.5.3 git checker are fetched below
# at one standards commit, each checked against its sha256.
#
#   --write   write the receipt from the ledgers and git
#   --check   regenerate it and refuse any difference, run the §8.5.3 git
#             checks, check that each current receipt still describes the
#             tree, and run the pinned reader with its negative control
#   --run-row PROVER --run URL --commit SHA --checker TEXT --reported TEXT
#             print the ledger row of a run that just passed
#
# What is typed and what is derived.  The claim list and the run ledger are
# typed here, each ledger row copied from the CI log of its run.  Every
# digest is a git fact: a current row's digest is rechecked against the tree
# by the generator, and a superseded row's digest is the reference digest of
# its leg's input set at that row's commit (scripts/proof-inputs-sha256.sh
# COMMIT PATHS...).  The grade is never typed: the generator derives it and
# the pinned reader derives it again, refusing a receipt that overstates it.
#
# Needs: git, sha256sum, curl, bun.
# The receipt_* hooks below are called by the sourced generator, not here.
# shellcheck disable=SC2317
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
export LC_ALL=C.UTF-8

# ── The pinned standard ───────────────────────────────────────────────────────
RECEIPT_STANDARDS_COMMIT=15ef29e4b3229f5d70f5a4d817ab4377568ed503
RECEIPT_STD=_build/standards
standards_raw="https://raw.githubusercontent.com/metadatastician/standards/$RECEIPT_STANDARDS_COMMIT"
pinned_files=(
  "scripts/proof-receipt.sh:d7d8977e57f6f8fab44d6258987e98eb26169f394e2f647b3d99c47ee87945c4"
  "1-formats/deed/tools/deed_lint.js:8978546740ca2d6a1d90b769fe02159883cb18f1eb2c44b2eeb737bab341dcd2"
  "scripts/proof-inputs-sha256.sh:dfc6dde01a3a79df638eebe4cad0eae584d8f17280077e149174c0667e9508f4"
  "scripts/check-proof-receipt.sh:f2375747238464535d1b4a61f658f07cb2f8504c69bfb1812311c79429a4e23e"
)
fetch_standard() {
  local pair rel sha f
  for pair in "${pinned_files[@]}"; do
    rel="${pair%%:*}"; sha="${pair#*:}"; f="$RECEIPT_STD/$rel"
    if [ ! -f "$f" ] || ! printf '%s  %s\n' "$sha" "$f" | sha256sum -c --quiet - 2>/dev/null; then
      mkdir -p "$(dirname "$f")"
      curl --proto '=https' --tlsv1.2 -sSfL -o "$f" "$standards_raw/$rel"
    fi
    printf '%s  %s\n' "$sha" "$f" | sha256sum -c --quiet - \
      || { printf 'ERROR: %s does not match its pinned sha256\n' "$f" >&2; exit 1; }
  done
}
fetch_standard

RECEIPT_OUT=verification/receipts/januskey_praxis.deed
RECEIPT_REPOSITORY=hyperpolymath/januskey
RECEIPT_DEFAULT_BRANCH=main
RECEIPT_CANONICAL_NAME=januskey-proof-receipt
RECEIPT_SCRIPT=scripts/proof-receipt.sh
sibling_repo=hyperpolymath/resource-algebra
sibling_dir=_build/dependencies/resource-algebra

# ── The claim list ────────────────────────────────────────────────────────────
# The theory theorems of PROOF-NEEDS.adoc (PROOF-POLICY §1): id | §5 token of
# the Agda proof | kind | Rocq route | Rocq §2.2 state | Lean route | Lean
# state | reasons.  Each has both recheck legs, re-proved by hand and
# reconciled statement by statement (docs/theory/PROOF-GRADES.adoc).  G-3
# names the FFI bridge, so it is engineering under §1 and has no claim here.
receipt_claims() {
  local id
  for id in G-1 T-1 T-2 T-3 E-1m; do
    printf '%s|proven|theorem|re-proved|RECONCILED|re-proved|RECONCILED||\n' "$id"
  done
}

# ── The run ledger ────────────────────────────────────────────────────────────
# prover | run | commit | checker | invocation | inputs sha256 | what the run
# reported | state | reason | axioms [| imports [| trusted base]].  Copied
# from the logs of agda-proofs.yml; a current row is printed by --run-row.
# Every gate fails on any axiom (check-agda.sh: --safe and the cone scan;
# check-recheck.sh: every Print Assumptions closed, every #print axioms
# empty), so no row lists one.
agda_checker='Agda version 2.6.4.3 (Debian trixie agda-bin=2.6.4.3-1+b2, container debian:trixie-slim@sha256:a29215f6a35e51e22adffa17f89e9d2ef06214e64a2bad10d765c46aea49f11f)'
agda_flags='agda --safe --without-K --cubical-compatible --no-libraries --ignore-interfaces --double-check -i verification/proofs/agda -i _build/dependencies/resource-algebra/src'
rocq_checker='The Coq Proof Assistant, version 8.18.0 (Ubuntu 24.04 coq=8.18.0+dfsg-1build2)'
lean_checker='Lean (version 4.35.0, x86_64-unknown-linux-gnu, commit eb6cafb7290e8bd5f2937522d8d804f58f02bafc, Release) (release tarball, sha256 7289597a44543246c9227b8cae501767e095ce17f6d448b95fc4a8dc97268f6d)'
agda_said_cone='import cone 7 first-party modules and 8 supplied with the prover; postulate scan 7 files, 0 postulates; 7 fixtures refused as expected; [check-agda] OK'
receipt_runs() {
  local u=https://github.com/hyperpolymath/januskey/actions/runs
  local e=e8deeaf7bc7b70500d57c282828512c8f8ba81f9
  local agda_inv3="$agda_flags verification/proofs/agda/{Reversibility,TransactionAtomicity,Epistemic}.agda (scripts/check-agda.sh)"
  local rocq_t1='coqc -q -Q . JanusKey Rollback.v (a copy of verification/proofs/rocq/Rollback.v; scripts/check-recheck.sh)'
  local lean_t1='lean -o Rollback.olean verification/proofs/lean/Rollback.lean (scripts/check-recheck.sh)'
  local rocq_said_t1='1 rocq leg checks with no assumptions (Rollback); the job refused its 2 fixtures (1 Rocq, 1 Lean) as expected; [check-recheck] OK'
  local lean_said_t1='1 lean leg checks with no axioms (Rollback); the job refused its 2 fixtures (1 Rocq, 1 Lean) as expected; [check-recheck] OK'
  cat <<ROWS
agda|$u/43945986467|$e|$agda_checker|$(receipt_invocation agda)|6e17ce4ad8264cd09de228eaa06e212caeb4d090436a4d9373078ee7978800d8|$agda_said_cone|current|||hyperpolymath/resource-algebra@cf9235c532712a783a404c359a1055f40134794a=adddbf38fe2ce754f50fc70f5743f381763fb6387e5f99ce8a7d0a38176df9c9|0c1f64c3c7f0a26cbc92595168858485a9883f6196d287497dcc805faf5c58eb
agda|$u/43213340167|b5034884dbe945139d03b38fe80cfdf1106a77e1|$agda_checker|$(receipt_invocation agda)|6e17ce4ad8264cd09de228eaa06e212caeb4d090436a4d9373078ee7978800d8|$agda_said_cone|superseded|the run at hyperpolymath/januskey#179; replaced by the run at #180's merge, the receipt's source commit|
agda|$u/42736066407|e16543866fa4fd31a94f3c7faa1fe0671b083a1b|$agda_checker|$(receipt_invocation agda)|6e17ce4ad8264cd09de228eaa06e212caeb4d090436a4d9373078ee7978800d8|$agda_said_cone|superseded|the first green run on main of the gate as changed by hyperpolymath/januskey#174; replaced by the run at the receipt's source commit|
agda|$u/42076969251|23bc8c38160a25c37917bce69cf05d91899b07e5|$agda_checker|$agda_inv3|a56ccfa9187aa5a285210ef380af91eb1d857e9f6e32c1275b87498923092787|6 fixtures refused as expected; [check-agda] OK|superseded|before the import-cone postulate scan (hyperpolymath/januskey#170) and before SingleActiveLog.agda (hyperpolymath/januskey#171)|
agda|$u/41687715055|fc188c44906faedfe91bfd414be3fda01ff4caf4|$agda_checker|$agda_inv3|a56ccfa9187aa5a285210ef380af91eb1d857e9f6e32c1275b87498923092787|6 fixtures refused as expected; [check-agda] OK|superseded|canonical flags and three modules; replaced by run 42076969251 at a later commit|
agda|$u/40867211509|4afb710e86646658f667f10bd6274ee47416bfc7|$agda_checker|agda --safe --without-K (before --cubical-compatible joined the gate)|14b816f8950f28250ca82e4c77c7b2771dce30b405fa55c23ac7d0ad0ebd4591|4 fixtures refused as expected; [check-agda] OK|superseded|before --cubical-compatible and before Epistemic.agda; it never covered E-1m, so it is listed for the other claims only|
rocq|$u/43945986467|$e|$rocq_checker|$(receipt_invocation rocq)|2f9a91ec283573822579d64feb5db2faf4d028e5597578c25b14b8ff865e5824|4 rocq legs check with no assumptions (Epistemic, Reversibility, Rollback, SingleActiveLog); the job refused its 8 fixtures (4 Rocq, 4 Lean) as expected; [check-recheck] OK|current||
rocq|$u/43213340167|b5034884dbe945139d03b38fe80cfdf1106a77e1|$rocq_checker|$rocq_t1|2e86900da60f3e8cde1198f69fc6f7b0b9d73c5fe0ab2ae42f7b66ef1b7fd50d|$rocq_said_t1|superseded|T-1's leg only, under the gate before hyperpolymath/januskey#180; replaced by the run at #180's merge|
rocq|$u/42646851111|0834ce73717a1faafa23fc1df55d85d54ee1e2b7|$rocq_checker|$rocq_t1|2e86900da60f3e8cde1198f69fc6f7b0b9d73c5fe0ab2ae42f7b66ef1b7fd50d|$rocq_said_t1|superseded|the first run on main (hyperpolymath/januskey#171); replaced by the run at the receipt's source commit|
lean|$u/43945986467|$e|$lean_checker|$(receipt_invocation lean)|7146f3dce43d6160bb2d902d56984ac5fbccf8880b7336127b4cb7568da5db1f|4 lean legs check with no axioms (Epistemic, Reversibility, Rollback, SingleActiveLog); the job refused its 8 fixtures (4 Rocq, 4 Lean) as expected; [check-recheck] OK|current||
lean|$u/43213340167|b5034884dbe945139d03b38fe80cfdf1106a77e1|$lean_checker|$lean_t1|2b89f7de00ec28537f8c1dd66d771048a3ee649183ed12c04f1d51b4246d37a8|$lean_said_t1|superseded|T-1's leg only, under the gate before hyperpolymath/januskey#180; replaced by the run at #180's merge|
lean|$u/42646851111|0834ce73717a1faafa23fc1df55d85d54ee1e2b7|$lean_checker|$lean_t1|2b89f7de00ec28537f8c1dd66d771048a3ee649183ed12c04f1d51b4246d37a8|$lean_said_t1|superseded|the first run on main (hyperpolymath/januskey#171); replaced by the run at the receipt's source commit|
ROWS
}

# ── Run scopes ────────────────────────────────────────────────────────────────
# A superseded run that did not check every claim with its leg is listed
# under the claims it checked only: the first Agda run predates
# Epistemic.agda (E-1m), and the first Rocq and Lean runs re-proved T-1
# alone (Rollback.v, Rollback.lean).
receipt_run_scope() {
  local u=https://github.com/hyperpolymath/januskey/actions/runs
  cat <<ROWS
agda|$u/40867211509|G-1 T-1 T-2 T-3
rocq|$u/43213340167|T-1
rocq|$u/42646851111|T-1
lean|$u/43213340167|T-1
lean|$u/42646851111|T-1
ROWS
}

receipt_echoes() { :; }
receipt_header() {
  cat <<'EOF'
;; SPDX-License-Identifier: CC-BY-SA-4.0
;; SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
;; The proof receipt of januskey (PROOF-POLICY §8.5.1).
;; GENERATED by scripts/proof-receipt.sh with the shared standards generator:
;; regenerate with --write, check with --check.  Each (receipt …) is a row of
;; the run ledger in that script, copied from the CI log of the run; every
;; digest is computed from git objects.  The grade is derived, and the pinned
;; reader refuses a :pwg its legs do not support.  Human record: PROOF-NEEDS.adoc.
EOF
}
# The prover's four universe-level primitives, each bound by its BUILTIN
# pragma; check-agda.sh checks them by name and binding on every run.
receipt_trusted_base() {
  cat <<'EOF'
Agda.Primitive|Level|BUILTIN LEVEL
Agda.Primitive|lzero|BUILTIN LEVELZERO
Agda.Primitive|lsuc|BUILTIN LEVELSUC
Agda.Primitive|_⊔_|BUILTIN LEVELMAX
EOF
}
receipt_invocation() {
  case "$1" in
    agda) printf '%s\n' "$agda_flags verification/proofs/agda/{Reversibility,TransactionAtomicity,Epistemic,SingleActiveLog}.agda (scripts/check-agda.sh)" ;;
    rocq) printf '%s\n' 'coqc -q -Q . JanusKey <leg>.v for each of Epistemic, Reversibility, Rollback, SingleActiveLog (copies of verification/proofs/rocq/*.v; scripts/check-recheck.sh)' ;;
    lean) printf '%s\n' 'lean -o <leg>.olean verification/proofs/lean/<leg>.lean for each of Epistemic, Reversibility, Rollback, SingleActiveLog (scripts/check-recheck.sh)' ;;
    *) receipt_die "unknown prover: $1" ;;
  esac
}
# §8.5.1 input sets, from the tree.
#  agda: the modules of verification/proofs/agda (all four are graded roots;
#        check-agda.sh resolves the rest of the cone to the sibling or the
#        prover), and scripts/fetch-siblings.sh, which holds the sibling's
#        pin, so a repin changes this digest.  No libraries file is read.
#  rocq: the .v files of verification/proofs/rocq.
#  lean: the .lean files of verification/proofs/lean (no lakefile is used).
receipt_inputs() {
  case "$1" in
    agda) git ls-files -- 'verification/proofs/agda/*.agda'; printf '%s\n' scripts/fetch-siblings.sh ;;
    rocq) git ls-files -- 'verification/proofs/rocq/*.v' ;;
    lean) git ls-files -- 'verification/proofs/lean/*.lean' ;;
    *) receipt_die "unknown prover: $1" ;;
  esac
}
# The sibling's modules in the Agda cone, at the pin scripts/fetch-siblings.sh
# holds at home commit C (check-agda.sh prints the cone: 7 first-party
# modules, the four roots and these three).
receipt_imports() {
  local pin p
  [ "$1" = agda ] || return 0
  # shellcheck disable=SC2016 # a literal ${…} in the pattern
  pin="$(git show "$2:scripts/fetch-siblings.sh" \
    | sed -n 's/^RESOURCE_ALGEBRA_COMMIT="\${RESOURCE_ALGEBRA_COMMIT:-\([0-9a-f]\{40\}\)}".*/\1/p')"
  [ -n "$pin" ] || receipt_die "no sibling pin in scripts/fetch-siblings.sh at $2"
  for p in src/ResourceAlgebra/Core.agda src/ResourceAlgebra/Instances/Loss.agda src/ResourceAlgebra/Prelude.agda; do
    printf '%s|%s|%s|%s\n' "$sibling_repo" "$pin" "$sibling_dir" "$p"
  done
}
# The claim ids PROOF-NEEDS.adoc grades: the ids its PWG census names.
receipt_status_ids() {
  sed -n '/^\*PWG census, theory theorems/,/ungraded/p' PROOF-NEEDS.adoc \
    | grep -oE '\([A-Z]-[0-9]+[a-z]*(, [A-Z]-[0-9]+[a-z]*)*\)' | tr -d '()' | tr ',' '\n' | tr -d ' ' || true
}
# The claims whose Rocq and Lean legs docs/theory/PROOF-GRADES.adoc records as
# RECONCILED: its one leg record serves every graded claim, so each graded
# claim is listed when both leg rows of that record say RECONCILED.
receipt_recorded_legs() {
  local rec
  rec="$(awk '/^==== Leg records \(PROOF-POLICY §8.5\)/{f=1} f&&/^\|===$/{n++; if(n==2) exit} f' docs/theory/PROOF-GRADES.adoc)"
  if awk '/^\| Rocq$/{l="rocq"} /^\| Lean$/{l="lean"} l && /^\| `RECONCILED`$/{ok[l]=1} END{exit !(ok["rocq"] && ok["lean"])}' <<<"$rec"; then
    receipt_status_ids
  fi
}

. "$RECEIPT_STD/scripts/proof-receipt.sh"
receipt_main "$@"
