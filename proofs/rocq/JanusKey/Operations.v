(* SPDX-License-Identifier: MIT OR AGPL-3.0-or-later *)
(* SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell *)
(*
   JanusKey Formal Proofs: Operations

   Definition of file operations and their metadata requirements.
*)

Require Import Coq.Strings.String.
Require Import Coq.Lists.List.
Require Import JanusKey.Preliminaries.
Require Import JanusKey.ContentHash.
Require Import JanusKey.FileSystem.
Require Import JanusKey.ContentStore.
Import ListNotations.

(** * Operation Types *)

Inductive OperationType :=
  | OpCreate
  | OpDelete
  | OpModify
  | OpMove
  | OpCopy
  | OpChmod
  | OpMkdir
  | OpRmdir
  | OpSymlink
  | OpAppend
  | OpTruncate
  | OpTouch.

(** * Operation Metadata *)

(**
   DEFINITION (Operation Metadata):
   Complete information needed to reverse an operation.
*)
Record OperationMetadata := mkOpMeta {
  om_id : OpId;
  om_type : OperationType;
  om_timestamp : Timestamp;
  om_path : Path;
  om_secondary_path : option Path;     (* For Move/Copy *)
  om_content_hash : option ContentHash; (* Original content *)
  om_new_content_hash : option ContentHash; (* New content for Modify *)
  om_original_metadata : option FileMetadata;
  om_original_size : option nat;       (* For Append/Truncate *)
  om_is_undone : bool
}.

(** * Metadata Sufficiency Predicate *)

(**
   LEMMA (Metadata Sufficiency):
   For each operation type, the metadata contains all information
   needed for reversal.
*)
Definition has_sufficient_metadata (m : OperationMetadata) : Prop :=
  match om_type m with
  | OpDelete =>
      (* Need original content hash to restore *)
      is_some (om_content_hash m) = true
  | OpModify =>
      (* Need both original and new content hashes *)
      is_some (om_content_hash m) = true /\
      is_some (om_new_content_hash m) = true
  | OpMove | OpCopy =>
      (* Need secondary path *)
      is_some (om_secondary_path m) = true
  | OpAppend =>
      (* Need original size for truncation *)
      is_some (om_original_size m) = true
  | OpTruncate =>
      (* Need original content to restore *)
      is_some (om_content_hash m) = true
  | _ => True
  end.

(** * Combined State *)

(**
   The complete JanusKey state includes:
   - File system state
   - Content store state
   - Operation history
*)
Record JanusKeyState := mkJKState {
  jk_fs : FileSystemState;
  jk_store : ContentStoreState;
  jk_history : list OperationMetadata
}.

(** * Operation Application *)

(**
   Apply an operation to the state, returning new state and metadata.
*)

