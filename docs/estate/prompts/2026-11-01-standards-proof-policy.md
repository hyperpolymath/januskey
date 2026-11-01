# Paste-ready prompt — `hyperpolymath/standards` (`3-practice/PROOF-POLICY.adoc`)

Scope: **this repo only.** Do not edit `januskey`. Pin everything to latest
stable (Agda 2.8.0.2, Idris2 v0.8.0, Zig 0.15.1, Rust latest stable via rustup).

---

Context: I am working in `hyperpolymath/januskey` (branch off `main` @
`4afb710`). I implemented PROOF-POLICY §2.1 in a real CI gate and hit four
places where the policy does not say what to do. I am not asking for a
relaxation; I am asking for the rule, because I had to invent one and I would
rather cite yours.

What I measured, with Agda 2.6.4.3 (the version the gate pins), so you can
check each claim rather than take it:

1. **The canonical invocation runs into the "infectious flag" case.** §2.1 says
   `agda --safe --without-K --cubical-compatible <module>`. `--cubical-compatible`
   is documented as requiring every directly imported module to use it too. My
   modules import a pinned sibling (`resource-algebra@cf9235c`) whose every
   module declares `{-# OPTIONS --safe --without-K #-}`. Empirically Agda
   *accepts* this: exit 0, with 14 and 16 `-W[no]UnsupportedIndexedMatch`
   warnings respectively, naming clauses (the sibling's `≤-trans`, my
   `obliterate-absorbs`) that will not compute under transports. So the policy's
   canonical form is achievable against a `--without-K` dependency. **Question:
   is that the intended reading, and should the policy say so?** As written, a
   reader like me inferred the opposite and shipped the weaker invocation for
   months.

2. **Warnings are not covered.** §2.2 lists statuses for *executions*; nothing
   says whether a proof that type-checks with warnings may be `AGDA-VERIFIED`,
   whether the warning classes must be recorded, or whether `-WnoUnsupportedIndexedMatch`
   is an acceptable silencing. I chose: keep them visible, record the classes
   and counts in the receipt, and treat them as statements about computation
   under transport rather than about the proposition. **Question: keep, ban, or
   require per-class justification?**

3. **Two prover versions in one repository's CI.** One workflow pins 2.6.4.3
   (the real receipt); another installs 2.6.3 from a different base image
   because it also needs Idris2. §2.1 says record the version per commit but not
   which of two recorded versions is the receipt. **Question: should a repo be
   required to have exactly one prover version per prover, and should a checker
   run under a different version be marked as a consistency check rather than a
   receipt?**

4. **Provenance of the prover binary.** A sandbox I work in can install Agda
   only as a PyPI wheel (`agda==2.6.4.3.post1`, reporting `2.6.4.3-56f5f56`),
   not from a signed distro archive. §2.1 says pin and record; §2.3 says CI
   receipts establish execution. **Question: is a local run with a wheel-built
   prover recordable as anything, and if so under what label — `reproduction`?**
   I have used "reproduction, not a receipt" and would rather use your word.

What I can offer back: the gate itself (`scripts/check-agda.sh`), the
expected-rejection fixtures whose refusal message is asserted (four Agda files
refused with the exact text they exist for), and the recorded receipt with
commit hash, prover version, flags and sibling pin — in
`docs/theory/PROOF-GRADES.adoc`. If you change the policy, that page is a
place where the change is *used*.

Boundary: I have implemented §1's mapping as written (Rust → Creusot, theory →
Agda, ABI → Idris2) and make no claim to know your intent beyond the text.
