(* SPDX-License-Identifier: MPL-2.0 *)
(* SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell *)
(*
   Rocq recheck leg of ledger row T-1 (PROOF-POLICY §2, route: re-proved).

   Source of truth: verification/proofs/agda/TransactionAtomicity.agda,
   module Rollback, theorem rollback-restores. This file restates that
   theorem and proves it again by hand, using only the Rocq prelude.

   Why re-proved and not transported: the Agda → Dedukti translator
   (Deducteam/Agda2Dedukti) was last changed in January 2022. It builds
   against a forked Agda branch of that period and cannot read modules checked
   by the pinned Agda 2.6.4.3, so no transport route exists for this
   toolchain. docs/theory/PROOF-GRADES.adoc records the statement comparison.
*)

Set Implicit Arguments.

(* A recorded operation (Agda: record Op). `execute` may fail; `undo`,
   applied to the state a successful execute produced, restores the state it
   started from. *)
Record Op (S : Type) : Type := mkOp {
  execute : S -> option S;
  undo    : S -> S;
  undoes  : forall x y, execute x = Some y -> undo y = x
}.

(* The recorded list ran successfully from x to z (Agda: data Executed). *)
Inductive Executed (S : Type) : list (Op S) -> S -> S -> Type :=
  | done : forall x, Executed nil x x
  | next : forall o os x y z,
      execute o x = Some y -> Executed os y z -> Executed (cons o os) x z.

(* cmd_rollback: undo the last recorded operation first (Agda: rollback). *)
Fixpoint rollback (S : Type) (os : list (Op S)) (z : S) : S :=
  match os with
  | nil => z
  | cons o os' => undo o (rollback os' z)
  end.

(* Theorem A-1 (Agda: rollback-restores). *)
Theorem rollback_restores :
  forall (S : Type) (os : list (Op S)) (x z : S),
    Executed os x z -> rollback os z = x.
Proof.
  intros S os x z H.
  induction H as [x | o os x y z e r IH]; simpl.
  - reflexivity.
  - rewrite IH. exact (@undoes S o x y e).
Qed.

Print Assumptions rollback_restores.
