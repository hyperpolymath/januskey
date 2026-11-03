# Paste-ready prompt — `hyperpolymath/resource-algebra`

Scope: **this repo only.** Do not edit `januskey`. Latest stable Agda (2.8.0.2
at the time of writing) unless you have a pin you would rather keep.

---

Context: I am working in `hyperpolymath/januskey`, which consumes this repo as
its one proof dependency, pinned at `cf9235c532712a783a404c359a1055f40134794a`
(`scripts/fetch-siblings.sh`, one commit per re-pin). The Agda gate imports
`ResourceAlgebra.Prelude`, `.Core` and `.Instances.Loss` and proves over them;
the modules are `verification/proofs/agda/{Reversibility,TransactionAtomicity}.agda`
and they type-check against that pin today (Agda 2.6.4.3, `--safe --without-K`).

Two things, both small, both measurably *your* side of the boundary:

1. **The homomorphism obligation is the one blocking my ledger.** Your tree
   lists homomorphisms as OPEN; my row G-3 is exactly that statement for the FFI
   bridge — that wrapping a grade-𝟙 operation preserves grade 𝟙 across the
   Zig/Rust boundary. I am not asking you to prove my bridge. I am asking
   whether you will state the obligation in your vocabulary so that I can
   consume it by pin, the way I consume `Loss`, instead of restating a
   homomorphism-shaped claim in my own tree, which would be the failure mode
   your own docs warn about (a dependent re-declaring what its sibling owns).

2. **`--cubical-compatible` in your module pragmas.** Every module here opens
   with `{-# OPTIONS --safe --without-K #-}`. The estate's PROOF-POLICY §2.1
   canonical invocation is `agda --safe --without-K --cubical-compatible`, and I
   now run it on my side. It works — Agda accepts a `--without-K` import and
   warns instead of failing — but 14 of my warnings come from this repo, the
   first being `≤-trans` in `ResourceAlgebra.Prelude` (indexed match on `suc`,
   so it will not compute under transports). If you adopt `--cubical-compatible`
   in your own pragmas, those warnings go away for every dependent, not just me.
   If you would rather not, say so and I will keep the warnings recorded in my
   receipt instead of treating them as a defect of yours.

Evidence you can check without trusting me: my pinned clone is your `cf9235c`;
the warning text and counts are in `docs/theory/PROOF-GRADES.adoc` and
`docs/estate/environment-toolchains.md` in `januskey`, and the gate is
`scripts/check-agda.sh`.

Boundary: I have read only `src/ResourceAlgebra/{Prelude,Core,Instances/Loss}`,
`docs/AFFIRMATION.adoc` and `PROOF-STATUS.adoc`, and only as a consumer. If the
homomorphism item is already stated somewhere I did not read, point at it and
ignore item 1.
