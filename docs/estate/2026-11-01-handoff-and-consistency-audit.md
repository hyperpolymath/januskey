# JanusKey — estate handoff and consistency audit

**Date:** 2026-11-01
**Branch:** `arena/ed943c46-januskey` (base = `main` @ `4afb710`, 2026-10-29, "chore(deps): Bump the actions group with 2 updates (#165)")
**Subject:** re-recon of `hyperpolymath/januskey`, division of concern between this repo and the other estate chats, a 16-item consistency audit against the proven invariants, and one paste-ready prompt per relevant sibling repo.

---

## 0. What this audit can and cannot stand on

Stated first, because the whole point of the invariant list is that claims get checked against source:

| Fact | Consequence |
|---|---|
| The checkout is **shallow (depth 1)** — `git log` returns exactly one commit. | No commit archaeology. "Recent work" cannot be reconstructed from history. |
| **`cargo`, `rustc`, `idris2`, `agda` are not installed**, and crates.io is not reachable from this sandbox. | I could **not re-run any gate**. Where I write PROVEN below I mean *"the ledger names a theorem and the theorem text exists in the named file, and the gate script that would check it exists"* — not *"I re-ran it and it passed"*. |
| Only `januskey` is checked out. No sibling repo is present. | Every statement I make about another repo is either (a) the user-supplied invariant, or (b) an explicit quotation of what **januskey's source asserts about them**. Nothing about a sibling's internals is derived from its name. |
| This branch contains **no agent commits** — `HEAD` is `main`'s tip. | There is no prior "recent output" of mine to audit. The audit below is therefore an audit of (i) the model I am about to work from and (ii) the repo's own published claims as they stand today. I have flagged that distinction wherever it matters. |

**Held disconfirmed-able:** every claim in §3 has a file, line or section behind it, listed inline.

---

## 1. Re-recon in one page

### 1.1 What the code says januskey is

`README.adoc` title is *"JanusKey — Reversible File Operations"*; `crates/reversible-core/src/lib.rs` defines the identity as a `ReversibleExecutor` trait (`execute` / `undo` / `generate_manifest`); the binary is `jk` with subcommands `init, delete, modify, move, copy, rename, obliterate, undo, begin, commit, rollback, preview, history, status, gc` (`crates/januskey-cli/src/main.rs:54-160`). Reversibility is the product; content-addressed storage (`SHA-256`, `crates/reversible-core/src/content_store.rs:17-32`) plus an append-only operation log is the mechanism.

### 1.2 Actual module map (three crates, not two)

```
crates/reversible-core/   content_store · metadata · transaction · manifest · error · lib(trait)
crates/januskey-cli/      operations · delta · attestation · obliteration · keys · keys_cli · main
crates/dashboard-check/   claims.rs (claims-ledger parser + runner) · main.rs (dashboard reconciliation)
src/abi/JanusKey/ABI/     Types · Layout · Foreign · Proofs   (Idris2 0.8.0, one tree, .ipkg beside it)
verification/proofs/agda/ Reversibility · TransactionAtomicity  (Agda 2.6.4.3, --safe --without-K)
ffi/zig/                  jk_* C-ABI surface + januskey.h
tests/reject-agda/        4 fixtures   tests/reject-idris/ 3 fixtures   (negative controls)
```

The proof split is a **kernel ruling dated 2026-10-12**: Agda is the kernel, Idris2 the runtime/ABI layer (`verification/proofs/agda/Reversibility.agda:14-18`, `PROOF-NEEDS.adoc §Recommended Prover`). The only external proof dependency is **`resource-algebra`, pinned at `cf9235c`** (`scripts/fetch-siblings.sh:9`), consumed through `ResourceAlgebra.Instances.Loss`.

### 1.3 The instruments that make this repo trustworthy — and are its best export

1. **A claims ledger** (`PROOF-NEEDS.adoc §Claims ledger`): one row per claim, one of five statuses `PROVEN | TESTED | ASSUMED | DESIGNED | OPEN`, each with an artefact and the command that discharges it.
2. **`dashboard-check`** (`crates/dashboard-check/src/claims.rs`): runs every PROVEN/TESTED command, and fails the build if the counts printed on README/EXPLAINME/TOPOLOGY/READINESS disagree with what it measured. Its own header names the motive: *"the estate's #1 recurring defect is 'dashboards that lie'"* (`main.rs:7`).
3. **Negative controls with expect-text matching** (`scripts/check-agda.sh:29-44`): a fixture must be refused *with the specific error it exists for*; a fixture refused for an unrelated reason fails the gate. This is the pattern the estate should steal.
4. **Removal discipline**: lemmas that were tautologies are deleted with the reason recorded in place, not left as decoration (`PROOF-NEEDS.adoc §Removed` — 18 entries, e.g. `identityIsCNO` "(`s = s`)", `hashOneWay` "(`Void -> Void`)").

### 1.4 True state (per the repo's own reconciled dashboards)

`STATE.a2ml`: completion 60 %, CRG grade **D**, phase *"Core tests complete; 2026-04..06 wave was CI/governance/license hardening, not feature work; API stability review still pending"*. READINESS: **D — Alpha, Unstable**, 135 tests + 5 Criterion groups + **11 PROVEN** ledger rows. The eleven are ABI/model lemmas (A-1, A-2, C-1..C-5, G-1, T-1, T-2, T-3); **none of the six security obligations is proved**, and the row *"Every operation is reversible (MPR)… " is `OPEN`*.

That is the single most important sentence in the repo: **the headline claim is still a ledger row marked OPEN.**

---

## 2. Division of concern — mine here, theirs elsewhere

### 2.1 Belongs to this repo (my work)

| Concern | Where | Why it is mine |
|---|---|---|
| Reversibility semantics (`execute ∘ undo ≡ CNO`) in the Rust | `crates/reversible-core`, `crates/januskey-cli/operations.rs`, `delta.rs` | Only januskey can change its own executor |
| Metadata/transaction atomicity (T-5 done, **T-8 open**: `metadata.rs:288-293` was a plain `fs::write` when this was measured; it now calls `write_atomic`, unbuilt — see the Addendum) | `reversible-core` | Internal invariant, and the concrete asymmetry I would fix next |
| Attestation chain, now a real HMAC-SHA256 (`attestation.rs:9,189-212`; STATE marks `homerolled-hmac` RESOLVED — `threat-model.a2ml` §primitives.mac is stale on this point) | `januskey-cli/attestation.rs` | Internal |
| Obliteration / CAS scrubbing (TESTED end-to-end, `tests/obliteration_cas_test.rs`) | `januskey-cli/obliteration.rs` | Internal |
| Key store (AES-256-GCM + Argon2id; Ed25519/X25519 refused at the CLI, `keys_cli.rs:282-291`) | `januskey-cli/keys.rs`, `keys_cli.rs` | Internal, and *scope-adjacent* — see audit §7 |
| Idris2 ABI + Agda kernel + the claims ledger + `dashboard-check` | `src/abi`, `verification/`, `crates/dashboard-check` | Internal |
| Doc-surface drift (§4) | README, EXPLAINME, TOPOLOGY, STATE, `0-AI-MANIFEST`, `docs/` | Internal, and currently the largest defect mass |

### 2.2 Belongs to other chats (I can state the interaction, not the truth of their internals)

