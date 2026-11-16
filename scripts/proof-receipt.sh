#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
#
# proof-receipt.sh: the proof receipt of this repository,
# verification/receipts/januskey_praxis.deed, in the shape of
# standards/3-practice/PROOF-POLICY.adoc §8.5.1 (profile proof-receipt), with
# the git checks of §8.5.3.
#
#   --write   write the receipt from the ledgers below and from git
#   --check   regenerate it and refuse any difference; fetch the pinned reader
#             and checker of the standard and check their sha256; run the
#             reader (which derives every grade and refuses an overstated one)
#             and the §8.5.3 git checks; then report whether the current
#             receipts still describe HEAD (a warning, not a failure: a
#             receipt describes its :source-commit, and a stale one is still
#             true of that commit)
#
# What is typed and what is derived.  Two lists are typed in this file: the
# claim list (which legs each theory theorem has) and the run ledger (each row
# copied from the CI log of the run).  Everything else is a git fact computed
# here: the input set of each leg at a run's commit, the SHA-256 of every file
# in it, the §8.5.1 digest, and the files of the pinned sibling.  The grade is
# never typed by hand in the receipt's meaning: the receipt states it because
# the profile requires it, and the pinned reader refuses it unless the legs and
# receipts support it (§8.4, "grades are derived, not asserted").
#
# Needs: git, sha256sum, curl (for --check), bun (for --check).
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
export LC_ALL=C.UTF-8

out=verification/receipts/januskey_praxis.deed
repo="hyperpolymath/januskey"
# The commit the receipt describes: the main commit at which the current runs
# below were earned.  On the default branch, so no echo is needed (§8.5.2).
source_commit=e8deeaf7bc7b70500d57c282828512c8f8ba81f9
sibling_repo="hyperpolymath/resource-algebra"
sibling_dir=_build/dependencies/resource-algebra

# The pinned standard: the reader readProofReceipt (PROOF-POLICY §8.5.1), the
# reference digest script and the §8.5.3 git checker, all at one commit, each
# with its sha256.  They keep the standards layout under _build/standards,
# because the checker finds the reader and the digest script by their paths.
STANDARDS_COMMIT=513cba8d1f6153389685bf34375f5aeaf9bcd150
READER_SHA256=03634a2636f6aea202d777584978f8c7d99dc3c4fea7488d1300ba372b4386bd
DIGEST_SHA256=dfc6dde01a3a79df638eebe4cad0eae584d8f17280077e149174c0667e9508f4
CHECKER_SHA256=f2375747238464535d1b4a61f658f07cb2f8504c69bfb1812311c79429a4e23e
STANDARDS_RAW="https://raw.githubusercontent.com/metadatastician/standards/$STANDARDS_COMMIT"
std_dir=_build/standards

die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

# ── The claim list ────────────────────────────────────────────────────────────
# The theory theorems of PROOF-NEEDS.adoc (PROOF-POLICY §1): id | the provers
# with a leg.  A Rocq or Lean leg nobody has started is not listed at all: the
# profile's routes (re-proved, transported, waived, unavailable) each describe
# a leg that was tried, excused or found impossible, and docs/theory/
# PROOF-GRADES.adoc records those legs as "not attempted".  G-3 names the FFI
# bridge, so it is engineering under §1 and has no claim here.
claims() {
  cat <<'ROWS'
G-1|agda rocq lean
T-1|agda rocq lean
T-2|agda rocq lean
T-3|agda rocq lean
E-1m|agda rocq lean
ROWS
}

