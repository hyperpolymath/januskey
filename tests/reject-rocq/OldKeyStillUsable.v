(* SPDX-License-Identifier: MPL-2.0 *)
(* SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell *)
(* Expected rejection, Rocq twin of tests/reject-agda/OldKeyStillUsable.agda,
   the rotation half of ledger row E-1m. After rotating key 0 -> key 1 (two
   different keys) in the world x1, the claim that the owner can still use
   key 0 is false, and the checker must refuse it. The instance is closed, so
   the refusal shows the claim evaluates to false, not merely that it does
   not reduce.
   expect: Unable to unify "true" with "false" *)
Require Import JanusKey.Epistemic.

Example wrong : usable (rotate 0 1 x1) owner 0 = true.
Proof. vm_compute. reflexivity. Qed.