| Repo | Relation to januskey, as recorded **here** | Whose call it is |
|---|---|---|
| `resource-algebra` | **Hard dependency.** Pinned `cf9235c`; supplies the `Loss` grade + `IsSeqMonoid` structure the Agda proofs are stated in; its homomorphisms are listed OPEN, which is why januskey's G-3 is OPEN | theirs |
| `absolute-zero` | Lineage only: *"maa-framework → absolute-zero (CNO theory) → januskey"* (`Types.idr:7`, `lib.rs:12`, `ARCHITECTURE.adoc §Lineage`). januskey **re-proves** CNO composition locally (`Proofs.idr::seqCNO`) instead of importing it | theirs |
| `ochrance` | Intended consumer of `ManifestEmitter` A2ML. **Hash mismatch is unresolved**: `ContentHash` is hard-coded `sha256:` (`content_store.rs:17-32`) and its doc comment claims correspondence to *"ochrance's `Hash` type with `algorithm = SHA256`"*, while ARCHITECTURE §Deferred says *"ochrance uses BLAKE3 height-indexed trees"* | shared; theirs to confirm |
| `valence-shell` | **Downstream.** ARCHITECTURE §Phase 2 (NEXT): shell depends on `reversible-core`, `ContentStore` into `ShellState`, replace inline `undo_data: Option<Vec<u8>>`, replace `verification.rs` stubs | theirs, once we publish the trait |
| `VeriSimDB` (`nextgen-databases`) | One declaration, no code: `.machine_readable/integrations/verisimdb.a2ml`, `type = "data-feed"`, destination `verisimdb-data/feeds/`, **`format = "hexad"`** | theirs |
| `hypatia` | Scan config only; `NEUROSYM.a2ml` sets `report-format = "logtalk"` | theirs |
| `boj-server` | A `januskey-mcp` cartridge is **blocked**: threat model is `draft-pending-human-review` since 2026-04-19 (`STATE.a2ml §blockers`) | owner sign-off + boj-server |
| `panic-attacker` | `just assail` shell-out (`Justfile:135-136`); 241 `unwrap()` sites currently | theirs (tooling) |
| `aletheia` | Third downstream, **deferred**: *"Too early (Phase 0 research, no operation types yet)"* | theirs |
| `tropical-types`, `epistemic-types`, `choreographic-types`, `echo-types`, `residual-evidence-types`, `JtV`, `ephapax`, `ECHIDNA`, `proof-burrower` | **No dependency, no pin, no code.** Only three incidental prose/comment mentions (see audit §5, §8, §12, §14) | theirs |

### 2.3 Boundary items that need an agreed answer from both sides

1. **Hash function at the manifest boundary** (SHA-256 here vs BLAKE3 alleged there).
2. **ABI struct layout** — `ObliterationProof` is 112 bytes, `ContentHash` 32, `KeyId` 16, 8-byte aligned (`Layout.idr`, PROVEN C-5). Any sibling embedding these must agree, and `scripts/check-abi-layout.sh` is the only guard.
3. **Log format** — `OperationLog` JSON is januskey's own; nothing else may read it without a version.

---

## 3. Consistency audit

Format: **STATUS** / one-line evidence / action.

### Structural invariants

**1. Central substrate (VeriSimDB octad, VCL, no point-to-point).**
**STATUS: ✅** — one flag.
*Evidence:* `grep` for `reqwest|hyper|TcpStream|UdpSocket|ureq|curl|http` across `crates/` returns **zero** hits; januskey contains no VCL implementation and no subsystem-to-subsystem protocol. Its only outbound artefact is a *string* (`ManifestEmitter`), plus a declared file-drop feed.
*Flag (not a contradiction):* the declared feed says `format = "hexad"` — six, not eight — and is not VCL-mediated. It is inert today (nothing writes it).
*Action:* either delete `.machine_readable/integrations/verisimdb.a2ml` as dead config, or restate it as an octad/VCL feed. Do not implement it as written.

**2. Echo↔Resource is E→R only; Echo is not a resource instance.**
**STATUS: ✅** (with a name-collision caveat).
*Evidence:* januskey has no Echo integration of any kind; `echo` occurs once, as *"the echo loss grade"* in `Reversibility.agda:6-7`, naming a grade in `resource-algebra`'s `Loss` instance. No conversion in either direction, so no bidirectional assumption could have crept in.
*Caveat:* I do **not** know that this "echo" and `echo-types` are the same thing. Treated as unresolved, not as an identity.
*Action:* none in code; ask echo-types (§4.9).

**3. No universal adapter (`hub_ceiling`).**
**STATUS: ✅**, one watch item.
*Evidence:* `ReversibleExecutor` is a *generic interface with per-backend implementations* (`lib.rs:52-72`), which is the shape a no-go theorem permits; it transduces nothing by itself, and `generate_manifest` is per-implementation. Not a universal adapter.
*Watch item:* the phrase *"the shared foundation that all three applications use"* (`lib.rs:15-17`) is a **code-sharing** claim that none of the three has yet exercised, and the A2ML manifest is exactly where a "one format fits all" assumption could form — indeed `ContentHash` already hard-codes `sha256:` while the deferred note says ochrance is BLAKE3.
*Action:* keep one executor per backend; make the hash algorithm an associated type before any second implementor appears; do not advertise manifest portability until §2.3(1) is settled.

**4. absolute-zero is a consumer of proof infrastructure with 61 undischarged axioms; OND-6 open by design.**
**STATUS: ✅.**
*Evidence:* absolute-zero appears only as lineage (`Types.idr:7-9`, `lib.rs:12-14`) and as a *local re-proof* — `Proofs.idr::seqCNO` proves CNO composition inside januskey's own model; nothing imports an absolute-zero theorem, and no januskey row cites an absolute-zero Qed count. External assumptions are held explicitly as ASSUMED `X-1` (hash one-wayness, physical erasure).
*Action:* keep it; never let an absolute-zero result become a januskey ledger receipt.

**5. K-CUT is OPEN; VCL-UT L8 blocked; choreographic-types is research-blocked.**
**STATUS: ✅.**
*Evidence:* `choreographic-types` appears twice, both as a **pattern citation** — `.github/workflows/agda-proofs.yml:3` ("Pattern: choreographic-types / residual-evidence-types (pinned Debian prover, sibling fetched by commit)") and `scripts/fetch-siblings.sh:4`. No code, no pin, no dependency, no deployability claim.
*Action:* reword those two comments to say "pattern, no dependency" so a future reader cannot mistake a citation for a link.

**6. Epistemic model is discrete-constructive; no continuous confidence; no bind/extract.**
**STATUS: ✅** — and this is the strongest match in the audit.
*Evidence:* januskey's own model is discrete too: `data Knows : Agent -> Type -> Type` with three constructors, no numeric confidence anywhere in `Types.idr:378-400`, and `KeyState` is a six-constructor sum (`keys.rs:110-117`). The ledger states plainly that `Knows Adversary a` being uninhabited is *"a modelling choice, not a security proof"* and that E-1 is **OPEN**. There is no monadic bind and no comonadic extract — it is an inductive family, not a modality.
*Action:* none; and do not ask epistemic-types for a 0–1 confidence model (§4.6).

**7. januskey is file-level reversibility, not key management; part of Valence Shell.**
**STATUS: ✅ on identity / ⚠️ on two particulars.**
*Evidence:* identity is reversibility (title, `lib.rs`, CLI surface). But:
(a) **`keys.rs` is 735 real lines** — AES-256-GCM wrap, Argon2id (64 MiB/3/4), rotation, revocation, obliteration, plus a second binary `jk-keys` (701 lines) and a 550-line `docs/security/KEY_LIFECYCLE.adoc`. "Not a key system" is true of its *purpose*, false of its *surface*. `PROOF-NEEDS.adoc §Priority` even says *"JanusKey handles cryptographic key management…"* — that framing is drift.
(b) **Direction of containment is inverted.** `ARCHITECTURE.adoc §Lineage` puts `valence-shell` **downstream** of januskey (`januskey → {ochrance, valence-shell, aletheia}`), with `reversible-core` as the shared library the shell will consume in Phase 2. januskey is a *sibling ancestor*, not a component of the shell.
*Action:* keep treating the work as file-level reversibility; correct the model to "upstream of valence-shell"; stop describing januskey as a key-management system in PROOF-NEEDS.

**8. JtV v2 is a separate research program; convergence monitored, not depended on.**
**STATUS: ✅.**
*Evidence:* zero references to JtV / ReversalToken / `reversaltoken` anywhere in the tree (case-insensitive, excluding `.git`).
*Action:* none.

**9. Ephapax legacy preservation is provably false; the `Admitted` is durable.**
**STATUS: ✅.**
*Evidence:* zero references to ephapax; nothing in `src/abi` or `verification/` attempts such a closure. The only `Admitted`-class markers in the tree are the *deliberately removed* tautologies listed in `PROOF-NEEDS.adoc §Removed`.
*Action:* none. Respect the negative result.