# ── The run ledger ────────────────────────────────────────────────────────────
# One row per checker run, copied from the log of the agda-proofs.yml job that
# printed it.  One run of a leg checks every theorem of that leg, so each row
# is a receipt of that leg for every claim that has the leg.
# prover | run | commit (40 hex) | checker | invocation | state | reason |
# claims the run did not cover (space-separated; the row is left out of them)
# A current row's inputs are computed at its commit; the axioms of every
# current row are empty, because each gate fails on any axiom (check-agda.sh:
# --safe and the cone scan; check-recheck.sh: Print Assumptions closed,
# #print axioms empty).
AGDA_CHECKER='Agda version 2.6.4.3 (Debian trixie agda-bin=2.6.4.3-1+b2, container debian:trixie-slim@sha256:a29215f6a35e51e22adffa17f89e9d2ef06214e64a2bad10d765c46aea49f11f)'
AGDA_FLAGS='agda --safe --without-K --cubical-compatible --no-libraries --ignore-interfaces --double-check -i verification/proofs/agda -i _build/dependencies/resource-algebra/src'
ROCQ_CHECKER='The Coq Proof Assistant, version 8.18.0 (Ubuntu 24.04 coq=8.18.0+dfsg-1build2)'
LEAN_CHECKER='Lean (version 4.35.0, x86_64-unknown-linux-gnu, commit eb6cafb7290e8bd5f2937522d8d804f58f02bafc, Release) (release tarball, sha256 7289597a44543246c9227b8cae501767e095ce17f6d448b95fc4a8dc97268f6d)'
runs() {
  local u=https://github.com/hyperpolymath/januskey/actions/runs
  cat <<ROWS
agda|$u/43945986467|e8deeaf7bc7b70500d57c282828512c8f8ba81f9|$AGDA_CHECKER|$AGDA_FLAGS verification/proofs/agda/{Reversibility,TransactionAtomicity,Epistemic,SingleActiveLog}.agda (scripts/check-agda.sh)|current|
agda|$u/43213340167|b5034884dbe945139d03b38fe80cfdf1106a77e1|$AGDA_CHECKER|$AGDA_FLAGS verification/proofs/agda/{Reversibility,TransactionAtomicity,Epistemic,SingleActiveLog}.agda (scripts/check-agda.sh)|superseded|the run at hyperpolymath/januskey#179; replaced by the run at #180's merge, the receipt's source commit
agda|$u/42736066407|e16543866fa4fd31a94f3c7faa1fe0671b083a1b|$AGDA_CHECKER|$AGDA_FLAGS verification/proofs/agda/{Reversibility,TransactionAtomicity,Epistemic,SingleActiveLog}.agda (scripts/check-agda.sh)|superseded|the first green run on main of the gate as changed by hyperpolymath/januskey#174; replaced by the run at the receipt's source commit
agda|$u/42076969251|23bc8c38160a25c37917bce69cf05d91899b07e5|$AGDA_CHECKER|$AGDA_FLAGS verification/proofs/agda/{Reversibility,TransactionAtomicity,Epistemic}.agda (scripts/check-agda.sh)|superseded|before the import-cone postulate scan (hyperpolymath/januskey#170) and before SingleActiveLog.agda (hyperpolymath/januskey#171)
agda|$u/41687715055|fc188c44906faedfe91bfd414be3fda01ff4caf4|$AGDA_CHECKER|$AGDA_FLAGS verification/proofs/agda/{Reversibility,TransactionAtomicity,Epistemic}.agda (scripts/check-agda.sh)|superseded|canonical flags and three modules; replaced by run 42076969251 at a later commit
agda|$u/40867211509|4afb710e86646658f667f10bd6274ee47416bfc7|$AGDA_CHECKER|agda --safe --without-K (before --cubical-compatible joined the gate)|superseded|before --cubical-compatible and before Epistemic.agda; it never covered E-1m, so it is listed for the other claims only|E-1m
rocq|$u/43945986467|e8deeaf7bc7b70500d57c282828512c8f8ba81f9|$ROCQ_CHECKER|coqc -q -Q . JanusKey <leg>.v for each of Epistemic, Reversibility, Rollback, SingleActiveLog (copies of verification/proofs/rocq/*.v; scripts/check-recheck.sh)|current|
rocq|$u/43213340167|b5034884dbe945139d03b38fe80cfdf1106a77e1|$ROCQ_CHECKER|coqc -q -Q . JanusKey Rollback.v (a copy of verification/proofs/rocq/Rollback.v; scripts/check-recheck.sh)|superseded|T-1's leg only, under the gate before hyperpolymath/januskey#180; replaced by the run at #180's merge|G-1 T-2 T-3 E-1m
rocq|$u/42646851111|0834ce73717a1faafa23fc1df55d85d54ee1e2b7|$ROCQ_CHECKER|coqc -q -Q . JanusKey Rollback.v (a copy of verification/proofs/rocq/Rollback.v; scripts/check-recheck.sh)|superseded|the first run on main (hyperpolymath/januskey#171); replaced by the run at the receipt's source commit|G-1 T-2 T-3 E-1m
lean|$u/43945986467|e8deeaf7bc7b70500d57c282828512c8f8ba81f9|$LEAN_CHECKER|lean -o <leg>.olean verification/proofs/lean/<leg>.lean for each of Epistemic, Reversibility, Rollback, SingleActiveLog (scripts/check-recheck.sh)|current|
lean|$u/43213340167|b5034884dbe945139d03b38fe80cfdf1106a77e1|$LEAN_CHECKER|lean -o Rollback.olean verification/proofs/lean/Rollback.lean (scripts/check-recheck.sh)|superseded|T-1's leg only, under the gate before hyperpolymath/januskey#180; replaced by the run at #180's merge|G-1 T-2 T-3 E-1m
lean|$u/42646851111|0834ce73717a1faafa23fc1df55d85d54ee1e2b7|$LEAN_CHECKER|lean -o Rollback.olean verification/proofs/lean/Rollback.lean (scripts/check-recheck.sh)|superseded|the first run on main (hyperpolymath/januskey#171); replaced by the run at the receipt's source commit|G-1 T-2 T-3 E-1m
ROWS
}

# ── Input sets (§8.5.1) ───────────────────────────────────────────────────────
# The home files a leg's run reads at COMMIT, one path a line, sorted bytewise.
#  agda: the modules of verification/proofs/agda (all four are graded roots,
#        and check-agda.sh resolves the rest of the cone to the sibling or to
#        the prover), and scripts/fetch-siblings.sh, which holds the sibling's
#        pin, so a repin changes this digest (§8.5.1, imported repositories).
#        No .agda-lib or libraries file is read (--no-libraries).
#  rocq: the .v files of verification/proofs/rocq.
#  lean: the .lean files of verification/proofs/lean (no lakefile is used).
inputs() {
  local prover="$1" commit="$2"
  {
    case "$prover" in
      agda) git ls-tree --name-only "$commit" -- verification/proofs/agda/ | grep -E '\.agda$'
            printf '%s\n' scripts/fetch-siblings.sh ;;
      rocq) git ls-tree --name-only "$commit" -- verification/proofs/rocq/ | grep -E '\.v$' ;;
      lean) git ls-tree --name-only "$commit" -- verification/proofs/lean/ | grep -E '\.lean$' ;;
      *) die "unknown prover: $prover" ;;
    esac
  } | LC_ALL=C sort
}
# The sibling's pin at COMMIT, read from the script that holds it.
sibling_pin() {
  # shellcheck disable=SC2016 # a literal ${…} in the pattern
  git show "$1:scripts/fetch-siblings.sh" \
    | sed -n 's/^RESOURCE_ALGEBRA_COMMIT="\${RESOURCE_ALGEBRA_COMMIT:-\([0-9a-f]\{40\}\)}".*/\1/p'
}
# The sibling's modules in the Agda cone (check-agda.sh prints the cone size:
# 7 first-party modules, the four roots and these three).
sibling_files() {
  printf '%s\n' src/ResourceAlgebra/Core.agda src/ResourceAlgebra/Instances/Loss.agda \
    src/ResourceAlgebra/Prelude.agda | LC_ALL=C sort
}
# "<sha256>  <path>" for each path on stdin, from the git blob at COMMIT in DIR.
manifest() {
  local dir="$1" commit="$2" p
  while IFS= read -r p; do
    git -C "$dir" cat-file -e "$commit:$p" 2>/dev/null || die "$p is not a file at $commit in $dir"
    printf '%s  %s\n' "$(git -C "$dir" cat-file blob "$commit:$p" | sha256sum | cut -d' ' -f1)" "$p"
  done
}
digest_of() { printf '%s' "$1" | sha256sum | cut -d' ' -f1; }

# ── Emit ──────────────────────────────────────────────────────────────────────
emit_files() {   # indent, manifest text
  local ind="$1" sha p
  while read -r sha p; do
    printf '%s(file :path "%s" :sha256 "%s")\n' "$ind" "$p" "$sha"
  done <<<"$2"
}
emit_receipts() {  # prover claim
  local prover="$1" claim="$2" p run commit checker inv state reason notfor m pin im
  while IFS='|' read -r p run commit checker inv state reason notfor; do
    [ "$p" = "$prover" ] || continue
    case " $notfor " in *" $claim "*) continue ;; esac
    printf '      (receipt :run "%s"\n' "$run"
    printf '               :commit "%s"\n' "$commit"
    printf '               :checker "%s"\n' "$checker"
    printf '               :invocation "%s"\n' "$inv"
    if [ "$state" = current ]; then
      m="$(inputs "$prover" "$commit" | manifest . "$commit")"
      printf '               :inputs-sha256 "%s"\n' "$(digest_of "$m"$'\n')"
      printf '               :state current\n'
      printf '               (inputs-manifest\n'
      emit_files '                 ' "$m"
      printf '               )\n'
      if [ "$prover" = agda ]; then
        pin="$(sibling_pin "$commit")"
        [ -n "$pin" ] || die "no sibling pin in scripts/fetch-siblings.sh at $commit"
        git -C "$sibling_dir" cat-file -e "$pin^{commit}" 2>/dev/null \
          || die "$sibling_repo $pin is not in $sibling_dir; run scripts/fetch-siblings.sh"
        im="$(sibling_files | manifest "$sibling_dir" "$pin")"
        printf '               (import :repository "%s" :commit "%s" :inputs-sha256 "%s"\n' \
          "$sibling_repo" "$pin" "$(digest_of "$im"$'\n')"
        emit_files '                 ' "$im"
        printf '               )\n'
      fi
      printf '               (axioms))\n'
    else
      printf '               :state %s\n' "$state"
      printf '               :reason "%s")\n' "$reason"
    fi
  done < <(runs)
}
emit() {
  cat <<EOF
;; SPDX-License-Identifier: CC-BY-SA-4.0
;; SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
;; The proof receipt of januskey (PROOF-POLICY §8.5.1).
;; GENERATED by scripts/proof-receipt.sh: regenerate with --write, check with --check.
;; Each (receipt …) is a row of the run ledger in that script, copied from the CI
;; log of the run; every digest is computed from git objects.  The grade is
;; derived by the pinned reader, which refuses a :pwg its legs do not support:
;;   bun deed_lint.js --proof-receipt-json verification/receipts/januskey_praxis.deed
;; with the reader at standards $STANDARDS_COMMIT.
;; A Rocq or Lean leg nobody has started is absent, not "unavailable"
;; (docs/theory/PROOF-GRADES.adoc).  Human record: PROOF-NEEDS.adoc.
(praxis-deed
  :schema-version "1.0.0"
  :canonical-name "januskey-proof-receipt"
  :profile proof-receipt
  :repository "$repo"
  :source-commit "$source_commit"
  (trusted-base
    (primitive :prover agda :module "Agda.Primitive" :name "Level" :binding "BUILTIN LEVEL")
    (primitive :prover agda :module "Agda.Primitive" :name "lzero" :binding "BUILTIN LEVELZERO")
    (primitive :prover agda :module "Agda.Primitive" :name "lsuc" :binding "BUILTIN LEVELSUC")
    (primitive :prover agda :module "Agda.Primitive" :name "_⊔_" :binding "BUILTIN LEVELMAX"))
EOF
  local id provers pwg n
  while IFS='|' read -r id provers; do
    n=$(wc -w <<<"$provers")
    # The grade these legs support under §8.2: the reader recomputes it and
    # refuses the receipt if this line and the legs disagree.
    case "$n" in
      1) pwg=PWG-3 ;;
      2) pwg=PWG-2i ;;
      3) pwg=PWG-1 ;;   # both legs, every current receipt at :source-commit
    esac
    printf '  (claim :id "%s" :pwg %s :foundation-diverse #f\n' "$id" "$pwg"
    printf '    (leg :prover agda :route source :token proven :state AGDA-VERIFIED\n'
    emit_receipts agda "$id" | sed '$ s/$/)/'
    for p in $provers; do
      [ "$p" = agda ] && continue
      printf '    (leg :prover %s :route re-proved :token proven :state RECONCILED\n' "$p"
      emit_receipts "$p" "$id" | sed '$ s/$/)/'
    done
    printf '  )\n'
  done < <(claims)
  printf ')\n'
}

