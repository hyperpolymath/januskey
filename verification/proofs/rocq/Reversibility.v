(* SPDX-License-Identifier: MPL-2.0 *)
(* SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell *)
(*
   Rocq recheck leg of ledger row G-1 (PROOF-POLICY §2, route: re-proved).

   Source of truth: verification/proofs/agda/Reversibility.agda, module Steps,
   theorems all-lossless-trace, lossless-trace-all, obliterate-absorbs,
   obliterated-not-lossless, wrap-unwrap-𝟙 and wrap-unwrap-id. This file
   restates them and proves them again by hand, using only the Rocq prelude.

   The Agda module takes the Loss grade from the pinned sibling
   resource-algebra (ResourceAlgebra.Instances.Loss at cf9235c). A recheck leg
   cannot import that library, so the part G-1 uses is restated below: the
   carrier, the sequence product `+g`, its left unit and both annihilation
   laws. The alternatives operation and the dioid laws are not used by G-1
   and are not restated. Why re-proved and not transported: see Rollback.v and
   docs/theory/PROOF-GRADES.adoc, which also records the statement
   comparison.
*)

(* resource-algebra, Instances/Loss.agda: Grade and _+g_. *)
Inductive Grade : Type := fin : nat -> Grade | inf : Grade.

Definition gplus (a b : Grade) : Grade :=
  match a, b with
  | fin m, fin n => fin (m + n)
  | fin _, inf => inf
  | inf, _ => inf
  end.

(* The laws G-1 uses: +g-unitˡ (seq.unitˡ), annihilˡ and annihilʳ. *)
Lemma gplus_unit_l : forall a, gplus (fin 0) a = a.
Proof. intros [a|]; reflexivity. Qed.

Lemma gplus_annihil_l : forall a, gplus inf a = inf.
Proof. reflexivity. Qed.

Lemma gplus_annihil_r : forall a, gplus a inf = inf.
Proof. intros [a|]; reflexivity. Qed.

Section Steps.

Variable S : Type.

(* A step: what it does to the state, and what it costs in Loss. *)
Record Step : Type := step { run : S -> S; grade : Grade }.

Inductive Trace : Type :=
  | tnil : Trace
  | tcons : Step -> Trace -> Trace.

Fixpoint traceGrade (ts : Trace) : Grade :=
  match ts with
  | tnil => fin 0
  | tcons s ts' => gplus (grade s) (traceGrade ts')
  end.

Definition Lossless (s : Step) : Prop := grade s = fin 0.

Inductive AllLossless : Trace -> Prop :=
  | al_nil : AllLossless tnil
  | al_cons : forall s ts, Lossless s -> AllLossless ts -> AllLossless (tcons s ts).

(* Theorem 1. A trace of lossless steps is lossless. *)
Theorem all_lossless_trace : forall ts, AllLossless ts -> traceGrade ts = fin 0.
Proof.
  intros ts H. induction H as [| s ts p _ IH]; simpl.
  - reflexivity.
  - unfold Lossless in p. rewrite p, gplus_unit_l. exact IH.
Qed.

Lemma gplus_zero_left : forall a b, gplus a b = fin 0 -> a = fin 0.
Proof.
  intros [[|m]|] [n|]; simpl; intro H; (reflexivity || discriminate H).
Qed.

Lemma gplus_zero_right : forall a b, gplus a b = fin 0 -> b = fin 0.
Proof.
  intros [[|m]|] [[|n]|]; simpl; intro H; (reflexivity || discriminate H).
Qed.

(* Theorem 2. A lossless trace hides no lossy step. *)
Theorem lossless_trace_all : forall ts, traceGrade ts = fin 0 -> AllLossless ts.
Proof.
  intros ts. induction ts as [| s ts IH]; simpl; intro p.
  - exact al_nil.
  - exact (al_cons s ts (gplus_zero_left _ _ p) (IH (gplus_zero_right _ _ p))).
Qed.

Inductive Contains (s : Step) : Trace -> Prop :=
  | here : forall ts, Contains s (tcons s ts)
  | there : forall t ts, Contains s ts -> Contains s (tcons t ts).

Definition Obliterating (s : Step) : Prop := grade s = inf.

(* Theorem 3. Obliteration absorbs, whatever comes before or after. *)
Theorem obliterate_absorbs : forall s ts,
  Obliterating s -> Contains s ts -> traceGrade ts = inf.
Proof.
  intros s ts o c. induction c as [ts | t ts _ IH]; simpl.
  - unfold Obliterating in o. rewrite o. apply gplus_annihil_l.
  - rewrite IH. apply gplus_annihil_r.
Qed.

Theorem obliterated_not_lossless : forall s ts,
  Obliterating s -> Contains s ts -> traceGrade ts = fin 0 -> False.
Proof.
  intros s ts o c p. rewrite (obliterate_absorbs s ts o c) in p. discriminate p.
Qed.

(* The reversible pair. Both steps are ASSIGNED grade 𝟙, as in the Agda. *)
Record ReversiblePair : Type := mkPair {
  wrap : S -> S;
  unwrap : S -> S;
  cancels : forall x, unwrap (wrap x) = x
}.

Definition wrapStep (p : ReversiblePair) : Step := step (wrap p) (fin 0).
Definition unwrapStep (p : ReversiblePair) : Step := step (unwrap p) (fin 0).

Theorem wrap_unwrap_one : forall p,
  traceGrade (tcons (wrapStep p) (tcons (unwrapStep p) tnil)) = fin 0.
Proof. reflexivity. Qed.

Theorem wrap_unwrap_id : forall p x,
  run (unwrapStep p) (run (wrapStep p) x) = x.
Proof. intros p x. exact (cancels p x). Qed.

End Steps.

Print Assumptions all_lossless_trace.
Print Assumptions lossless_trace_all.
Print Assumptions obliterate_absorbs.
Print Assumptions obliterated_not_lossless.
Print Assumptions wrap_unwrap_one.
Print Assumptions wrap_unwrap_id.
