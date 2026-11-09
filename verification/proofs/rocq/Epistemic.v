(* SPDX-License-Identifier: MPL-2.0 *)
(* SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell *)
(*
   Rocq recheck leg of ledger row E-1m (PROOF-POLICY §2, route: re-proved).

   Source of truth: verification/proofs/agda/Epistemic.agda, theorems
   knows-observation, different-store-unknown, adversary-ignorant,
   obliterate-cuts-recovery, obliterate-ignorance, rotate-keeps-new,
   rotate-forgets-old and rotate-forgets-canUse. This file restates them and
   proves them again by hand, using only the Rocq prelude. Why re-proved and
   not transported: see Rollback.v and docs/theory/PROOF-GRADES.adoc, which
   also records the statement comparison.
*)

(* Agda: _==_ on Nat, and the one fact the proofs need about it. *)
Fixpoint eqb (m n : nat) : bool :=
  match m, n with
  | O, O => true
  | O, S _ => false
  | S _, O => false
  | S m', S n' => eqb m' n'
  end.

Lemma eqb_refl : forall n, eqb n n = true.
Proof. induction n as [| n IH]; [reflexivity | exact IH]. Qed.

Definition ContentHash := nat.
Definition Content := nat.
Definition KeyId := nat.

Inductive Agent : Type := owner | system | auditor | adversary.

Record World : Type := w {
  content : ContentHash -> option Content;
  usable : Agent -> KeyId -> bool;
  see : Agent -> list (ContentHash * Content)
}.

Definition observe (a : Agent) (x : World) : list (ContentHash * Content) := see x a.

(* Knowledge as indiscriminability. *)
Definition Knows (a : Agent) (phi : World -> Prop) (x : World) : Prop :=
  forall x', observe a x' = observe a x -> phi x'.

Definition CanUse (x : World) (a : Agent) (k : KeyId) : Prop := usable x a k = true.

(* Agda: Σ Content (λ c → content x h ≡ just c), a dependent pair in Set. *)
Definition Recovers (x : World) (h : ContentHash) : Type :=
  { c : Content & content x h = Some c }.

(* Theorem 1. An agent knows its own observation. *)
Theorem knows_observation : forall a x,
  Knows a (fun y => observe a y = observe a x) x.
Proof. intros a x y e. exact e. Qed.

(* Theorem 2. Ignorance is a proposition. *)
Theorem different_store_unknown : forall a x x',
  observe a x' = observe a x -> content x' <> content x ->
  ~ Knows a (fun y => content y = content x) x.
Proof. intros a x x' e n k. exact (n (k x' e)). Qed.

Definition x1 : World := w (fun _ => Some 1) (fun _ _ => true) (fun _ => nil).
Definition x2 : World := w (fun _ => Some 2) (fun _ _ => true) (fun _ => nil).

(* Theorem 3. Same observation, different content: the adversary does not
   know the stored value. *)
Theorem adversary_ignorant : ~ Knows adversary (fun y => content y = content x1) x1.
Proof.
  intro k. pose proof (f_equal (fun f => f 0) (k x2 eq_refl)) as H.
  simpl in H. discriminate H.
Qed.

Definition obliterate (h : ContentHash) (x : World) : World :=
  w (fun k => if eqb k h then None else content x k) (usable x) (see x).

(* Theorem 4. Obliteration cuts recovery. *)
Theorem obliterate_cuts_recovery : forall h x, Recovers (obliterate h x) h -> False.
Proof.
  intros h x [c p]. simpl in p. rewrite eqb_refl in p. discriminate p.
Qed.

(* Theorem 5. Obliteration does not make the adversary any wiser. *)
Theorem obliterate_ignorance :
  ~ Knows adversary (fun y => content y = content (obliterate 0 x1)) (obliterate 0 x1).
Proof.
  intro k. pose proof (f_equal (fun f => f 0) (k x2 eq_refl)) as H.
  simpl in H. discriminate H.
Qed.

(* Theorem 6. Rotation to a different key forgets the old one. *)
Definition rotate (old new : KeyId) (x : World) : World :=
  w (content x)
    (fun a k => if eqb k new then true else (if eqb k old then false else usable x a k))
    (see x).

Theorem rotate_keeps_new : forall a old new x, usable (rotate old new x) a new = true.
Proof. intros a old new x. simpl. rewrite eqb_refl. reflexivity. Qed.

Theorem rotate_forgets_old : forall a old new x,
  eqb old new = false -> usable (rotate old new x) a old = false.
Proof. intros a old new x n. simpl. rewrite n, eqb_refl. reflexivity. Qed.

Theorem rotate_forgets_canUse : forall a old new x,
  eqb old new = false -> ~ CanUse (rotate old new x) a old.
Proof.
  intros a old new x n p. unfold CanUse in p.
  rewrite (rotate_forgets_old a old new x n) in p. discriminate p.
Qed.

Print Assumptions knows_observation.
Print Assumptions different_store_unknown.
Print Assumptions adversary_ignorant.
Print Assumptions obliterate_cuts_recovery.
Print Assumptions obliterate_ignorance.
Print Assumptions rotate_keeps_new.
Print Assumptions rotate_forgets_old.
Print Assumptions rotate_forgets_canUse.
