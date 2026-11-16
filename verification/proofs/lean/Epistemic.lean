-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
--
-- Lean recheck leg of ledger row E-1m (PROOF-POLICY §2, route: re-proved).
--
-- Source of truth: verification/proofs/agda/Epistemic.agda, theorems
-- knows-observation, different-store-unknown, adversary-ignorant,
-- obliterate-cuts-recovery, obliterate-ignorance, rotate-keeps-new,
-- rotate-forgets-old and rotate-forgets-canUse. This file restates them and
-- proves them again by hand in core Lean 4: no Mathlib, no imports, and no
-- `simp`. Why re-proved and not transported: see Rollback.lean and
-- docs/theory/PROOF-GRADES.adoc, which also records the statement comparison.

namespace JanusKey.Epistemic

/-- Agda: `_==_` on Nat. -/
def eqb : Nat → Nat → Bool
  | 0, 0 => true
  | 0, _ + 1 => false
  | _ + 1, 0 => false
  | m + 1, n + 1 => eqb m n

theorem eqb_refl : ∀ n, eqb n n = true
  | 0 => rfl
  | n + 1 => eqb_refl n

/-- Agda: `if_then_else_` on Bool, by matching (not Lean's `if` on `Prop`). -/
def ite' {A : Type} : Bool → A → A → A
  | true, a, _ => a
  | false, _, b => b

abbrev ContentHash := Nat
abbrev Content := Nat
abbrev KeyId := Nat

inductive Agent : Type where
  | owner
  | system
  | auditor
  | adversary

structure World : Type where
  content : ContentHash → Option Content
  usable : Agent → KeyId → Bool
  see : Agent → List (ContentHash × Content)

def observe (a : Agent) (x : World) : List (ContentHash × Content) := x.see a

/-- Knowledge as indiscriminability. -/
def Knows (a : Agent) (φ : World → Prop) (x : World) : Prop :=
  ∀ x', observe a x' = observe a x → φ x'

def CanUse (x : World) (a : Agent) (k : KeyId) : Prop := x.usable a k = true

/-- Agda: `Σ Content (λ c → content x h ≡ just c)`, a dependent pair in `Set`. -/
def Recovers (x : World) (h : ContentHash) : Type :=
  PSigma (fun c : Content => x.content h = some c)

/-- Theorem 1. An agent knows its own observation. -/
theorem knows_observation (a : Agent) (x : World) :
    Knows a (fun y => observe a y = observe a x) x := fun _ e => e

/-- Theorem 2. Ignorance is a proposition. -/
theorem different_store_unknown (a : Agent) (x x' : World)
    (e : observe a x' = observe a x) (n : x'.content ≠ x.content) :
    ¬ Knows a (fun y => y.content = x.content) x := fun k => n (k x' e)

def x₁ : World := ⟨fun _ => some 1, fun _ _ => true, fun _ => []⟩
def x₂ : World := ⟨fun _ => some 2, fun _ _ => true, fun _ => []⟩

/-- Theorem 3. Same observation, different content: the adversary does not
know the stored value. -/
theorem adversary_ignorant : ¬ Knows .adversary (fun y => y.content = x₁.content) x₁ :=
  fun k =>
    have h : (some 2 : Option Nat) = some 1 := congrArg (fun f => f 0) (k x₂ rfl)
    -- `nomatch h` on two numerals brings in `propext`, so the numerals are
    -- told apart by `eqb`, which computes.
    have b : true = false :=
      congrArg (fun o : Option Nat => match o with | some n => eqb n 2 | none => false) h
    nomatch b

def obliterate (h : ContentHash) (x : World) : World :=
  ⟨fun k => ite' (eqb k h) none (x.content k), x.usable, x.see⟩

/-- Theorem 4. Obliteration cuts recovery. -/
theorem obliterate_cuts_recovery (h : ContentHash) (x : World) :
    Recovers (obliterate h x) h → False
  | ⟨c, p⟩ => by
    have q : ite' (eqb h h) none (x.content h) = some c := p
    rw [eqb_refl] at q
    exact nomatch q

/-- Theorem 5. Obliteration does not make the adversary any wiser. -/
theorem obliterate_ignorance :
    ¬ Knows .adversary (fun y => y.content = (obliterate 0 x₁).content) (obliterate 0 x₁) :=
  fun k =>
    have h : (some 2 : Option Nat) = none := congrArg (fun f => f 0) (k x₂ rfl)
    nomatch h

def rotate (old new : KeyId) (x : World) : World :=
  ⟨x.content,
   fun a k => ite' (eqb k new) true (ite' (eqb k old) false (x.usable a k)),
   x.see⟩

theorem rotate_keeps_new (a : Agent) (old new : KeyId) (x : World) :
    (rotate old new x).usable a new = true := by
  show ite' (eqb new new) true (ite' (eqb new old) false (x.usable a new)) = true
  rw [eqb_refl]
  rfl

/-- Theorem 6. Rotation to a different key forgets the old one. -/
theorem rotate_forgets_old (a : Agent) (old new : KeyId) (x : World)
    (n : eqb old new = false) : (rotate old new x).usable a old = false := by
  show ite' (eqb old new) true (ite' (eqb old old) false (x.usable a old)) = false
  rw [n, eqb_refl]
  rfl

theorem rotate_forgets_canUse (a : Agent) (old new : KeyId) (x : World)
    (n : eqb old new = false) : ¬ CanUse (rotate old new x) a old := fun p => by
  have q : (rotate old new x).usable a old = true := p
  rw [rotate_forgets_old a old new x n] at q
  exact nomatch q

end JanusKey.Epistemic

#print axioms JanusKey.Epistemic.knows_observation
#print axioms JanusKey.Epistemic.different_store_unknown
#print axioms JanusKey.Epistemic.adversary_ignorant
#print axioms JanusKey.Epistemic.obliterate_cuts_recovery
#print axioms JanusKey.Epistemic.obliterate_ignorance
#print axioms JanusKey.Epistemic.rotate_keeps_new
#print axioms JanusKey.Epistemic.rotate_forgets_old
#print axioms JanusKey.Epistemic.rotate_forgets_canUse
