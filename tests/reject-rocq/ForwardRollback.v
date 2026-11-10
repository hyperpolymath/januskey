(* SPDX-License-Identifier: MPL-2.0 *)
(* SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell *)
(* Expected rejection, the Rocq twin of tests/reject-agda/ForwardRollback.agda.
   Two real, individually invertible operations (double, then add one) run
   from 1 (1 -> 2 -> 3). Undoing them in EXECUTION order ends at 0, not 1, so
   the checker must refuse the claim that it restores 1. This pins that
   rollback_restores is not vacuous about order.
   expect: Unable to unify "1" with "0" *)

From JanusKey Require Import Rollback.
Require Import Arith Lia.

Fixpoint rollbackForward {S : Type} (os : list (Op S)) (z : S) : S :=
  match os with
  | nil => z
  | cons o os' => rollbackForward os' (undo o z)
  end.

Definition doubleOp : Op nat.
Proof.
  refine (mkOp (fun x => Some (x + x)) Nat.div2 _).
  intros x y H. injection H as <-.
  replace (x + x) with (2 * x) by lia. apply Nat.div2_double.
Defined.

Definition incOp : Op nat.
Proof.
  refine (mkOp (fun x => Some (S x)) pred _).
  intros x y H. injection H as <-. reflexivity.
Defined.

(* The correct order is a theorem (rollback_restores); the forward order is not. *)
Example forward_restores :
  rollbackForward (cons doubleOp (cons incOp nil)) 3 = 1.
Proof. vm_compute. reflexivity. Qed.
