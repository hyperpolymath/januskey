-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
--
-- Lean recheck leg of ledger row T-1 (PROOF-POLICY §2, route: re-proved).
--
-- Source of truth: verification/proofs/agda/TransactionAtomicity.agda,
-- module Rollback, theorem rollback-restores. This file restates that theorem
-- and proves it again by hand in core Lean 4: no Mathlib, no imports.
--
-- Why re-proved and not transported: PROOF-POLICY §2 re-proves the Lean leg
-- until a released Lambdapi has a Lean exporter, and the Agda → Dedukti
-- translator (Deducteam/Agda2Dedukti, last changed January 2022) cannot read
-- modules checked by the pinned Agda 2.6.4.3. docs/theory/PROOF-GRADES.adoc
-- records the statement comparison.

namespace JanusKey.Rollback

/-- A recorded operation (Agda: record Op). `execute` may fail; `undo`,
applied to the state a successful execute produced, restores the state it
started from. -/
structure Op (S : Type) : Type where
  execute : S → Option S
  undo : S → S
  undoes : ∀ {x y : S}, execute x = some y → undo y = x

/-- The recorded list ran successfully from `x` to `z` (Agda: data Executed).
It lives in `Type`, as the Agda family lives in `Set`. -/
inductive Executed {S : Type} : List (Op S) → S → S → Type where
  | done {x : S} : Executed [] x x
  | next {o : Op S} {os : List (Op S)} {x y z : S} :
      o.execute x = some y → Executed os y z → Executed (o :: os) x z

/-- `cmd_rollback`: undo the last recorded operation first (Agda: rollback). -/
def rollback {S : Type} : List (Op S) → S → S
  | [], z => z
  | o :: os, z => o.undo (rollback os z)

/-- Theorem A-1 (Agda: rollback-restores). -/
theorem rollback_restores {S : Type} {os : List (Op S)} {x z : S} :
    Executed os x z → rollback os z = x := by
  intro h
  induction h with
  | done => rfl
  | @next o os x y z e _ ih =>
    show o.undo (rollback os z) = x
    rw [ih]
    exact o.undoes e

end JanusKey.Rollback

#print axioms JanusKey.Rollback.rollback_restores
