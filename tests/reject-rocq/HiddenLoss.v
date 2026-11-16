(* SPDX-License-Identifier: MPL-2.0 *)
(* SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell *)
(* Expected rejection, Rocq twin of tests/reject-agda/HiddenLoss.agda: a trace
   with a lossy step cannot be certified lossless. The claim below is false
   (fin 1 +g fin 0 = fin 1), and the checker must refuse it.
   expect: Unable to unify "fin 0" with "fin 1" *)
Require Import JanusKey.Reversibility.

Definition lossy : Step nat := step nat (fun x => x) (fin 1).

Example hidden : traceGrade nat (tcons nat lossy (tnil nat)) = fin 0.
Proof. vm_compute. reflexivity. Qed.
