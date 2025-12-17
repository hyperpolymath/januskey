(* SPDX-License-Identifier: MIT OR AGPL-3.0-or-later *)
(* SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell *)
(*
   JanusKey Formal Proofs: Content Hash

   SHA256-based content addressing with collision resistance assumption.
*)

Require Import Coq.Strings.String.
Require Import Coq.Lists.List.
Require Import JanusKey.Preliminaries.
Import ListNotations.

(** * Content and Hash Definitions *)

(** Content is modeled as a list of bytes (nat for simplicity) *)
Definition Content := list nat.

(** A content hash is an abstract type *)
(** We model it as a natural number with an injective hash function *)
Parameter ContentHash : Type.

(** The hash function from content to hash *)
Parameter hash : Content -> ContentHash.

(** Hash equality is decidable *)
Parameter hash_eq_dec : forall h1 h2 : ContentHash, {h1 = h2} + {h1 <> h2}.

Instance content_hash_eq_dec : EqDec ContentHash := {
  eq_dec := hash_eq_dec
}.

(** * Axioms (Cryptographic Assumptions) *)

(**
   AXIOM: Collision Resistance

   We assume SHA256 is collision-resistant: different content produces
   different hashes with overwhelming probability. For formal verification,
   we model this as an axiom.
*)
Axiom hash_injective : forall c1 c2 : Content,
  hash c1 = hash c2 -> c1 = c2.

(**
   AXIOM: Hash Uniqueness (Lemma from documentation)

   Content can be uniquely identified by its hash.
*)
Lemma hash_unique : forall c : Content,
  forall h : ContentHash,
  hash c = h ->
  forall c' : Content, hash c' = h -> c' = c.
Proof.
  intros c h Hc c' Hc'.
  apply hash_injective.
  rewrite Hc, Hc'.
  reflexivity.
Qed.

(** * Verification *)

(** Verify that content matches a hash *)
Definition verify_hash (c : Content) (h : ContentHash) : bool :=
  if hash_eq_dec (hash c) h then true else false.

Lemma verify_hash_correct : forall c h,
  verify_hash c h = true <-> hash c = h.
Proof.
  intros. unfold verify_hash.
  destruct (hash_eq_dec (hash c) h); split; intros; auto.
  discriminate.
Qed.

(** * Null Hash *)

(** Empty content hash *)
Definition empty_content : Content := [].
Definition null_hash : ContentHash := hash empty_content.

(** * Hash Properties *)

Lemma hash_deterministic : forall c : Content,
  hash c = hash c.
Proof.
  reflexivity.
Qed.

Lemma hash_content_recoverable : forall c1 c2 : Content,
  c1 = c2 -> hash c1 = hash c2.
Proof.
  intros. subst. reflexivity.
Qed.