**10. VSH trust boundary is the extraction gap (Lean→Rust); vsh proves algorithmic, not thermodynamic, reversibility; RMO is an engineering primitive, not GDPR compliance.**
**STATUS: ⚠️ — my model is correct, but januskey's own docs contradict it in three places.**
*Evidence:* januskey has no Lean at all; its gap is Agda/Idris2 → Rust, and it is scrupulous about it — G-2 (Idris↔Agda correspondence) is `ASSUMED`, *"no mechanical link claimed"*, and X-1 keeps crypto/physical assumptions outside the theorem set. **But:**
1. `docs/wiki/theory/rmo-primitive.md:8` — *"…implements GDPR Article 17 'Right to Erasure' **with mathematical certainty**"*, with a "Theorem" whose proof is four informal steps and a `∎`.
2. `docs/security/THREAT_MODEL.adoc:287` — compliance table claims *"256-bit symmetric, **256-bit ECC**"*, while `keys.rs:66-76` says Ed25519/X25519 are *"Reserved / NOT IMPLEMENTED"* and `keys_cli.rs:282-291` refuses to generate them.
3. `docs/wiki/theory/maa-framework.md:193-199` maps GDPR Art. 17/5/30 onto RMO/RMR as though the mapping were established.
*Action (in this repo, next):* rewrite the three surfaces to engineering-primitive language; state explicitly that GDPR compliance is a *controller-level* determination and that RMO supplies (i) best-effort overwrite and (ii) a commitment record, neither of which is an erasure guarantee on SSD/CoW/journaled media (already correctly stated in README §"secure obliteration exception" and GLOSSARY — the wiki contradicts the README).

### Architectural consistency

**11. Logtalk was removed; `.lgt` files are dead; the live rule system is Elixir.**
**STATUS: ⚠️ (one live-looking reference).**
*Evidence:* zero `.lgt`/`.logtalk` files. But `.machine_readable/6a2/NEUROSYM.a2ml` still carries `report-format = "logtalk"` — a machine-readable config value naming a removed system, in the file `0-AI-MANIFEST.a2ml` declares authoritative.
*Action:* confirm with `hypatia` whether `logtalk` is still a valid report-format enum; if not, change it to the current value. Do not treat Logtalk as live in prose anywhere else (I do not).

**12. tropical-types is Lean-pure: Lean Init only, zero sorry, zero Classical.choice, zero Mathlib.**
**STATUS: ✅.**
*Evidence:* **no `.lean` file, no `lakefile`, no `lean-toolchain` anywhere in januskey.** The only tropical content is 6 lines of Idris2 min-plus inside the ABI (`Types.idr:337-380`), and both "proofs" are `Refl` over literals (`Proofs.idr:196-201`: `tropAdd`/`tropMul` restated) — which is precisely the class of row this repo *deletes* elsewhere.
*Action:* never propose porting januskey's Idris tropical block into tropical-types; a Lean dependency introduced there on januskey's behalf would break the purity constraint outright. Offer the negative lesson only (§4.5).

**13. echo-types' identity claim is OPEN by design; the repo is built to falsify itself.**
**STATUS: ✅.**
*Evidence:* no echo-types dependency; januskey applies the same discipline to its *own* claim — the MPR row ("Every operation is reversible … as a theorem linked to the Rust") is `OPEN`, and the CLI's own `about` string over-claims it (see §4 finding D3).
*Action:* do not cite `resource-algebra`'s "echo loss grade" as support for echo-types' claim.

**14. residual-evidence-types is early research with no stable API; distinctness from echo+epistemic unanswered.**
**STATUS: ✅.**
*Evidence:* one comment mention (`scripts/fetch-siblings.sh:4`, "pattern: choreographic-types, residual-evidence-types"), no pin, no import, no API use.
*Action:* none; keep it out of januskey's dependency set until that repo publishes an API.

### Meta-level

**15. Name vs code.**
**STATUS: ✅ — and the discipline paid off: it found seven defects *inside* this repo.**
*Evidence:* every claim above cites a file/line or a ledger row read from source. The name-driven errors I found are all in januskey's own surfaces (appendix §4): a `docs/wiki` GDPR theorem, a `256-bit ECC` compliance row, a missing `jk shred` command that README documents, `EXPLAINME` file paths that no longer exist, three different `unwrap()` counts, and a `hexad` feed in an octad estate.
*Action:* fix the surfaces (§4), and keep the rule: no claim about a sibling that does not quote januskey source or the user's invariant.

**16. Invented connections.**
**STATUS: ✅.**
*Evidence — connections I am proposing, all source-backed:* `resource-algebra` (pinned commit), `ochrance` (manifest + unresolved hash algorithm), `valence-shell` (Phase 2, downstream), `verisimdb-data` (declared feed, inert), `hypatia` (scan config), `boj-server` (MCP cartridge, blocked), `panic-attacker` (Justfile shell-out), `aletheia` (deferred).
*Evidence — connections I explicitly decline:* Echo↔Resource bidirectional; any universal adapter; Echo-as-resource; a tropical-types dependency; an epistemic-types dependency; a choreographic-types dependency; any VCL implementation inside januskey.
*Only residual risk:* the A2ML "shared manifest" phrase (§3, item 3).

### Audit verdict on the model

**The structural model is intact. All sixteen invariants are consistent with what I found; none is contradicted by code.** The drift is elsewhere — in januskey's *published surfaces*, which over-claim against januskey's *own* instruments. Three items carry real uncertainty and are listed as such rather than resolved by inference: `hexad` vs octad (§1), whether `resource-algebra`'s "echo" is `echo-types`' echo (§2, §13), and whether `report-format = "logtalk"` is still a live enum (§11).

---

## 4. Defects found in this repo (the actual regression surface)

Ranked by hazard. Each is fixable here, by me, without touching any sibling.

| # | Defect | Receipt | Hazard |
|---|---|---|---|
| D1 | Wiki asserts GDPR Art. 17 compliance "with mathematical certainty" and a hand-waved theorem | `docs/wiki/theory/rmo-primitive.md:8,199-215` | **High** — legal/ethical over-claim, contradicts README + GLOSSARY + `X-1` |
| D2 | Security doc claims "256-bit ECC" | `docs/security/THREAT_MODEL.adoc:287` vs `keys.rs:66-76`, `keys_cli.rs:282-291` | **High** — claims an unimplemented primitive |
| D3 | CLI `about` says "Provably reversible file operations" while the MPR ledger row is `OPEN` | `main.rs:25` vs `PROOF-NEEDS.adoc §Claims ledger` (last row) | **High** — ships the over-claim in the binary |
| D4 | `EXPLAINME.adoc` describes a tree that no longer exists (`januskey-cli/src/metadata.rs`, `content_store.rs`), claims "100% Reversible", and prints "CRG Grade: C" | `EXPLAINME.adoc:11,29-41,65` vs `reversible-core/src/*.rs`, `STATE.a2ml` (D) | **Med** — it is the *receipts* document and it is stale |
| D5 | VeriSimDB feed declares `format = "hexad"` | `.machine_readable/integrations/verisimdb.a2ml` | **Med** — wrong arity in a machine-readable file |
| D6 | Three different `unwrap()` counts: 225 / 265 / 241 (measured) | `READINESS.adoc:14,130`, `PROOF-NEEDS.adoc §Current State` (265) vs `grep -c 'unwrap()'` = 241 | **Med** — dashboards that lie, in a repo whose thesis is "dashboards that lie" |
| D7 | `STATE.a2ml [test-coverage] total = 67 tests, 30 Idris2 proofs` — stale (READINESS/TOPOLOGY say 135 / 11 PROVEN) | `STATE.a2ml` (assessed 2026-04-03) vs `READINESS.adoc`, `TOPOLOGY.adoc` | **Med** — the machine-readable source of truth is the stale one |
| D8 | `0-AI-MANIFEST.a2ml` points at `.bot_directives/` (actual: `.machine_readable/bot_directives/`) and `README.md` (actual: `README.adoc`) | `0-AI-MANIFEST.a2ml` §Bot Directives / §Repository Structure | **Med** — the first file an agent reads is wrong |
| D9 | README documents `jk shred`; no `Shred` subcommand exists (it is `jk obliterate`), and `rename`/`obliterate`/`gc` are undocumented | `README.adoc §Commands`, `GLOSSARY.adoc` vs `main.rs:54-160` | **Low–Med** — name-vs-code, in the README |
| D10 | `threat-model.a2ml` still describes a home-rolled HMAC as the live state | `.machine_readable/threat-model.a2ml §primitives.mac` vs `attestation.rs:9,189-212`, `STATE.a2ml §blockers` (RESOLVED) | **Low** — stale in the security-critical direction (harmless, but wrong) |
| D11 | Ledger says "265" and "225" `unwrap()` calls in two places in the *same file* | `PROOF-NEEDS.adoc §Current State` / `§Recommended Prover` | **Low** |

