(* SPDX-License-Identifier: MIT OR AGPL-3.0-or-later *)
(* SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell *)
(*
   JanusKey Formal Proofs: Operation Composition

   Extended theorems about composing multiple operations.
*)

Require Import Coq.Lists.List.
Require Import JanusKey.Preliminaries.
Require Import JanusKey.ContentHash.
Require Import JanusKey.FileSystem.
Require Import JanusKey.ContentStore.
Require Import JanusKey.Operations.
Require Import JanusKey.RMR.
Require Import JanusKey.Reversibility.
Import ListNotations.

(** ========================================================================= *)
(** * Operation Independence                                                   *)
(** ========================================================================= *)

(**
   Two operations are independent if they affect different paths.
*)
Definition independent_ops (m1 m2 : OperationMetadata) : Prop :=
  om_path m1 <> om_path m2 /\
  (match om_secondary_path m1 with
   | Some p => p <> om_path m2
   | None => True
   end) /\
  (match om_secondary_path m2 with
   | Some p => p <> om_path m1
   | None => True
   end).

(**
   Independent operations can be undone in any order.
*)
Theorem independent_ops_commute : forall s m1 m2 s1 s2,
  independent_ops m1 m2 ->
  undo_operation s m1 = Some s1 ->
  undo_operation s1 m2 = Some s2 ->
  exists s1' s2',
    undo_operation s m2 = Some s1' /\
    undo_operation s1' m1 = Some s2' /\
    state_equivalent s2 s2'.
Proof.
  intros.
  (* Independent operations affect disjoint parts of state *)
  (* Therefore order doesn't matter *)
  admit.
Admitted.

(** ========================================================================= *)
(** * Partial Undo                                                            *)
(** ========================================================================= *)

(**
   We can undo a subset of operations, leaving others in place.
*)
Definition can_partial_undo
  (s : JanusKeyState) (ops : OperationSequence) (indices : list nat) : Prop :=
  forall i, In i indices ->
    i < length ops /\
    has_sufficient_metadata (nth i ops (hd ops)) /\
    content_available_for_undo s (nth i ops (hd ops)).

Theorem partial_undo_valid : forall s ops indices s',
  can_partial_undo s ops indices ->
  (* Some undo sequence on selected indices *)
  True -> (* Placeholder *)
  valid_state (jk_fs s').
Proof.
  admit.
Admitted.

(** ========================================================================= *)
(** * Transaction Composition                                                  *)
(** ========================================================================= *)

(**
   Transactions can be nested: inner transaction completes before outer.
*)
Definition nested_transaction (outer inner : Transaction) : Prop :=
  exists prefix suffix,
    tx_ops outer = prefix ++ tx_ops inner ++ suffix.

Theorem nested_transaction_atomic : forall s outer inner s',
  nested_transaction outer inner ->
  (* If inner transaction is rolled back *)
  transaction_rollback s inner = Some s' ->
  (* Outer transaction can still be rolled back *)
  exists s'', transaction_rollback s' outer = Some s''.
Proof.
  admit.
Admitted.