# ── The pinned standard ───────────────────────────────────────────────────────
fetch_standard() {
  local rel sha f
  for pair in "1-formats/deed/tools/deed_lint.js:$READER_SHA256" \
              "scripts/proof-inputs-sha256.sh:$DIGEST_SHA256" \
              "scripts/check-proof-receipt.sh:$CHECKER_SHA256"; do
    rel="${pair%%:*}"; sha="${pair#*:}"; f="$std_dir/$rel"
    if [ ! -f "$f" ]; then
      mkdir -p "$(dirname "$f")"
      curl --proto '=https' --tlsv1.2 -sSfL -o "$f" "$STANDARDS_RAW/$rel"
    fi
    printf '%s  %s\n' "$sha" "$f" | sha256sum -c --quiet - || die "$f does not match its pinned sha256"
  done
}

mode="${1:---check}"
case "$mode" in
  --write)
    emit > "$out"
    echo "WROTE $out (source-commit $source_commit)" ;;
  --check)
    tmp="$(mktemp)"; trap 'rm -f "$tmp"' EXIT
    emit > "$tmp"
    diff -u "$out" "$tmp" || die "the receipt is not what the ledgers and git give; regenerate with --write"
    fetch_standard
    command -v bun >/dev/null || die "bun is needed to run the pinned reader"
    bun "$std_dir/1-formats/deed/tools/deed_lint.js" "$out"
    bun "$std_dir/1-formats/deed/tools/deed_lint.js" --proof-receipt-json "$out" >"$tmp" \
      || die "the pinned reader refuses the receipt"
    # shellcheck disable=SC2016 # JavaScript, not shell
    bun -e 'const r = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
            for (const c of r.claims) console.log(`derived: ${c.id} ${c.pwg}`);' "$tmp"
    # §8.5.3: the commits hold what the receipt says.  The home clone must be
    # full; the checker refuses a shallow one.
    bash "$std_dir/scripts/check-proof-receipt.sh" --branch "${RECEIPT_BRANCH:-origin/main}" \
      --import "$sibling_repo=$sibling_dir" "$out" . || die "the git checks of §8.5.3 failed"
    # Freshness: does each current receipt's input digest still hold at HEAD?
    stale=0
    while IFS='|' read -r p run commit checker inv state reason _; do
      [ "$state" = current ] || continue
      now="$(digest_of "$(inputs "$p" HEAD | manifest . HEAD)"$'\n')"
      was="$(digest_of "$(inputs "$p" "$commit" | manifest . "$commit")"$'\n')"
      if [ "$now" != "$was" ]; then
        stale=1
        echo "::warning::the $p inputs changed after ${commit:0:12}: the receipt still describes its source commit, not HEAD; record a run of HEAD's main commit to bring it forward"
      fi
    done < <(runs)
    [ "$stale" -eq 1 ] || echo "fresh: every current receipt's inputs are unchanged at HEAD"
    echo "PASS: $out" ;;
  *) echo "usage: ${0##*/} [--write|--check]" >&2; exit 2 ;;
esac