Open technical gap I would take next after the surfaces: **T-8** — `MetadataStore::save` is a plain `fs::write` (`metadata.rs:288-293`) while `TransactionLog::save` already has the tested write→permission-copy→fsync→rename path (`transaction.rs:159-193`). Porting that one function closes a ledger row and removes an asymmetry no reader would guess from the module names. *(Done later the same day — see the Addendum. The row stays OPEN until the test runs.)*

### 4.1 Corrections applied (same day)

D1–D3 are fixed in the working tree, together with every other instance of the
same two claim classes (a guarantee of reversibility; a GDPR/erasure
determination). No ledger row changed status, and no test or proof count
changed, so `dashboard-check`'s dashboards/ledger reconciliation is unaffected.

| Defect | Change | Files |
|---|---|---|
| D1 | RMO page rewritten: what `jk obliterate` does (3 passes with `sync_all`, unlink, shared-blob rule, log purge), that the commitment is a tamper-evident record and not a proof of absence, that `storage_cleared` is self-asserted, a PROVEN/TESTED/ASSUMED/OPEN table, and an erasure-request table that states what a controller still has to determine. The invented `∎` theorem and the three CLI commands that do not exist (`--hash`, `obliteration-history`, `verify-obliteration`) are gone | `docs/wiki/theory/rmo-primitive.md` |
| D1 (siblings) | RMR page rewritten to the strength of the ledger (T-1..T-8), including what `FileMetadata::apply` actually restores — permissions yes, owner/group/mtime/symlink identity no — and that transactions provide resumable rollback, not isolation. MPR, MAA and formal-model pages de-claimed, and MAA's compliance table became a requirements table with a "what this does not establish" column | `rmr-primitive.md`, `mpr.md`, `maa-framework.md`, `formal-model.md`, `theory/index.md`, `Home.md`, `architecture/index.md` |
| D2 | Compliance mapping rewritten as design intent with a "what exists" column; the `256-bit ECC` row replaced (Ed25519/X25519 refused at the CLI); BLAKE3 removed from the primitives table (not a dependency); content store noted as unencrypted; the executive summary no longer claims MPR achieves guarantees | `docs/security/THREAT_MODEL.adoc` |
| D3 | `about` / `long_about` restated; the `Obliterate` help no longer claims a GDPR determination; module headers and both crate descriptions aligned | `crates/januskey-cli/src/main.rs`, `lib.rs`, `obliteration.rs`, `crates/reversible-core/src/lib.rs`, both `Cargo.toml` |
| D4 | EXPLAINME rewritten: the phantom "100% Reversible — every operation can be undone, always" heading (a claim the README does not actually make) became the mechanism it can defend; the file map now points at the files that exist (`reversible-core` for store/log/transaction, `operations.rs` for the executors); the audit-trail claim states what is purged by `obliterate`/`gc` and that the obliteration ledger is unkeyed; the phantom CRG "C (Beta)" line now matches STATE/READINESS (D) and says the checker enforces it | `EXPLAINME.adoc` |
| D5-adjacent | TOPOLOGY's diagram label `(Provably Reversible State)` → `(Reversible, proofs pending)`, width preserved; STATE records this session's work; the TOPOLOGY date is bumped so it does not predate STATE | `TOPOLOGY.adoc`, `.machine_readable/6a2/STATE.a2ml` |

Superseded by §4.2: D6, D7, D8 and D9 are closed; D10, the Trustfile
sentence, T-8 and the module move are blocked for the reasons tabulated there.

### 4.2 Identity resolved, and the mechanical defects (same day, second pass)

**ADR-0001 (`docs/adr/0001-identity-and-scope.md`)** resolves the identity from code
rather than from the name, the README or the Trustfile:

- The `jk` binary's implementation is `main.rs + lib.rs + operations.rs + delta.rs +
  obliteration.rs` over `reversible-core`. That is the product.
- Measured coupling to the key subsystem is **zero in both directions** (`main.rs`
  contains no `keys::`, `KeyManager` or `keys_cli` reference; `keys.rs` imports only
  `attestation::AuditLog`). The audit chain is **library-only** — `main.rs` never
  constructs an `AuditLog` — and the content store is plaintext, so the key store
  protects no file content.
- Decision: key lifecycle is not part of the product. The `jk-keys` binary is gated
  behind the opt-in `key-lifecycle` feature (default `cargo build --workspace`
  produces `jk` only); the module move to `crates/januskey-keys` is specified in
  ADR-0001 §5 with acceptance criteria, and is **not** applied because this session
  has no toolchain to build or test it.
- The invariant "not a cryptographic key system" is therefore satisfied for the
  *product* and not yet for the *module*; ADR-0001 §7 says so rather than claiming
  the win.

**Consumption claims pinned.** `reversible-core`'s "shared foundation that all three
applications use" was false. Ledger rows **V-1** (valence-shell), **V-2** (ochrance
verification) and **V-3** (aletheia) now carry status `DESIGNED` with a one-line
condition for changing it, the `ContentStore`/`ManifestEmitter`/`ReversibleExecutor`
doc comments say "intended, not consumed", and ARCHITECTURE has a
§Consumption status table. The ledger re-parses clean (46 rows, 4 cells each, all
non-run rows `-`, all artefacts present) and the PROVEN count is unchanged at 11.

**Mechanical defects closed:** **D6** (241 `unwrap()` calls, one number in both
files, with the measurement command; Rust LOC corrected from "~12,200" to the
measured 11,160), **D7** (STATE's test-coverage block rewritten to the reconciled
135 tests / 11 PROVEN rows, per-category figures marked as the un-re-derived
2026-04-03 assessment), **D8** (AI manifest paths: `.machine_readable/bot_directives/`,
`README.adoc`), **D9** (README and GLOSSARY now say `jk obliterate`; `rename`,
`obliterate`, `gc` documented; `KEY_LIFECYCLE` marks `obliterate`,
`verify-obliteration`, `restore` and `recover` as specified-but-not-implemented and
carries a scope banner).

**Still blocked, with the reason recorded rather than worked around:**

| Item | Blocked on |
|---|---|
| **T-8** (atomic metadata log) — the one open ledger row a second implementor would inherit | No `rustc`/`cargo`/crates.io in this sandbox. **Attempted later the same day** (Addendum): `write_atomic` need not move — it became `pub(crate)` and `save` calls it — so no T-5 test moved. Still unbuilt, so still OPEN |
| **D10** (`threat-model.a2ml` still describes the removed home-rolled HMAC) | Agent-denied: Trustfile `@trust-deny: modify-threat-model` |
| **Trustfile's "key-management tool" sentence** (its lines 8–10 contradict ADR-0001) | Agent-denied: Trustfile `@trust-deny: modify-trustfile`. Owner edit, ADR-0001 §6 |
| D5 (`hexad` feed in an octad estate) | Needs VeriSimDB's answer on whether `hexad` is a live format (prompt §5.5) |
| The `crates/januskey-keys` move | Toolchain (ADR-0001 §5) |

---

## 5. Handoff prompts — one per repo

Each is written to be pasted into its own chat verbatim. They carry the januskey facts the other side cannot see, ask only questions that repo can answer, and offer only things januskey actually has.

### 5.1 `resource-algebra` — the only hard proof dependency

```
Context: I am working in hyperpolymath/januskey (base commit 4afb710, 2026-10-29). Update from here, all receipts from source.

januskey's Agda kernel is stated in YOUR Loss instance, pinned at cf9235c ("Loss/MaxPlus/HWM as read 2026-10-12"), fetched by scripts/fetch-siblings.sh. Kernel flags are --safe --without-K --no-libraries --ignore-interfaces --double-check. Our ledger (PROOF-NEEDS.adoc) currently has three rows that depend on you:
- G-1 PROVEN locally: lossless traces have only lossless steps; obliteration absorbs; wrap/unwrap is grade 1 given cancellation. Our cancellation premise is a TESTED property test, not a proof.
- G-2 ASSUMED: our Idris2 IsCNO/ReversibleOp pair corresponds to your ReversiblePair "by transcription — no mechanical link claimed".
- G-3 OPEN: the FFI bridge preserves grade 1 (a homomorphism). We record that YOU list homomorphisms as OPEN, so G-3 is blocked on your side as much as ours.

