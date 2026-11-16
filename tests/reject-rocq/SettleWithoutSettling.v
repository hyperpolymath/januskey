(* SPDX-License-Identifier: MPL-2.0 *)
(* SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell *)
(* Expected rejection, Rocq twin of tests/reject-agda/SettleWithoutSettling.agda.
   finish_good needs its settling premise. Here the "finish" transition is the
   identity, so the record it should settle stays active: the log ends with no
   pointer but one active record. The claim that countActive matches the
   pointer (0) is false, and the checker must refuse it.
   expect: Unable to unify "Some 0" with "Some 1" *)
Require Import JanusKey.SingleActiveLog.

Definition nat_dec (m n : nat) : {m = n} + {m <> n}.
Proof. decide equality. Defined.

(* Records are bare keys, and every record is active. *)
Definition after : option (Log nat nat) :=
  finish nat nat nat_dec (fun k => k) (fun r => r)
    (mkLog nat nat (cons 0 nil) (Some 0)).

Definition count_after : option nat :=
  match after with
  | None => None
  | Some L => Some (countActive nat (fun _ => active) (txs nat nat L))
  end.

Example no_active_left : count_after = Some 0.
Proof. vm_compute. reflexivity. Qed.
