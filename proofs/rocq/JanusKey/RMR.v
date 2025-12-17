(* SPDX-License-Identifier: MIT OR AGPL-3.0-or-later *)
(* SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell *)
(*
   JanusKey Formal Proofs: RMR Primitive

   THEOREM 1 (Individual Operation Reversibility):
   For any operation op in {delete, modify, move, copy, chmod, ...}
   and valid state S:
     reverse(apply(op, S)) = S

   This file contains the formal statement and proof stubs for the
   RMR (Reversible Transaction) primitive.
*)

Require Import Coq.Strings.String.
Require Import Coq.Lists.List.
Require Import JanusKey.Preliminaries.
Require Import JanusKey.ContentHash.
Require Import JanusKey.FileSystem.
Require Import JanusKey.ContentStore.
Require Import JanusKey.Operations.
Import ListNotations.

(** * State Equivalence *)

(**
   Two states are equivalent if their file systems have the same
   content at all paths. We ignore history for equivalence.
*)
Definition fs_equivalent (fs1 fs2 : FileSystemState) : Prop :=
  forall p : Path,
    get_content fs1 p = get_content fs2 p /\
    get_hash fs1 p = get_hash fs2 p.

Definition state_equivalent (s1 s2 : JanusKeyState) : Prop :=
  fs_equivalent (jk_fs s1) (jk_fs s2).

(** * Preconditions for Reversibility *)

(**
   Content must be available in the store for operations that need it.
*)
Definition content_available_for_undo
  (s : JanusKeyState) (m : OperationMetadata) : Prop :=
  match om_type m with
  | OpDelete | OpModify | OpTruncate =>
      match om_content_hash m with
      | Some h => content_exists (jk_store s) h = true
      | None => False
      end
  | _ => True
  end.

(** ========================================================================= *)
(** * THEOREM 1: Individual Operation Reversibility                           *)
(** ========================================================================= *)

(**
   THEOREM (Delete Reversibility):
   Deleting a file and then undoing restores the original state.
*)
Theorem delete_reversible : forall s p s' m s'',
  (* Preconditions *)
  valid_state (jk_fs s) ->
  file_exists (jk_fs s) p = true ->
  (* Apply delete *)
  apply_delete s p = Some (s', m) ->
  (* Content is available *)
  content_available_for_undo s' m ->
  (* Undo delete *)
  undo_delete s' m = Some s'' ->
  (* Postcondition: file system restored *)
  state_equivalent s s''.
Proof.
  intros s p s' m s'' Hvalid Hexists Happly Havail Hundo.
  unfold state_equivalent, fs_equivalent.
  intros p'.
  (* The proof proceeds by case analysis on whether p' = p *)
  (* If p' = p: the file was deleted then restored *)
  (* If p' <> p: the file was unchanged *)
  destruct (path_eq_dec p' p).
  - (* p' = p: the deleted file *)
    subst p'.
    (* After delete, file doesn't exist *)
    (* After undo, file is restored with original content *)
    (* Need to show get_content s'' p = get_content s p *)
    admit. (* Requires detailed analysis of apply_delete and undo_delete *)
  - (* p' <> p: unaffected file *)
    (* Show that apply_delete and undo_delete don't change other files *)
    admit. (* Requires lemmas about remove_file and set_file *)
Admitted.

(**
   THEOREM (Create Reversibility):
   Creating a file and then undoing removes it.
*)
Theorem create_reversible : forall s p c s' m s'',
  valid_state (jk_fs s) ->
  file_exists (jk_fs s) p = false ->
  apply_create s p c = Some (s', m) ->
  undo_create s' m = Some s'' ->
  state_equivalent s s''.
Proof.
  intros.
  unfold state_equivalent, fs_equivalent.
  intros p'.
  destruct (path_eq_dec p' p).
  - subst. admit.
  - admit.
Admitted.

(**
   THEOREM (Modify Reversibility):
   Modifying a file and then undoing restores original content.
*)
Theorem modify_reversible : forall s p new_c s' m s'',
  valid_state (jk_fs s) ->
  file_exists (jk_fs s) p = true ->
  apply_modify s p new_c = Some (s', m) ->
  content_available_for_undo s' m ->
  undo_modify s' m = Some s'' ->
  state_equivalent s s''.
Proof.
  intros.
  unfold state_equivalent, fs_equivalent.
  intros p'.
  destruct (path_eq_dec p' p).
  - subst. admit.
  - admit.
Admitted.

(**
   THEOREM (Move Reversibility):
   Moving a file and then undoing restores original location.
*)
Theorem move_reversible : forall s src dst s' m s'',
  valid_state (jk_fs s) ->
  file_exists (jk_fs s) src = true ->
  file_exists (jk_fs s) dst = false ->
  apply_move s src dst = Some (s', m) ->
  undo_move s' m = Some s'' ->
  state_equivalent s s''.
Proof.
  intros.
  unfold state_equivalent, fs_equivalent.
  intros p'.
  destruct (path_eq_dec p' src); destruct (path_eq_dec p' dst).
  - (* p' = src = dst - contradiction since src exists, dst doesn't *)
    admit.
  - (* p' = src *)
    admit.
  - (* p' = dst *)
    admit.
  - (* p' <> src, p' <> dst *)
    admit.
Admitted.

(** ========================================================================= *)
(** * THEOREM 2: General Reversibility                                        *)
(** ========================================================================= *)

(**
   THEOREM (General Individual Reversibility):
   Any operation with sufficient metadata can be reversed.
*)
Theorem individual_reversibility : forall s m s' s'',
  valid_state (jk_fs s) ->
  has_sufficient_metadata m ->
  content_available_for_undo s m ->
  (* There exists some operation that produced m from s to s' *)
  (* and undo_operation succeeds *)
  undo_operation s' m = Some s'' ->
  (* Then s'' is equivalent to the state before the operation *)
  state_equivalent s s''.
Proof.
  intros s m s' s'' Hvalid Hsuff Havail Hundo.
  destruct (om_type m) eqn:Htype.
  - (* OpCreate *)
    admit.
  - (* OpDelete *)
    admit.
  - (* OpModify *)
    admit.
  - (* OpMove *)
    admit.
  - (* OpCopy *)
    admit.
  - (* OpChmod *)
    admit.
  - (* OpMkdir *)
    admit.
  - (* OpRmdir *)
    admit.
  - (* OpSymlink *)
    admit.
  - (* OpAppend *)
    admit.
  - (* OpTruncate *)
    admit.
  - (* OpTouch *)
    admit.
Admitted.