What I need from you:
1. Is G-3 still open in your tree, and is there a statement shape you would accept as the homomorphism obligation so we can state it once, in your vocabulary, instead of twice?
2. Does the Loss instance intend to absorb a *failed* step (execute errors before mutation)? Our T-6 says "a failed execute leaves the filesystem unchanged" and is ASSUMED, so we cannot currently state a grade for the failure path.
3. Is "echo loss grade" (our Reversibility.agda header uses that phrase) your terminology for something with a fixed meaning? We cite it; we do not want to be citing a name we invented.

What I can offer: januskey's claims-ledger + dashboard-check pattern — every claim is a row with a status (PROVEN/TESTED/ASSUMED/DESIGNED/OPEN), an artefact and the command that discharges it, and CI fails if the counts printed on the dashboards disagree with what was measured. Also our negative-control gate: each rejection fixture must fail *with the specific error it exists for*, so a fixture that breaks for the wrong reason also fails the gate. Both are small and portable if useful.

Boundary: I will not infer your internals from repo names; everything above is what januskey's source asserts about you. Correct anything I have misquoted.
```

### 5.2 `absolute-zero` — lineage, not ground truth

```
Context: I am working in hyperpolymath/januskey (base 4afb710). Update from here, receipts from source.

Our docs place januskey downstream of you: maa-framework (policy) -> absolute-zero (CNO theory) -> januskey (PoC) -> {ochrance, valence-shell, aletheia}. src/abi/JanusKey/ABI/Types.idr:7 and crates/reversible-core/src/lib.rs:12 both record that lineage.

But we do NOT import your theorems. We re-prove what we use inside our own model: Proofs.idr::seqCNO ("composition of CNOs is a CNO") is PROVEN locally, and Layout.idr::copyIsCNO is stated over a (path,hash) log model — not over contents. Our ledger holds external assumptions explicitly as ASSUMED X-1 (hash one-wayness, physical erasure) rather than borrowing your Qed.

I understand your status as: CNO (Certified Null Effect) + OND (Observational Null Disclosure), 115 Coq Qed with 61 undischarged axioms, a consumer of proof infrastructure rather than a provider of ground truth, and OND-6 open by design. I am treating it that way — as a proof-bearing system with known open obligations — and I am not going to cite your Qed count as evidence for anything in januskey.

Questions:
1. Is there a canonical statement of the CNO identity (op ; inv ≡ CNO) in your tree that we should *point at* as lineage, even while proving our own? A stable name/commit would let us replace the folklore citation with a reference.
2. Does OND-6 have a target shape that a *grade*-based formulation could express? We state reversibility as a Loss grade (lossless = 1, loss = fin n, obliteration = absorbing) in Agda over resource-algebra, not as a Boolean. If OND-6 wants an observational statement, a grade may be the wrong or the right carrier and we would rather ask than assume.
3. Which of your open obligations are you content to have *downstream consumers silently ignore*? We currently ignore all of them by construction; if any are load-bearing for a consumer that claims CNO-composition, name them and we will mark them ASSUMED in our ledger.

What I can offer: our "removal with reason recorded in place" discipline. We deleted 18 lemmas that were tautologies or restatements (identityIsCNO was `s = s`; hashOneWay was `Void -> Void`) and left the reason beside the replacement in source, plus a fixed five-status vocabulary so "proven" cannot drift into "we think so". If any of your 61 undischarged axioms are of the restatement class, that census may be cheaper than discharging them.

Boundary: everything above is either januskey source or your stated status as given to me; I have not read your tree from here.
```

### 5.3 `ochrance` — the manifest bridge and an unresolved hash

```
Context: I am working in hyperpolymath/januskey (base 4afb710). Update from here, receipts from source.

januskey names you as the verifier in three places, and `ReversibleExecutor::generate_manifest` exists specifically to emit A2ML that you parse (crates/reversible-core/src/lib.rs:52-72, crates/reversible-core/src/manifest.rs). ARCHITECTURE.adoc §Phase 3 lists the pending work as adding Reversibility.idr types (ReversibleOp, ReversibilityProof) plus an OperationLog VerifiedSubsystem to your Idris2 ABI.

The blocker I want to resolve with you: hash algorithm. Our ContentHash is hard-coded SHA-256 with a "sha256:" prefix (content_store.rs:17-32) and its doc comment claims it "corresponds to ochrance's Hash type with algorithm = SHA256". ARCHITECTURE §Deferred says you use BLAKE3 height-indexed Merkle trees and that "the Rust side needs a compatible implementation". Those two statements cannot both be right. Until this is settled I am refusing to call the manifest portable — the manifest is exactly where a "one format fits all backends" assumption would form, and the estate's no-go theorem says that shape is impossible.

Questions:
1. What is your Hash type today: is `algorithm` a parameter, a fixed BLAKE3, or SHA-256? Please answer from your source.
2. At what level do you need agreement — the leaf hash, the Merkle shape, or only the A2ML field names? If only the field names, we can be honest that the bytes differ and stop pretending the trees match.
3. Do you want Lax/Checked/Attested as an ordered strengthening (each implies the last), or as independent witnesses? Our docs currently describe them as three witnesses; if you have specified an ordering, we are describing you wrongly.
4. Your VerifiedSubsystem interface: is `verify` total over malformed manifests, or may it fail? We would rather generate fewer manifests than generate ones your checker has to reject.

What I can offer: (a) an A2ML emitter with a frozen field list and a version field you can gate on; (b) our ABI-layout guard (scripts/check-abi-layout.sh) — it checks Idris2 struct sizes/enum order/exported symbols against a C header and the Zig FFI, with a negative control that perturbs one code and requires FAIL. If you carry an Idris2 ABI, that guard is the cheap way to stop transcription drift.

Boundary: I have not read your tree; the above is januskey's own claims about you, and I expect some of them to be stale.
```

### 5.4 `valence-shell` — you are downstream; also, the direction is inverted in my model

```
Context: I am working in hyperpolymath/januskey (base 4afb710). Update from here, receipts from source.

Two things, one of which is a correction to my own model.

1. Correction: I had been carrying the belief that januskey is "part of Valence Shell". Our own ARCHITECTURE.adoc §Lineage says the opposite — januskey is upstream of you (maa-framework -> absolute-zero -> januskey -> {ochrance, valence-shell, aletheia}), and reversible-core is the library extracted in Phase 1 (2026-03-21, workspace builds, 44 tests) for you to consume. If your tree says something different, that is worth knowing now, because one of us is describing the dependency graph backwards.

2. Phase 2 is listed as NEXT and unstarted from our side: wire valence-shell (impl/rust-cli/) to depend on reversible-core; add ContentStore to ShellState so undo data is content-addressed instead of the inline `undo_data: Option<Vec<u8>>`; add a2ml_emitter.rs; replace verification.rs stubs with a ReversibleExecutor impl. Nothing in januskey currently requires you to move; the trait is stable enough to read but has never had a second implementor, and I would rather you be the one to find its seams.

What I would ask you to check before I publish anything as stable:
- Does execute/undo returning Metadata (rather than a handle) fit the shell's undo stack, or do you need the pre-state handed back?
- Our undo takes a metadata_id: &str, i.e. it looks the operation up in our log. Is that a workable contract for a shell that wants to undo *its own* last action, or does it force you to keep a parallel index?
- We record our trust boundary explicitly: the Agda/Idris2 -> Rust step is not mechanical (ledger row G-2 is ASSUMED: "transcription, no mechanical link claimed"), and the FFI grade-preservation obligation (G-3) is OPEN. If VSH's selling point is the extraction gap from Lean, ours is a different gap in a different pair of provers — please do not let the two be conflated in write-ups, in either direction.

What I can offer: the ReversibleExecutor trait, the content store (SHA-256, dedup), the operation log with a tested atomic save (write -> permission-preserving open -> fsync -> rename, with mutants that each fail a test), and the claims-ledger discipline so that "proved" in VSH means a named artefact ran in CI, not a claim in a README.

Boundary: I have not read your tree from here.
```

### 5.5 `VeriSimDB` / `nextgen-databases` — a feed declaration that is inert and the wrong arity

```
Context: I am working in hyperpolymath/januskey (base 4afb710). Update from here, receipts from source.

