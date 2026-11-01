# Paste-ready prompt — `hyperpolymath/the-metadatastician`

Scope: **this repo only.** Do not edit `januskey`. Latest stable versions only
(Rust latest stable via rustup; Node/Python whatever your tree already pins).

---

Context: I work in `hyperpolymath/januskey`, a reversible-operation tool that
emits A2ML manifests (`crates/reversible-core/src/manifest.rs`) and maintains a
claims ledger (`PROOF-NEEDS.adoc`) plus a proof-grade page graded against
`mathesis` and the estate's PROOF-POLICY.

I am writing because your README promises "auditable provenance trails",
"grounded documentation claims" and "release-time evidence checks", and januskey
now has exactly those artefacts in a machine-readable shape: 46 claim rows, five
statuses, per-row artefact and checker command, and a receipt naming commit,
prover version and flags. Two questions, no dependency claimed either way:

1. **Do you ingest A2ML manifests today, and in what shape?** januskey's
   `ManifestEmitter` has never been parsed by anything outside its own tests —
   my ledger records that as `DESIGNED`, not as a working integration. If you
   already read A2ML, I would like to know which fields you require, so that the
   row can be restated at the strength of your parser rather than my emitter.
2. **Would a claims ledger be in scope for your evidence checks?** Concretely:
   would you want (a) the ledger as data, (b) the grade page as a narrative, or
   (c) nothing until the manifest path is settled? I would rather record "not in
   scope" than a designed-but-unbuilt integration.

Evidence: januskey's `PROOF-NEEDS.adoc`, `docs/theory/PROOF-GRADES.adoc`,
`crates/reversible-core/src/manifest.rs`, and ledger rows V-1..V-3 which record
downstream consumption as `DESIGNED` with the change condition (a dependency
edge in the consumer's manifest plus a green test there).

Boundary: I have read only your `README.adoc` and file listing; I have not read
your implementation, so I make no claim about what it does. If any of the above
misreads your scope, correct it — that is more useful to me than a yes.
