-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
--
-- Lean recheck leg of ledger row G-1 (PROOF-POLICY §2, route: re-proved).
--
-- Source of truth: verification/proofs/agda/Reversibility.agda, module Steps,
-- theorems all-lossless-trace, lossless-trace-all, obliterate-absorbs,
-- obliterated-not-lossless, wrap-unwrap-𝟙 and wrap-unwrap-id. This file
-- restates them and proves them again by hand in core Lean 4: no Mathlib, no
-- imports, and no `simp`.
--
-- The Agda module takes the Loss grade from the pinned sibling
-- resource-algebra (ResourceAlgebra.Instances.Loss at cf9235c). A recheck leg
-- cannot import that library, so the part G-1 uses is restated below: the
-- carrier, the sequence product `+g`, its left unit and both annihilation
-- laws. Why re-proved and not transported: see Rollback.lean and
-- docs/theory/PROOF-GRADES.adoc, which also records the statement comparison.

namespace JanusKey.Reversibility

/-- resource-algebra, Instances/Loss.agda: Grade. -/
inductive Grade : Type where
  | fin : Nat → Grade
  | inf : Grade

open Grade

/-- resource-algebra, Instances/Loss.agda: _+g_. -/
def gplus : Grade → Grade → Grade
  | fin m, fin n => fin (m + n)
  | fin _, inf => inf
  | inf, _ => inf

/-- `+g-unitˡ`. Lean's `Nat.add` recurses on its second argument, so
`0 + a = a` is a lemma here, where Agda and Rocq have it by computation. -/
theorem gplus_unit_l : ∀ a, gplus (fin 0) a = a
  | fin a => congrArg fin (Nat.zero_add a)
  | inf => rfl

theorem gplus_annihil_l (a : Grade) : gplus inf a = inf := rfl

theorem gplus_annihil_r : ∀ a, gplus a inf = inf
  | fin _ => rfl
  | inf => rfl

section Steps

variable {S : Type}

/-- A step: what it does to the state, and what it costs in Loss. -/
structure Step (S : Type) : Type where
  run : S → S
  grade : Grade

inductive Trace (S : Type) : Type where
  | nil : Trace S
  | cons : Step S → Trace S → Trace S

def traceGrade : Trace S → Grade
  | .nil => fin 0
  | .cons s ts => gplus s.grade (traceGrade ts)

def Lossless (s : Step S) : Prop := s.grade = fin 0

inductive AllLossless : Trace S → Prop where
  | nil : AllLossless .nil
  | cons {s : Step S} {ts : Trace S} : Lossless s → AllLossless ts → AllLossless (.cons s ts)

/-- Theorem 1. A trace of lossless steps is lossless. -/
theorem all_lossless_trace : ∀ {ts : Trace S}, AllLossless ts → traceGrade ts = fin 0
  | .nil, .nil => rfl
  | .cons s ts, .cons p ps => by
    rw [traceGrade]
    rw [show s.grade = fin 0 from p, gplus_unit_l, all_lossless_trace ps]

theorem gplus_zero_left : ∀ a b, gplus a b = fin 0 → a = fin 0
  | fin 0, fin _, _ => rfl
  -- Split on n so that m + 1 + n is a successor by computation: no lemma
  -- about addition is needed. (`Nat.succ_ne_zero` is not used: in this core
  -- library it depends on `propext`.)
  | fin (_ + 1), fin 0, h => by
    injection h with h
    exact Nat.noConfusion h
  | fin (_ + 1), fin (_ + 1), h => by
    injection h with h
    exact Nat.noConfusion h
  | fin _, inf, h => nomatch h
  | inf, _, h => nomatch h

theorem gplus_zero_right : ∀ a b, gplus a b = fin 0 → b = fin 0
  | fin _, fin 0, _ => rfl
  | fin _, fin (n + 1), h => by
    injection h with h
    exact Nat.noConfusion h
  | fin _, inf, h => nomatch h
  | inf, _, h => nomatch h

/-- Theorem 2. A lossless trace hides no lossy step. -/
theorem lossless_trace_all : ∀ (ts : Trace S), traceGrade ts = fin 0 → AllLossless ts
  | .nil, _ => .nil
  | .cons _ ts, p =>
    .cons (gplus_zero_left _ _ p) (lossless_trace_all ts (gplus_zero_right _ _ p))

inductive Contains (s : Step S) : Trace S → Prop where
  | here {ts : Trace S} : Contains s (.cons s ts)
  | there {t : Step S} {ts : Trace S} : Contains s ts → Contains s (.cons t ts)

def Obliterating (s : Step S) : Prop := s.grade = inf

/-- Theorem 3. Obliteration absorbs, whatever comes before or after. -/
theorem obliterate_absorbs {s : Step S} (o : Obliterating s) :
    ∀ {ts : Trace S}, Contains s ts → traceGrade ts = inf
  | .cons _ ts, .here => by
    rw [traceGrade, show s.grade = inf from o]
    exact gplus_annihil_l (traceGrade ts)
  | .cons t _, .there c => by
    rw [traceGrade, obliterate_absorbs o c]
    exact gplus_annihil_r t.grade

theorem obliterated_not_lossless {s : Step S} {ts : Trace S}
    (o : Obliterating s) (c : Contains s ts) (p : traceGrade ts = fin 0) : False := by
  rw [obliterate_absorbs o c] at p
  exact nomatch p

/-- The reversible pair. Both steps are ASSIGNED grade 𝟙, as in the Agda. -/
structure ReversiblePair (S : Type) : Type where
  wrap : S → S
  unwrap : S → S
  cancels : ∀ x, unwrap (wrap x) = x

def wrapStep (p : ReversiblePair S) : Step S := ⟨p.wrap, fin 0⟩
def unwrapStep (p : ReversiblePair S) : Step S := ⟨p.unwrap, fin 0⟩

theorem wrap_unwrap_one (p : ReversiblePair S) :
    traceGrade (.cons (wrapStep p) (.cons (unwrapStep p) .nil)) = fin 0 := rfl

theorem wrap_unwrap_id (p : ReversiblePair S) (x : S) :
    (unwrapStep p).run ((wrapStep p).run x) = x := p.cancels x

end Steps

end JanusKey.Reversibility

#print axioms JanusKey.Reversibility.all_lossless_trace
#print axioms JanusKey.Reversibility.lossless_trace_all
#print axioms JanusKey.Reversibility.obliterate_absorbs
#print axioms JanusKey.Reversibility.obliterated_not_lossless
#print axioms JanusKey.Reversibility.wrap_unwrap_one
#print axioms JanusKey.Reversibility.wrap_unwrap_id