januskey declares exactly one integration with you: .machine_readable/integrations/verisimdb.a2ml — type "data-feed", data-store "verisimdb-data", destination "verisimdb-data/feeds/", with emit-scan-results, emit-build-metrics, emit-dependency-graph all true. Two problems I want to fix on my side rather than leave as a lie in machine-readable metadata:

1. Nothing emits it. There is no network code anywhere in crates/ (no reqwest/hyper/TcpStream/ureq; grep returns zero), and no writer for that destination. The declaration is aspirational.
2. It says format = "hexad". My understanding of the estate is that VeriSimDB is an 8-modal octad engine. A hexad feed into an octad store is either a subset I should declare as such, or dead config from before the arity changed. I am not going to guess which.

Questions:
1. Is "hexad" still a valid feed format, and if so what is it a projection of? If it is retired, what is the current identifier?
2. For a file-level reversibility tool, which of the three declared emitters are actually useful to you — build metrics and dependency graph are estate-wide scan outputs, not januskey's own data. Would scan results alone be the honest minimum?
3. Do you ingest by file drop into feeds/, or is there a VCL-shaped path you would prefer? My working model of the estate is that subsystems communicate *through* VeriSimDB via VCL and never point-to-point, so if a plain file drop into verisimdb-data counts as a bypass, I want to know before implementing it.
4. What does a *correct* januskey feed even contain? The obvious candidates are operation-log records (op kind, content hash, timestamp, transaction id) — but our ledger is explicit that operation metadata is a plain fs::write today (row T-8, OPEN), so until that is fixed any feed would be reporting from a non-atomic source. Tell me if you would rather receive nothing until T-8 closes.

What I can offer: the concrete shape of an operation record and its content hashes, plus the caution that a feed is unverifiable provenance unless the emitting side can show its own log integrity. I would rather give you a small honest feed than a broad fake one.

Boundary: I have not read your tree; the octad/hexad framing above is my model of the estate, not a quotation from your source.
```

### 5.6 `epistemic-types` — the concrete defect our model has, which your model may be able to state

```
Context: I am working in hyperpolymath/januskey (base 4afb710). Update from here, receipts from source.

januskey has a small epistemic model in its Idris2 ABI (src/abi/JanusKey/ABI/Types.idr:378-400): `data Knows : Agent -> Type -> Type` with OwnerKnows/SystemKnows/AuditorKnows, over agents Owner/System/Auditor/Adversary. It is discrete-constructive — no 0.0-1.0 confidence anywhere, and I am not asking you for one, since I understand your model to be discrete too (FactiveModality has reflect = knowledge; BeliefModality has no reflect = belief), with monadic bind and comonadic extract as future commitments rather than provided.

Two obligations our model provably cannot state, both recorded in our ledger as OPEN:
- E-1: "an attacker cannot recover obliterated content or use a revoked key." We cannot state it because `Knows Adversary a` is simply uninhabited — the comment in Proofs.idr says so outright: it holds "only by construction of the model", which we classify as "a modelling choice, not a security proof".
- "The system forgets the old key" after rotation. We deleted the lemma (systemForgetsOldKey) because Knows has no negative form: it could only return knowledge of the new key and say nothing about the old. Our ledger records the deletion.

That is the concrete question I would like to put to you: is there a modality shape — in your vocabulary, without inventing anything — that expresses *ignorance* or *forgetting* as a positive claim rather than as an absence of a constructor? Two sub-questions:
1. Does your non-reflective side (belief) help at all here, or is belief-about-X still a positive knowledge claim about X?
2. If bind/extract do not exist yet, is there a *weaker* structural property you would already stand behind (e.g. that forgetting is monotone, or that a revoked-key state is not reachable from a believing state)?

What I can offer: a real, small consumer with a ledger that would mark any such lemma PROVEN/TESTED/ASSUMED/OPEN and would fail CI if we claimed more than we discharged — i.e. a place where a statement from you would be *used*, not just added to a pile. Also our negative-control fixtures: three Idris2 files that must be refused (double close, forgotten close, unknown-status-as-Ok) with the specific error named in the file.

Boundary: I have not read your tree. I am treating your model as discrete and bind/extract as unprovided, and if either is wrong, say so before answering the question above.
```

### 5.7 `tropical-types` — a name collision, and nothing more

```
Context: I am working in hyperpolymath/januskey (base 4afb710). Update from here, receipts from source.

Short version: januskey does not depend on tropical-types, and after this recon I am going to keep it that way. I am writing partly to record that, and partly because januskey currently contains a *name-level* lookalike that could confuse the estate later.

The lookalike: src/abi/JanusKey/ABI/Types.idr §"Level 11: Tropical Cost-Tracking" defines `TropCost n` with tropAdd = min, tropMul = +, copyCost n = n, oblitCost n = 3n, and Proofs.idr §8 states two lemmas — sequentialCostAdditive and parallelCostMin — whose proofs are both `Refl` over literals. In other words it is min-plus arithmetic restated, and by our own ledger rules (we deleted 18 rows of this class, e.g. identityIsCNO was `s = s`) it is not evidence of anything. It exists because an older Idris ABI wanted cost bounds in the operation types.

What that means for you:
1. I will not propose porting it, importing it, or citing it as a tropical-anything result. If a Lean file appeared downstream on januskey's account, that would break your purity constraint (Lean Init only, zero sorry, zero Classical.choice, zero Mathlib) for no gain.
2. If januskey ever needs a real cost bound, the right move is to ask you for the statement in your vocabulary and consume it the way we consume resource-algebra — pinned by commit, never re-declared. That is the pattern we already use for our one genuine proof dependency.
3. If you would rather the lookalike not exist at all, say so and I will delete Level 11 from the ABI and record the removal with the reason in place, which is our house style.

What I can offer: the pinned-sibling pattern (fetch by commit, verify the prover version, never vendor), and the negative-control discipline — the fixture must be refused with the error it exists for, otherwise the gate fails. If your purity constraint is currently enforced only by review, that is a cheap gate to add: grep for Mathlib/Classical.choice/sorry and fail on any hit.

Boundary: I have not read your tree; "zero sorry / zero Classical.choice / zero Mathlib" is your stated constraint as I understand it, not something I verified from here.
```

### 5.8 `choreographic-types` — K-CUT is yours; januskey only borrows your CI pattern

```
Context: I am working in hyperpolymath/januskey (base 4afb710). Update from here, receipts from source.

No dependency, no pin, no code. The only mentions of you in januskey are two comments describing your *build pattern*: .github/workflows/agda-proofs.yml:3 ("Pattern: choreographic-types / residual-evidence-types (pinned Debian prover, sibling fetched by commit)") and scripts/fetch-siblings.sh:4. So this is a courtesy update plus one genuine question, not a request for a dependency.

My model: K-CUT ("grading and transport commute with endpoint projection across a consistent frontier") is an OPEN CONJECTURE in your repo; VCL-UT L8 is blocked until it resolves; K-CUT-LOSS and K-CUT-WARRANT are both OPEN. I am treating choreographic-types as research-blocked and L8 as not deployable, and I am not going to cite you as infrastructure anywhere.

The genuine question — an analogy I do not want to overstate. We have a structurally similar obligation called G-3: "the FFI bridge preserves grade 1 (homomorphism)", i.e. a property that a bridge commutes with the algebraic structure, open on both sides (resource-algebra lists homomorphisms as OPEN). We got there the honest way: our ledger row G-2 says the Idris2 and Agda presentations correspond "by transcription — no mechanical link claimed", so the bridge property has nothing to lean on.

1. Has the K-CUT work produced a *statement shape* for "grading commutes with projection" that is generic enough to be worth restating for a grade-preserving FFI boundary? I am asking for shape, not proof.
2. Is there a formulation in which K-CUT being false would be *informative* rather than fatal — e.g. a frontier condition that names exactly where it fails? That is the style we now prefer: we keep four rejection fixtures that must fail with a named error, because a broken proof attempt is only useful if it fails for the reason you intended.
3. If we reword our two comments to "pattern, no dependency", is there a form of words you would prefer, so that the citation does not read as an integration?