Definition apply_delete (s : JanusKeyState) (p : Path)
  : option (JanusKeyState * OperationMetadata) :=
  match find_file (jk_fs s) p with
  | None => None  (* File doesn't exist *)
  | Some f =>
      match fe_content f, fe_hash f with
      | Some c, Some h =>
          let (store', _) := store_content (jk_store s) c in
          let fs' := remove_file (jk_fs s) p in
          let meta := {|
            om_id := length (jk_history s);
            om_type := OpDelete;
            om_timestamp := 0; (* Would be current time *)
            om_path := p;
            om_secondary_path := None;
            om_content_hash := Some h;
            om_new_content_hash := None;
            om_original_metadata := fe_metadata f;
            om_original_size := None;
            om_is_undone := false
          |} in
          let s' := {|
            jk_fs := fs';
            jk_store := store';
            jk_history := jk_history s ++ [meta]
          |} in
          Some (s', meta)
      | _, _ => None
      end
  end.

Definition apply_create (s : JanusKeyState) (p : Path) (c : Content)
  : option (JanusKeyState * OperationMetadata) :=
  if file_exists (jk_fs s) p then
    None  (* File already exists *)
  else
    let h := hash c in
    let f := {|
      fe_path := p;
      fe_content := Some c;
      fe_hash := Some h;
      fe_metadata := Some default_metadata;
      fe_exists := true
    |} in
    let fs' := set_file (jk_fs s) f in
    let meta := {|
      om_id := length (jk_history s);
      om_type := OpCreate;
      om_timestamp := 0;
      om_path := p;
      om_secondary_path := None;
      om_content_hash := None;
      om_new_content_hash := Some h;
      om_original_metadata := None;
      om_original_size := None;
      om_is_undone := false
    |} in
    let s' := {|
      jk_fs := fs';
      jk_store := jk_store s;
      jk_history := jk_history s ++ [meta]
    |} in
    Some (s', meta).

Definition apply_modify (s : JanusKeyState) (p : Path) (new_c : Content)
  : option (JanusKeyState * OperationMetadata) :=
  match find_file (jk_fs s) p with
  | None => None
  | Some f =>
      match fe_content f, fe_hash f with
      | Some old_c, Some old_h =>
          let (store', _) := store_content (jk_store s) old_c in
          let new_h := hash new_c in
          let f' := {|
            fe_path := p;
            fe_content := Some new_c;
            fe_hash := Some new_h;
            fe_metadata := fe_metadata f;
            fe_exists := true
          |} in
          let fs' := set_file (jk_fs s) f' in
          let meta := {|
            om_id := length (jk_history s);
            om_type := OpModify;
            om_timestamp := 0;
            om_path := p;
            om_secondary_path := None;
            om_content_hash := Some old_h;
            om_new_content_hash := Some new_h;
            om_original_metadata := fe_metadata f;
            om_original_size := None;
            om_is_undone := false
          |} in
          let s' := {|
            jk_fs := fs';
            jk_store := store';
            jk_history := jk_history s ++ [meta]
          |} in
          Some (s', meta)
      | _, _ => None
      end
  end.

Definition apply_move (s : JanusKeyState) (src dst : Path)
  : option (JanusKeyState * OperationMetadata) :=
  if file_exists (jk_fs s) dst then
    None  (* Destination exists *)
  else
    match find_file (jk_fs s) src with
    | None => None
    | Some f =>
        let f_dst := {|
          fe_path := dst;
          fe_content := fe_content f;
          fe_hash := fe_hash f;
          fe_metadata := fe_metadata f;
          fe_exists := true
        |} in
        let fs' := set_file (remove_file (jk_fs s) src) f_dst in
        let meta := {|
          om_id := length (jk_history s);
          om_type := OpMove;
          om_timestamp := 0;
          om_path := src;
          om_secondary_path := Some dst;
          om_content_hash := None;
          om_new_content_hash := None;
          om_original_metadata := fe_metadata f;
          om_original_size := None;
          om_is_undone := false
        |} in
        let s' := {|
          jk_fs := fs';
          jk_store := jk_store s;
          jk_history := jk_history s ++ [meta]
        |} in
        Some (s', meta)
    end.

(** * Operation Reversal *)

Definition undo_delete (s : JanusKeyState) (m : OperationMetadata)
  : option JanusKeyState :=
  match om_content_hash m with
  | None => None
  | Some h =>
      match retrieve (jk_store s) h with
      | None => None  (* Content not available! *)
      | Some c =>
          let f := {|
            fe_path := om_path m;
            fe_content := Some c;
            fe_hash := Some h;
            fe_metadata := om_original_metadata m;
            fe_exists := true
          |} in
          Some {|
            jk_fs := set_file (jk_fs s) f;
            jk_store := jk_store s;
            jk_history := jk_history s
          |}
      end
  end.

Definition undo_create (s : JanusKeyState) (m : OperationMetadata)
  : option JanusKeyState :=
  Some {|
    jk_fs := remove_file (jk_fs s) (om_path m);
    jk_store := jk_store s;
    jk_history := jk_history s
  |}.

Definition undo_modify (s : JanusKeyState) (m : OperationMetadata)
  : option JanusKeyState :=
  match om_content_hash m with
  | None => None
  | Some h =>
      match retrieve (jk_store s) h with
      | None => None
      | Some c =>
          match find_file (jk_fs s) (om_path m) with
          | None => None
          | Some f =>
              let f' := {|
                fe_path := om_path m;
                fe_content := Some c;
                fe_hash := Some h;
                fe_metadata := om_original_metadata m;
                fe_exists := true
              |} in
              Some {|
                jk_fs := set_file (jk_fs s) f';
                jk_store := jk_store s;
                jk_history := jk_history s
              |}
          end
      end
  end.

Definition undo_move (s : JanusKeyState) (m : OperationMetadata)
  : option JanusKeyState :=
  match om_secondary_path m with
  | None => None
  | Some dst =>
      match find_file (jk_fs s) dst with
      | None => None
      | Some f =>
          let f_src := {|
            fe_path := om_path m;
            fe_content := fe_content f;
            fe_hash := fe_hash f;
            fe_metadata := fe_metadata f;
            fe_exists := true
          |} in
          let fs' := set_file (remove_file (jk_fs s) dst) f_src in
          Some {|
            jk_fs := fs';
            jk_store := jk_store s;
            jk_history := jk_history s
          |}
      end
  end.

(** General undo dispatcher *)
Definition undo_operation (s : JanusKeyState) (m : OperationMetadata)
  : option JanusKeyState :=
  match om_type m with
  | OpDelete => undo_delete s m
  | OpCreate => undo_create s m
  | OpModify => undo_modify s m
  | OpMove => undo_move s m
  | _ => None (* Not yet implemented *)
  end.
