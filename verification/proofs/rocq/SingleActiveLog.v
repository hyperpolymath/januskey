(* SPDX-License-Identifier: MPL-2.0 *)
(* SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell *)
(*
   Rocq recheck leg of ledger rows T-2 and T-3 (PROOF-POLICY §2, route:
   re-proved).

   Source of truth: verification/proofs/agda/SingleActiveLog.agda, module
   Log, theorems begin-good, modify-good, finish-good and at-most-one-active,
   with the refusal lemmas they rest on. This file restates them and proves
   them again by hand, using only the Rocq prelude. Why re-proved and not
   transported: see Rollback.v and docs/theory/PROOF-GRADES.adoc, which also
   records the statement comparison.
*)

Local Open Scope list_scope.

Inductive Phase : Type := active | settled.

(* Agda: data All. *)
Inductive All {A : Type} (P : A -> Prop) : list A -> Prop :=
  | All_nil : All P nil
  | All_cons : forall x xs, P x -> All P xs -> All P (cons x xs).

Lemma all_app : forall (A : Type) (P : A -> Prop) xs ys,
  All P xs -> All P ys -> All P (xs ++ ys).
Proof.
  intros A P xs ys H K. induction H as [| x xs p _ IH]; simpl.
  - exact K.
  - exact (All_cons P x (xs ++ ys) p IH).
Qed.

Section Log.

Variables R I : Type.
Variable dec : forall a b : I, {a = b} + {a <> b}.
Variable key : R -> I.
Variable phase : R -> Phase.

Record Log : Type := mkLog { txs : list R; activeId : option I }.

Definition Settled (r : R) : Prop := phase r = settled.
Definition Fresh (i : I) (ts : list R) : Prop := All (fun u => key u <> i) ts.