What I can offer: the pinned-sibling + pinned-prover pattern in concrete form (Debian agda-bin=2.6.4.3-1+b2 with the version asserted at runtime, sibling fetched by commit, --safe --without-K --double-check --ignore-interfaces), and the expect-text rejection harness, which is about 30 lines of bash and has already caught fixtures that were passing for the wrong reason.

Boundary: I have not read your tree; K-CUT/L8 status above is my model, and I would like it corrected if it has moved.
```

### 5.9 `echo-types` — a word I use that may not be mine

```
Context: I am working in hyperpolymath/januskey (base 4afb710). Update from here, receipts from source.

januskey has no code relationship with echo-types: no import, no pin, no reference in any Rust or Idris file. But it has one *word*, and I would rather ask than assume.

verification/proofs/agda/Reversibility.agda:6-7 says: "januskey's operations are graded in resource-algebra's Loss instance (the echo loss grade): a reversible step costs 1 = fin 0, information loss is fin n, obliteration is inf = 0." That phrase — "the echo loss grade" — is ours, not quoted from anyone, and I do not know whether the echo in it refers to echo-types or is internal vocabulary of resource-algebra's Loss instance.

What I am treating as fixed, so you can correct me:
- EchoBridge proves the bridge is E->R only; NotResourceInstance proves Echo is not a resource instance. So: one-directional, no bidirectional conversion, and Echo is not a resource.
- echo-types' own identity claim is OPEN by design (criterion 2 unmet) and the repo is built to falsify its own claim. I am not treating it as established, and I am not citing our "echo loss grade" as support for it.

Questions:
1. Is "echo loss grade" your term, resource-algebra's, or neither? If it is yours, what does the grade attach to, and does our usage (cost of a trace step) misdescribe it?
2. Since Echo is not a resource instance and the bridge is E->R only, is there any legitimate direction in which a *file-operation* log could be an echo input — i.e. an observational trace that Echo consumes? I am asking before building anything, not after.
3. Is there a one-line statement of the E->R-only result you would like quoted when other repos mention it, so the directionality does not get flattened into "Echo and resources interoperate"?

What I can offer: nothing that depends on you, which is the point. If a plausible-looking januskey -> echo-types link ever appears, it would be an invented connection of exactly the kind the estate's separation theorems refute.

Boundary: I have not read your tree, and I will not restate your theorems from memory beyond the two named above.
```

### 5.10 `residual-evidence-types` — cited once, depended on never

```
Context: I am working in hyperpolymath/januskey (base 4afb710). Update from here, receipts from source.

There is exactly one mention of you in januskey: a comment in scripts/fetch-siblings.sh:4 — "Fetch the proof sibling at a pinned commit (pattern: choreographic-types, residual-evidence-types)". It is a citation of a build pattern, not an import; our only pinned sibling is resource-algebra at cf9235c. No API is used, no type is named, nothing is expected to be stable.

My model: residual-evidence-types is early research with no stable API, and its key open question is whether it is distinct from echo + epistemic or subsumed by them. So I am treating it as research, not infrastructure, and januskey will not take a dependency until that question resolves.

The only thing I would ask: when the distinctness question resolves, whichever way it goes, a one-line note in your repo would be enough for me to stop guessing — and if the answer is "subsumed", then januskey has no business citing the name at all and I will delete the mention.

What I can offer in the meantime, if it is useful: januskey's evidence discipline for exactly this problem — the thing that makes a claim unusable downstream is that nobody can see which part was checked. Our claims ledger forces every claim into one of five statuses (PROVEN/TESTED/ASSUMED/DESIGNED/OPEN) with an artefact and the command that discharges it, and CI fails if a dashboard prints a count that the checker did not measure. A research repo can use the same table with mostly OPEN rows; the value is that the OPEN rows are visible instead of implied.

Boundary: I have not read your tree; the distinctness question above is my model of your open problem, not a quotation.
```

### 5.11 `hypatia` — one config value that may name a removed system

```
Context: I am working in hyperpolymath/januskey (base 4afb710). Update from here, receipts from source.

Two items, both small, both about januskey's config describing you correctly.

1. .machine_readable/6a2/NEUROSYM.a2ml (last updated 2026-03-16, declared authoritative by 0-AI-MANIFEST.a2ml) carries `[hypatia-config] scan-enabled = true, scan-depth = "standard", report-format = "logtalk"`. My understanding of the estate is that Logtalk was removed and that the live rule system is Elixir (rules/hypatia-elixir/), with the .lgt files dead artefacts. januskey has zero .lgt files. So either "logtalk" is still a valid report-format identifier in your current tool and I should leave it, or that value is naming a removed system and is exactly the kind of dead pointer that later gets mistaken for a live dependency. Please answer from your source which it is.

2. .hypatia/state is stale in this repo: activity.jsonl has a single entry (2026-03-08, hypatia-autofix, scan, fixes=0) and last-visit.json reports visits_total = 1. If the scanner has run since, the state file is not being written; if it has not, that is worth knowing too. Also worth flagging: the estate tech-debt scan of 2026-05-26 recorded "Idr-blv = 1" for januskey — one believe_me-class marker in a proof-bearing file — while our ledger row asserts no believe_me in src/abi (enforced by .machine_readable/contractiles/Mustfile.a2ml:110-111, which greps only src/abi/). So the marker is probably outside the guarded path. If your scanner can name the file:line, that one finding is worth more to me than the aggregate count.

What I can offer: a claims-ledger-shaped target for scan output — if a scan finding carried claim-id, status, artefact and command, it could be reconciled by the same checker that reconciles our dashboards (crates/dashboard-check), instead of being a number a human has to trust. Also our negative-control pattern for scanner fixtures.

Boundary: I have not read your tree; the Logtalk removal and Elixir rules path are my model of the estate.
```

### 5.12 `aletheia` — deferred on purpose

```
Context: I am working in hyperpolymath/januskey (base 4afb710). Update from here, receipts from source.

januskey lists you as the third downstream application of its reversibility work (maa-framework -> absolute-zero -> januskey -> {ochrance, valence-shell, aletheia}) and then defers you explicitly: "aletheia: Too early (Phase 0 research, no operation types yet)". I am keeping it deferred — no wiring, no expectations, and I am not going to design your operation types from here.

