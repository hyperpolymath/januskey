-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
--
-- Lean recheck leg of ledger rows T-2 and T-3 (PROOF-POLICY §2, route:
-- re-proved).
--
-- Source of truth: verification/proofs/agda/SingleActiveLog.agda, module Log,
-- theorems begin-good, modify-good, finish-good and at-most-one-active, with
-- the refusal lemmas they rest on. This file restates them and proves them
-- again by hand in core Lean 4: no Mathlib, no imports, and no `simp` (whose
-- lemmas can bring in `propext`). Why re-proved and not transported: see
-- Rollback.lean and docs/theory/PROOF-GRADES.adoc, which also records the
-- statement comparison.

namespace JanusKey.SingleActiveLog

inductive Phase : Type where
  | active
  | settled

open Phase

/-- Agda: data All. -/
inductive All {A : Type} (P : A → Prop) : List A → Prop where
  | nil : All P []
  | cons {x : A} {xs : List A} : P x → All P xs → All P (x :: xs)

theorem all_app {A : Type} {P : A → Prop} :
    ∀ {xs ys : List A}, All P xs → All P ys → All P (xs ++ ys)
  | [], _, _, k => k
  | _ :: _, _, .cons p ps, k => .cons p (all_app ps k)

section Log

variable {R I : Type} (dec : (a b : I) → PSum (a = b) (a ≠ b))
  (key : R → I) (phase : R → Phase)

structure Log (R I : Type) : Type where
  txs : List R
  activeId : Option I

/-- Rewrite the FIRST record with key `i`; `none` if there is none. -/
def update (i : I) (f : R → R) : List R → Option (List R)
  | [] => none
  | t :: ts =>
    match dec (key t) i with
    | .inl _ => some (f t :: ts)
    | .inr _ =>
      match update i f ts with
      | none => none
      | some us => some (t :: us)

/-- Open a new record; refused while a record is active. -/
def begin (r : R) : Log R I → Option (Log R I)
  | ⟨_, some _⟩ => none
  | ⟨ts, none⟩ => some ⟨ts ++ [r], some (key r)⟩

/-- Edit the active record; refused when idle. -/
def modify (f : R → R) : Log R I → Option (Log R I)
  | ⟨_, none⟩ => none
  | ⟨ts, some i⟩ =>
    match update dec key i f ts with
    | none => none
    | some us => some ⟨us, some i⟩

/-- Settle the active record; refused when idle. -/
def finish (g : R → R) : Log R I → Option (Log R I)
  | ⟨_, none⟩ => none
  | ⟨ts, some i⟩ =>
    match update dec key i g ts with
    | none => none
    | some us => some ⟨us, none⟩

inductive Good : Log R I → Prop where
  | idle {ts : List R} : All (fun u => phase u = settled) ts → Good ⟨ts, none⟩
  | busy {pre : List R} {t : R} {post : List R} :
      All (fun u => key u ≠ key t) pre →
      All (fun u => phase u = settled) pre →
      phase t = active →
      All (fun u => phase u = settled) post →
      Good ⟨pre ++ t :: post, some (key t)⟩

theorem good_empty : Good key phase (⟨[], none⟩ : Log R I) := .idle .nil

/-- Only the active record is rewritten. -/
theorem update_active (f : R → R) (t : R) (post : List R) :
    ∀ {pre : List R}, All (fun u => key u ≠ key t) pre →
      update dec key (key t) f (pre ++ t :: post) = some (pre ++ f t :: post)
  | [], .nil => by
    rw [List.nil_append, List.nil_append]
    unfold update
    cases dec (key t) (key t) with
    | inl _ => rfl
    | inr n => exact absurd rfl n
  | u :: pre, .cons n ns => by
    rw [List.cons_append, List.cons_append]
    unfold update
    cases dec (key u) (key t) with
    | inl e => exact absurd e n
    | inr _ =>
      rw [update_active f t post ns]