(* Rewrite the FIRST record with key i; None if there is none. *)
Fixpoint update (i : I) (f : R -> R) (ts : list R) : option (list R) :=
  match ts with
  | nil => None
  | cons t ts' =>
      if dec (key t) i then Some (cons (f t) ts')
      else match update i f ts' with
           | None => None
           | Some us => Some (cons t us)
           end
  end.

(* Open a new record; refused while a record is active. *)
Definition begin (r : R) (L : Log) : option Log :=
  match L with
  | mkLog _ (Some _) => None
  | mkLog ts None => Some (mkLog (ts ++ cons r nil) (Some (key r)))
  end.

(* Edit the active record; refused when idle. *)
Definition modify (f : R -> R) (L : Log) : option Log :=
  match L with
  | mkLog _ None => None
  | mkLog ts (Some i) =>
      match update i f ts with
      | None => None
      | Some us => Some (mkLog us (Some i))
      end
  end.

(* Settle the active record; refused when idle. *)
Definition finish (g : R -> R) (L : Log) : option Log :=
  match L with
  | mkLog _ None => None
  | mkLog ts (Some i) =>
      match update i g ts with
      | None => None
      | Some us => Some (mkLog us None)
      end
  end.

Inductive Good : Log -> Prop :=
  | idle : forall ts, All Settled ts -> Good (mkLog ts None)
  | busy : forall pre t post,
      All (fun u => key u <> key t) pre ->
      All Settled pre ->
      phase t = active ->
      All Settled post ->
      Good (mkLog (pre ++ cons t post) (Some (key t))).

Lemma good_empty : Good (mkLog nil None).
Proof. exact (idle nil (All_nil _)). Qed.

(* Only the active record is rewritten. *)
Lemma update_active : forall f pre t post,
  All (fun u => key u <> key t) pre ->
  update (key t) f (pre ++ cons t post) = Some (pre ++ cons (f t) post).
Proof.
  intros f pre t post H. induction H as [| u pre' n _ IH]; simpl.
  - destruct (dec (key t) (key t)) as [_ | n].
    + reflexivity.
    + exfalso. exact (n eq_refl).
  - destruct (dec (key u) (key t)) as [e | _].
    + exfalso. exact (n e).
    + rewrite IH. reflexivity.
Qed.

(* A key-preserving edit keeps every earlier record distinct from it. *)
Lemma distinct_after : forall (k k' : I) pre, k' = k ->
  All (fun u => key u <> k) pre -> All (fun u => key u <> k') pre.
Proof.
  intros k k' pre e H. induction H as [| u pre' n _ IH].
  - exact (All_nil _).
  - refine (All_cons _ u pre' _ IH).
    intro q. exact (n (eq_trans q e)).
Qed.

Lemma begin_refused_while_active : forall r ts j,
  begin r (mkLog ts (Some j)) = None.
Proof. reflexivity. Qed.

Theorem begin_good : forall r L L',
  phase r = active -> Good L -> Fresh (key r) (txs L) ->
  begin r L = Some L' -> Good L'.
Proof.
  intros r L L' act G fr E. destruct G as [ts ss | pre t post np sp a sq].
  - simpl in E. injection E as <-.
    exact (busy ts r nil fr ss act (All_nil _)).
  - discriminate E.
Qed.

Lemma modify_refused_when_idle : forall f ts,
  modify f (mkLog ts None) = None.
Proof. reflexivity. Qed.

(* An edit must keep the key and the phase of the record it edits. *)
Theorem modify_good : forall f L L',
  (forall r, key (f r) = key r) -> (forall r, phase (f r) = phase r) ->
  Good L -> modify f L = Some L' -> Good L'.
Proof.
  intros f L L' kf pf G E. destruct G as [ts ss | pre t post np sp a sq].
  - discriminate E.
  - simpl in E. rewrite (update_active f pre t post np) in E.
    injection E as <-. rewrite <- (kf t).
    exact (busy pre (f t) post
             (distinct_after (key t) (key (f t)) pre (kf t) np)
             sp (eq_trans (pf t) a) sq).
Qed.

Lemma finish_refused_when_idle : forall g ts,
  finish g (mkLog ts None) = None.
Proof. reflexivity. Qed.

(* A settling transition must land in the settled phase. *)
Theorem finish_good : forall g L L',
  (forall r, phase (g r) = settled) ->
  Good L -> finish g L = Some L' -> Good L'.
Proof.
  intros g L L' pg G E. destruct G as [ts ss | pre t post np sp a sq].
  - discriminate E.
  - simpl in E. rewrite (update_active g pre t post np) in E.
    injection E as <-.
    exact (idle _ (all_app _ _ pre _ sp (All_cons _ (g t) post (pg t) sq))).
Qed.

Fixpoint countActive (ts : list R) : nat :=
  match ts with
  | nil => 0
  | cons t ts' =>
      match phase t with
      | active => S (countActive ts')
      | settled => countActive ts'
      end
  end.

Definition expected (o : option I) : nat :=
  match o with None => 0 | Some _ => 1 end.

Lemma settled_count : forall ts, All Settled ts -> countActive ts = 0.
Proof.
  intros ts H. induction H as [| t ts' s _ IH]; simpl.
  - reflexivity.
  - unfold Settled in s. rewrite s. exact IH.
Qed.

Lemma count_app : forall xs ys,
  countActive (xs ++ ys) = countActive xs + countActive ys.
Proof.
  intros xs ys. induction xs as [| x xs IH]; simpl.
  - reflexivity.
  - destruct (phase x); simpl; rewrite IH; reflexivity.
Qed.

(* At most one record is active, and exactly when the pointer is set. *)
Theorem at_most_one_active : forall L,
  Good L -> countActive (txs L) = expected (activeId L).
Proof.
  intros L G. destruct G as [ts ss | pre t post np sp a sq]; simpl.
  - exact (settled_count ts ss).
  - rewrite count_app. simpl. rewrite a.
    rewrite (settled_count pre sp), (settled_count post sq). reflexivity.
Qed.

End Log.

Print Assumptions begin_good.
Print Assumptions modify_good.
Print Assumptions finish_good.
Print Assumptions at_most_one_active.
Print Assumptions begin_refused_while_active.
Print Assumptions modify_refused_when_idle.
Print Assumptions finish_refused_when_idle.
