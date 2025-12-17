(* SPDX-License-Identifier: MIT OR AGPL-3.0-or-later *)
(* SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell *)
(*
   JanusKey Formal Proofs: Content Store

   Content-addressed storage model with deduplication.

   LEMMA (Content Availability):
     If store(c) succeeds, then retrieve(hash(c)) = c
*)

Require Import Coq.Lists.List.
Require Import JanusKey.Preliminaries.
Require Import JanusKey.ContentHash.
Import ListNotations.

(** * Content Store State *)

Record ContentEntry := mkContentEntry {
  ce_hash : ContentHash;
  ce_content : Content;
  ce_stored : bool
}.

Definition ContentStoreState := list ContentEntry.

(** Empty content store *)
Definition empty_store : ContentStoreState := [].

(** * Store Operations *)

(** Check if content hash exists in store *)
Fixpoint content_exists (store : ContentStoreState) (h : ContentHash) : bool :=
  match store with
  | [] => false
  | e :: rest =>
      if hash_eq_dec (ce_hash e) h then
        ce_stored e
      else
        content_exists rest h
  end.

(** Retrieve content by hash *)
Fixpoint retrieve (store : ContentStoreState) (h : ContentHash) : option Content :=
  match store with
  | [] => None
  | e :: rest =>
      if hash_eq_dec (ce_hash e) h then
        if ce_stored e then Some (ce_content e) else None
      else
        retrieve rest h
  end.

(** Store content (returns updated store and hash) *)
Definition store_content (store : ContentStoreState) (c : Content)
  : ContentStoreState * ContentHash :=
  let h := hash c in
  if content_exists store h then
    (store, h)  (* Deduplication: already stored *)
  else
    let entry := {| ce_hash := h; ce_content := c; ce_stored := true |} in
    (entry :: store, h).

(** Remove content from store (for obliteration) *)
Fixpoint remove_content (store : ContentStoreState) (h : ContentHash)
  : ContentStoreState :=
  match store with
  | [] => []
  | e :: rest =>
      if hash_eq_dec (ce_hash e) h then
        {| ce_hash := ce_hash e;
           ce_content := []; (* Content zeroed *)
           ce_stored := false |} :: rest
      else
        e :: remove_content rest h
  end.

(** * Lemmas *)

(** LEMMA: Content Availability
    After storing content, it can be retrieved *)
Lemma content_availability : forall store c,
  let (store', h) := store_content store c in
  retrieve store' h = Some c.
Proof.
  intros store c.
  unfold store_content.
  destruct (content_exists store (hash c)) eqn:E.
  - (* Already stored - need to prove retrieve works *)
    (* This requires showing content_exists implies retrieve succeeds *)
    admit. (* Requires induction on store structure *)
  - (* Newly stored *)
    simpl.
    destruct (hash_eq_dec (hash c) (hash c)); try contradiction.
    reflexivity.
Admitted.

(** LEMMA: Hash Integrity
    Retrieved content has the expected hash *)
Lemma retrieve_hash_integrity : forall store h c,
  retrieve store h = Some c ->
  hash c = h.
Proof.
  induction store; intros; simpl in *.
  - discriminate.
  - destruct (hash_eq_dec (ce_hash a) h).
    + destruct (ce_stored a).
      * injection H; intros; subst.
        (* ce_content a has hash ce_hash a = h *)
        (* This requires store invariant that ce_hash = hash ce_content *)
        admit.
      * discriminate.
    + apply IHstore; auto.
Admitted.

(** LEMMA: Deduplication
    Storing the same content twice doesn't duplicate *)
Lemma store_deduplication : forall store c,
  let (store1, _) := store_content store c in
  let (store2, _) := store_content store1 c in
  length store2 = length store1.
Proof.
  intros.
  unfold store_content.
  destruct (content_exists store (hash c)) eqn:E1.
  - simpl. rewrite E1. reflexivity.
  - simpl.
    destruct (hash_eq_dec (hash c) (hash c)); try contradiction.
    reflexivity.
Qed.

(** LEMMA: Remove makes content unavailable *)
Lemma remove_makes_unavailable : forall store h,
  content_exists (remove_content store h) h = false.
Proof.
  induction store; intros; simpl.
  - reflexivity.
  - destruct (hash_eq_dec (ce_hash a) h).
    + simpl. destruct (hash_eq_dec (ce_hash a) h); try contradiction.
      reflexivity.
    + simpl. destruct (hash_eq_dec (ce_hash a) h); try contradiction.
      apply IHstore.
Qed.
