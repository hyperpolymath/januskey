# Paste-ready prompt — `hyperpolymath/mathesis` (`estate-mathematical-architecture`)

Scope: **this repo only.** Do not edit `januskey`. Latest stable versions only
(Agda 2.8.0.2; Rocq/Lean only if §2's transport work is in scope).

---

Context: I am grading the proof claims of `hyperpolymath/januskey` under §5 and
§6 of `estate-mathematical-architecture`, at HEAD. Four of my eleven `PROVEN`
ledger rows have an Agda artefact and type-check; nothing has been translated.
By §5's sentence they are Grade 3. Three questions about the *receipt*, because
§6 asks for commit hashes and §2.3 of the estate's PROOF-POLICY puts executions
in CI:

1. **Does a Grade 3 receipt have to come from CI?** §5 says a proof "type-checks
   locally in Agda". Taken literally, a local type-check is the grade, and CI is
   how the estate records it. I have a green CI run at commit `4afb710`
   (`agda-proofs.yml`, Agda 2.6.4.3, pinned sibling `cf9235c`), and separately a
   local run of the same modules under the same prover version after I changed
   the gate's flag set to PROOF-POLICY §2.1's canonical form. Which is the
   receipt for the current bytes — the CI run for the old flags, or the local
   run for the new ones, or neither until CI runs again? I am currently claiming
   "AGDA-VERIFIED, receipt pending re-run" and would rather use your rule.

2. **How are non-axiom caveats recorded?** My modules type-check with
   `-W[no]UnsupportedIndexedMatch` warnings: clauses that will not compute
   under cubical transports. They are not axioms and not holes, so they do not
   belong in the axiom list, but they are exactly the kind of caveat a consumer
   would want before citing a Grade 3 result as if it were fully computational.
   Does §6's axiom tracking have a slot for this, or is a separate line the
   convention?

3. **What happens to a grade when the checker changes but the artefact does
   not?** I added `--cubical-compatible` to the gate; the two `.agda` files are
   byte-identical before and after. §6 says a grade that stops applying is
   marked `Superseded` or `Regressed` — but here nothing was re-proved and
   nothing stopped holding; only the invocation was tightened. Is that a new
   receipt with the old one superseded, or the same grade with a re-earned
   receipt? I have written it as the latter and flagged the difference.

What I can offer: a real ledger with 47 rows in five statuses, five Grade 3 rows
with an artefact-per-row discipline and expected-rejection fixtures, a page
(`docs/theory/PROOF-GRADES.adoc`) that maps every row onto §1 and §5 and records
what is *not* graded — the seven Idris2 ABI rows, which PROOF-POLICY §1 classes
as engineering rather than theory. If any of that mapping is wrong by your
reading, say which and I will change the page rather than the grade.

Boundary: I treat §5's sentences as the definition and §6's bullets as the
reporting rules; I have not inferred anything about Grade 2i/2ii beyond the
text, and I claim no translation work.