/-- A key-preserving edit keeps every earlier record distinct from it. -/
theorem distinct_after {k k' : I} (e : k' = k) :
    ∀ {pre : List R}, All (fun u => key u ≠ k) pre → All (fun u => key u ≠ k') pre
  | [], .nil => .nil
  | _ :: _, .cons n ns => .cons (fun q => n (q.trans e)) (distinct_after e ns)

theorem begin_refused_while_active (r : R) (ts : List R) (j : I) :
    begin key r ⟨ts, some j⟩ = none := rfl

theorem begin_good {r : R} {L L' : Log R I} (act : phase r = active)
    (G : Good key phase L) (fr : All (fun u => key u ≠ key r) L.txs)
    (E : begin key r L = some L') : Good key phase L' := by
  cases G with
  | @idle ts ss =>
    cases E
    exact @Good.busy R I key phase ts r [] fr ss act .nil
  | busy _ _ _ _ => cases E

theorem modify_refused_when_idle (f : R → R) (ts : List R) :
    modify dec key f (⟨ts, none⟩ : Log R I) = none := rfl

/-- An edit must keep the key and the phase of the record it edits. -/
theorem modify_good {f : R → R} {L L' : Log R I}
    (kf : ∀ r, key (f r) = key r) (pf : ∀ r, phase (f r) = phase r)
    (G : Good key phase L) (E : modify dec key f L = some L') :
    Good key phase L' := by
  cases G with
  | idle _ => cases E
  | @busy pre t post np sp a sq =>
    have h : modify dec key f (⟨pre ++ t :: post, some (key t)⟩ : Log R I)
        = some ⟨pre ++ f t :: post, some (key t)⟩ := by
      dsimp only [modify]
      rw [update_active dec key f t post np]
    rw [h] at E
    cases E
    rw [← kf t]
    exact Good.busy (distinct_after key (kf t) np) sp ((pf t).trans a) sq

theorem finish_refused_when_idle (g : R → R) (ts : List R) :
    finish dec key g (⟨ts, none⟩ : Log R I) = none := rfl

/-- A settling transition must land in the settled phase. -/
theorem finish_good {g : R → R} {L L' : Log R I}
    (pg : ∀ r, phase (g r) = settled)
    (G : Good key phase L) (E : finish dec key g L = some L') :
    Good key phase L' := by
  cases G with
  | idle _ => cases E
  | @busy pre t post np sp a sq =>
    have h : finish dec key g (⟨pre ++ t :: post, some (key t)⟩ : Log R I)
        = some ⟨pre ++ g t :: post, none⟩ := by
      dsimp only [finish]
      rw [update_active dec key g t post np]
    rw [h] at E
    cases E
    exact Good.idle (all_app sp (.cons (pg t) sq))

def countActive : List R → Nat
  | [] => 0
  | t :: ts =>
    match phase t with
    | active => countActive ts + 1
    | settled => countActive ts

def expected : Option I → Nat
  | none => 0
  | some _ => 1

theorem settled_count : ∀ {ts : List R},
    All (fun u => phase u = settled) ts → countActive phase ts = 0
  | [], .nil => rfl
  | t :: ts, .cons s ss => by
    show (match phase t with
          | active => countActive phase ts + 1
          | settled => countActive phase ts) = 0
    rw [s]
    exact settled_count ss

theorem count_app : ∀ (xs ys : List R),
    countActive phase (xs ++ ys) = countActive phase xs + countActive phase ys
  | [], ys => (Nat.zero_add _).symm
  | x :: xs, ys => by
    show (match phase x with
          | active => countActive phase (xs ++ ys) + 1
          | settled => countActive phase (xs ++ ys))
        = (match phase x with
          | active => countActive phase xs + 1
          | settled => countActive phase xs) + countActive phase ys
    cases phase x with
    | active => rw [count_app xs ys, Nat.add_right_comm]
    | settled => rw [count_app xs ys]

/-- At most one record is active, and exactly when the pointer is set. -/
theorem at_most_one_active {L : Log R I} (G : Good key phase L) :
    countActive phase L.txs = expected L.activeId := by
  cases G with
  | idle ss => exact settled_count phase ss
  | @busy pre t post _ sp a sq =>
    show countActive phase (pre ++ t :: post) = 1
    rw [count_app, settled_count phase sp]
    show 0 + (match phase t with
              | active => countActive phase post + 1
              | settled => countActive phase post) = 1
    rw [a, settled_count phase sq]
    rfl

end Log

end JanusKey.SingleActiveLog

#print axioms JanusKey.SingleActiveLog.begin_good
#print axioms JanusKey.SingleActiveLog.modify_good
#print axioms JanusKey.SingleActiveLog.finish_good
#print axioms JanusKey.SingleActiveLog.at_most_one_active
#print axioms JanusKey.SingleActiveLog.begin_refused_while_active
#print axioms JanusKey.SingleActiveLog.modify_refused_when_idle
#print axioms JanusKey.SingleActiveLog.finish_refused_when_idle