What januskey actually has that you may eventually want, stated with its limits:
- reversible-core: content store (SHA-256, dedup), operation metadata, transaction log with a tested atomic save (write -> open with the log's mode -> fsync -> rename, with six mutants that each fail a test) and a ReversibleExecutor trait (execute/undo/generate_manifest).
- The limits matter more than the features: transaction atomicity is proved only for the *model* (three Agda rows over a log model), the metadata store's own save is still a plain fs::write (our row T-8, OPEN), the Idris2-to-Agda correspondence is ASSUMED not mechanical, and the headline "every operation is reversible" is an OPEN row in our ledger despite what our README and CLI banner say. I would rather you inherit an accurate picture than an optimistic one.
- OS-level operations are a different problem from file-level ones: our proofs are about a (path, hash) log, not about block devices, and our secure-overwrite path is best-effort only (SSD wear levelling, CoW, journaling). If aletheia's claim is OS-level reversibility, the gap between our model and your domain is not a porting exercise.

What I would ask, whenever you get to Phase 1: tell me which of execute/undo/metadata is the wrong shape for an OS operation, rather than fitting your design to our trait. A trait with one implementor that nobody has stress-tested should not become the estate's interface by default.

Boundary: I have not read your tree.
```

### 5.13 `boj-server` — a cartridge blocked on a signature, not on code

```
Context: I am working in hyperpolymath/januskey (base 4afb710). Update from here, receipts from source.

A `januskey-mcp` cartridge is recorded in our STATE as blocked, not by engineering but by governance: .machine_readable/threat-model.a2ml is status "draft-pending-human-review" (authored 2026-04-19) and cannot be signed off by an agent. So nothing is coming to you yet, and I will not route around the block.

What the draft already establishes, from source, so you can judge the exposure: AES-256-GCM key wrapping with per-key fresh nonces and the key UUID as AAD; Argon2id at 64 MiB / 3 iterations / 4 lanes / 16-byte salt; SHA-256 fingerprints and audit-chain linking; a second binary jk-keys that manages key lifecycle including revocation and obliteration. Two of the draft's findings are now stale in the safe direction and one in the unsafe direction: the home-rolled HMAC (H(key||data||prev)) has been replaced with real HMAC-SHA256 via the hmac crate (attestation.rs:9,189-212), but Ed25519/X25519 remain enum labels only — generation is refused at the CLI (keys_cli.rs:282-291) — while docs/security/THREAT_MODEL.adoc:287 still advertises "256-bit ECC" in a compliance table.

What I would ask of you, before any cartridge exists:
1. What is boj-server's minimum threat-model bar for a cartridge — a signed document, a specific findings list closed, or a review record? I would rather close the right list than sign the wrong one.
2. Does a cartridge need to expose obliteration at all? Our obliteration path is irreversible by design and best-effort physically; exposing it over MCP converts a local file tool into a remote data-destruction primitive, and I think that should be a separate decision from exposing reversible operations.
3. Would you accept a cartridge that exposes only read-only history/status first, so the surface is small while the draft is unsigned?

What I can offer: the draft's primitive inventory as a file:line-anchored document you can review directly, plus the ledger discipline so "the threat model is approved" cannot quietly become "the cartridge has no open findings".

Boundary: I have not read boj-server's source; the above is januskey's own blocked state.
```

---

## 6. No prompt needed — deliberately out of scope for januskey

| Repo | Why no prompt |
|---|---|
| `JtV` (v2, ReversalToken) | Zero references in januskey. Separate research programme; convergence monitored, not depended on. A prompt would be manufactured contact. |
| `ephapax` | Zero references. Legacy preservation is provably false (`Counterexample.v`, 5 Qed); the `Admitted` is durable, and januskey attempts nothing here. |
| `ECHIDNA`, `proof-burrower` | Zero references. `echidnabot` appears once, in `0-AI-MANIFEST.a2ml`, as an estate code-quality bot — a different role from the VCL peer in the invariant list, and not something januskey configures. |
| `gv-clade-index` | `.machine_readable/CLADE.a2ml` is a registry record (uuid `e216170e…`, clade `fv`, phase `active` since 2026-03-16). Nothing to discuss; it is a correct declaration. |
| `panic-attacker`, `reposystem`, `proof-of-work`, `vql-ut` | The first is a real shell-out (`Justfile:135`, 241 `unwrap()` sites to feed it); the other three appear only in `EXPLAINME.adoc` as "Dogfooted Across The Account" speculation — which is itself D4-class drift and should be deleted rather than acted on. |

---

## 7. Summary

**Intent.** januskey is a file-reversibility tool with an unusually honest verification apparatus: a claims ledger, a dashboard checker that fails the build when prose disagrees with measurement, prover-version-pinned gates, and rejection fixtures that must fail for the right reason. It is at 60 % / grade D, alpha, with the headline claim still an OPEN ledger row.

**Audit result.** All sixteen invariants are consistent with what the code actually does; none is contradicted. The three items I am not fully certain about are stated as uncertainties rather than resolved by inference: the `hexad` feed in an octad estate (§1), whether `resource-algebra`'s "echo loss grade" is `echo-types`' echo (§2, §13), and whether `report-format = "logtalk"` is a live enum or dead config (§11).

**Where the drift actually is.** Not in the structural model — in januskey's own published surfaces, which over-claim against januskey's own instruments: a wiki that asserts GDPR Article 17 "with mathematical certainty", a security doc advertising an ECC implementation that is refused at the CLI, a CLI banner claiming "provably reversible" over an OPEN row, a receipts file describing a directory tree that no longer exists, and three different `unwrap()` counts. That is the same failure mode this repo was built to detect, one layer up from the code.

**Next, in this repo, in order:** (1) ~~the three over-claims D1–D3~~ — **done, see §4.1**; (2) the stale surfaces D4–D8; (3) T-8 — point `MetadataStore::save` at the already-tested atomic path — because it is the one open ledger row that a second implementor (valence-shell, ochrance) would inherit; (4) reword the two sibling "pattern" citations so a citation cannot be read as a dependency.

**Cross-repo posture.** One hard proof dependency (`resource-algebra`), one intended verifier (`ochrance`), one intended consumer (`valence-shell`), one inert data feed (`VeriSimDB`), one blocked cartridge (`boj-server`). Everything else in the invariant list is a sibling I read about and do not depend on — which is, on this evidence, the correct relationship.

---

## Addendum — same day, later, on this branch

The body above is left as the record of what was true when it was measured.
Two things changed afterwards.

**T-8 is attempted, and still OPEN.** `MetadataStore::save`
(`crates/reversible-core/src/metadata.rs`) now calls
`crate::transaction::write_atomic`, the write → sync → rename path that
`transaction.rs` already used and that T-5 already tests; `write_atomic` became
`pub(crate)` for the purpose, so nothing moved out of `transaction.rs` and no
T-5 test changed. A new test,
`metadata::tests::save_replaces_the_log_atomically`, asserts that a save leaves
no temporary file behind, ignores a torn one planted from an earlier crash, and
that the log parses on reopen. The ledger row stays **OPEN**: no toolchain in
this session built or ran the test, so the evidence is "written" and not
"observed". The retry half of the row — an undo whose `mark_undone` fails to
persist — is untouched. Declared test count moves 135 → 136 in `STATE.a2ml`,
`READINESS.adoc` and `TOPOLOGY.adoc`, which is the figure `dashboard-check`
measures with `cargo test -- --list`.

**The proof claims are now graded, and the grade is Grade 3 for four rows.**
`docs/theory/PROOF-GRADES.adoc` maps the 46-row ledger onto
`hyperpolymath/mathesis` §5/§6 and `hyperpolymath/standards`
`3-practice/PROOF-POLICY.adoc` §1/§2. The result is not a relabel: of the 11
PROVEN rows, the four whose artefact is Agda (G-1, T-1, T-2, T-3) type-check
in Agda and have not been translated to another foundational system, which is
mathesis's definition of Grade 3. The receipt is a green `agda-proofs.yml` run
(40867211509) at commit `4afb710` with Agda 2.6.4.3 and `resource-algebra`
pinned at `cf9235c`; the modules, the gate script, the sibling fetch and the
workflow are byte-identical at that commit and here. The other seven PROVEN
rows are Idris2 interface/ABI theorems, which PROOF-POLICY §1 classifies as
engineering rather than theory, so the grade ladder does not apply to them.
Nothing is claimed at Grade 2i, 2ii or 1. Two obligations are recorded rather
than skipped: the canonical invocation's `--cubical-compatible` is absent
because the flag is infectious and the pinned sibling is `--without-K`; and CI
pins Agda twice, at 2.6.4.3 (the receipt) and 2.6.3 (the dashboard job).

**Toolchains, and what that changed.** The sandbox's egress allowlist decides
which prover can be run at all. Measured, with the exact failures, in
`docs/estate/environment-toolchains.md`: Agda (2.6.4.3, the CI pin) and Zig
(0.13.0, the e2e pin) install from PyPI wheels and both gates now run here —
`check-agda.sh` green, `ffi/zig` building with 15/15 integration tests passing —
while Rust, Creusot and Idris2 cannot be installed, because
`static.rust-lang.org`, `crates.io`, `deb.debian.org` and the GitHub release-asset
host are all outside the allowlist. Idris2 is anyway already pinned at its latest
release (v0.8.0). Running the Agda gate locally is what turned the §2.1 question
from an inference into a measurement: the canonical invocation
(`--safe --without-K --cubical-compatible`) type-checks both modules, so the gate
now uses it and the earlier prediction that it would fail on the pinned
`--without-K` sibling is withdrawn.

**Out-of-scope findings, written as separate prompts.** Four, in
`docs/estate/prompts/`, one per repository, none of them touching januskey:
`standards` (four gaps in PROOF-POLICY §2.1/§2.2 that implementing it exposed),
`mathesis` (whether a Grade 3 receipt may be local, how to record non-axiom
caveats, what a checker change does to an existing grade), `resource-algebra`
(the OPEN homomorphism that blocks row G-3, plus `--cubical-compatible` in its
pragmas), and `the-metadatastician` (whether it ingests A2ML manifests — the
question behind ledger row V-2). Two of the three repositories named in the
request do not resolve: `hyperpolymath/stuff-of-awesome` and
`hyperpolymath/catachresis-colloquium` both return 404 to an authenticated
`gh api`, and neither name appears among the owner's 360 public repositories; the
nearest match for the third is `the-metadatastician`. No prompt was written for a
repository that could not be found and read.

