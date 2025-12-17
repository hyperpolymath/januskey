(* SPDX-License-Identifier: MIT OR AGPL-3.0-or-later *)
(* SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell *)
(*
   JanusKey Formal Proofs: File System Model

   Abstract model of a file system state, following the formal model
   from the JanusKey documentation.

   Definition (File System State):
     S = (F, M) where
     F : Path → Content ∪ {⊥}  (file content function)
     M : Path → Metadata ∪ {⊥} (file metadata function)
*)

Require Import Coq.Strings.String.
Require Import Coq.Lists.List.
Require Import Coq.Arith.Arith.
Require Import JanusKey.Preliminaries.
Require Import JanusKey.ContentHash.
Import ListNotations.

(** * Path Definition *)

(** File paths are strings *)
Definition Path := string.

(** Path equality is decidable *)
Definition path_eq_dec := string_dec.

Instance path_eq_dec_inst : EqDec Path := {
  eq_dec := path_eq_dec
}.

(** * File Metadata *)

Record FileMetadata := mkFileMetadata {
  fm_permissions : nat;        (* Unix permissions *)
  fm_owner : nat;              (* User ID *)
  fm_group : nat;              (* Group ID *)
  fm_size : nat;               (* File size in bytes *)
  fm_modified : Timestamp;     (* Last modification time *)
  fm_is_symlink : bool;        (* Is this a symbolic link? *)
  fm_symlink_target : option Path  (* Target if symlink *)
}.

(** Default metadata *)
Definition default_metadata : FileMetadata := {|
  fm_permissions := 420; (* 0o644 *)
  fm_owner := 0;
  fm_group := 0;
  fm_size := 0;
  fm_modified := 0;
  fm_is_symlink := false;
  fm_symlink_target := None
|}.

(** * File Entry *)

Record FileEntry := mkFileEntry {
  fe_path : Path;
  fe_content : option Content;
  fe_hash : option ContentHash;
  fe_metadata : option FileMetadata;
  fe_exists : bool
}.

(** * File System State *)

(**
   The file system state is modeled as a list of file entries.
   This is a simplification - a real model might use finite maps.
*)
Definition FileSystemState := list FileEntry.

(** Empty file system *)
Definition empty_fs : FileSystemState := [].

(** * File System Operations (Queries) *)

(** Find a file entry by path *)
Fixpoint find_file (fs : FileSystemState) (p : Path) : option FileEntry :=
  match fs with
  | [] => None
  | f :: rest =>
      if path_eq_dec (fe_path f) p then
        if fe_exists f then Some f else None
      else find_file rest p
  end.

(** Check if a file exists *)
Definition file_exists (fs : FileSystemState) (p : Path) : bool :=
  match find_file fs p with
  | Some _ => true
  | None => false
  end.

(** Get file content *)
Definition get_content (fs : FileSystemState) (p : Path) : option Content :=
  match find_file fs p with
  | Some f => fe_content f
  | None => None
  end.

(** Get file hash *)
Definition get_hash (fs : FileSystemState) (p : Path) : option ContentHash :=
  match find_file fs p with
  | Some f => fe_hash f
  | None => None
  end.

(** * File System State Modification *)

(** Add or update a file *)
Fixpoint set_file (fs : FileSystemState) (f : FileEntry) : FileSystemState :=
  match fs with
  | [] => [f]
  | h :: t =>
      if path_eq_dec (fe_path h) (fe_path f) then
        f :: t
      else
        h :: set_file t f
  end.

(** Remove a file (mark as not existing) *)
Definition remove_file (fs : FileSystemState) (p : Path) : FileSystemState :=
  map (fun f =>
    if path_eq_dec (fe_path f) p then
      {| fe_path := fe_path f;
         fe_content := None;
         fe_hash := None;
         fe_metadata := None;
         fe_exists := false |}
    else f
  ) fs.

(** * State Validity *)

(** A file system state is valid if hashes match content *)
Definition valid_state (fs : FileSystemState) : Prop :=
  forall f,
    In f fs ->
    fe_exists f = true ->
    match fe_content f, fe_hash f with
    | Some c, Some h => hash c = h
    | None, None => True
    | _, _ => False
    end.

(** * Lemmas *)

Lemma find_file_exists : forall fs p f,
  find_file fs p = Some f ->
  fe_exists f = true.
Proof.
  induction fs; intros; simpl in *.
  - discriminate.
  - destruct (path_eq_dec (fe_path a) p).
    + destruct (fe_exists a) eqn:E.
      * injection H; intros; subst; auto.
      * discriminate.
    + apply IHfs; auto.
Qed.

Lemma file_exists_find : forall fs p,
  file_exists fs p = true <->
  exists f, find_file fs p = Some f.
Proof.
  intros. unfold file_exists.
  split; intros.
  - destruct (find_file fs p) eqn:E.
    + exists f; auto.
    + discriminate.
  - destruct H as [f Hf]. rewrite Hf. auto.
Qed.

Lemma set_file_preserves_others : forall fs f p,
  fe_path f <> p ->
  find_file (set_file fs f) p = find_file fs p.
Proof.
  induction fs; intros; simpl.
  - destruct (path_eq_dec (fe_path f) p); try contradiction. auto.
  - destruct (path_eq_dec (fe_path a) (fe_path f)).
    + simpl. destruct (path_eq_dec (fe_path f) p); try contradiction.
      destruct (path_eq_dec (fe_path a) p).
      * subst. contradiction.
      * auto.
    + simpl. destruct (path_eq_dec (fe_path a) p); auto.
Qed.
