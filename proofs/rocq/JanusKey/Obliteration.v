(* SPDX-License-Identifier: MIT OR AGPL-3.0-or-later *)
(* SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell *)
(*
   JanusKey Formal Proofs: Obliteration Model

   The RMO (Obliterative Wipe) primitive for GDPR Article 17 compliance.
*)

Require Import Coq.Lists.List.
Require Import Coq.Arith.Arith.
Require Import JanusKey.Preliminaries.
Require Import JanusKey.ContentHash.
Require Import JanusKey.ContentStore.
Import ListNotations.

(** * Obliteration Proof Structure *)

(**
   An obliteration proof demonstrates that:
   1. Content was securely overwritten
   2. A cryptographic commitment exists
   3. The timestamp of obliteration is recorded
*)
Record ObliterationProof := mkOblitProof {
  op_content_hash : ContentHash;      (* Hash of obliterated content *)
  op_timestamp : Timestamp;           (* When obliteration occurred *)
  op_nonce : nat;                     (* Random nonce for commitment *)
  op_commitment : ContentHash;        (* H(hash || nonce || timestamp) *)
  op_overwrite_passes : nat;          (* Number of secure overwrite passes *)
  op_storage_cleared : bool           (* Whether storage was cleared *)
}.

(** Minimum passes for secure deletion (DoD 5220.22-M) *)
Definition min_overwrite_passes : nat := 3.

(** Valid obliteration proof predicate *)
Definition valid_obliteration_proof (p : ObliterationProof) : Prop :=
  op_storage_cleared p = true /\
  op_overwrite_passes p >= min_overwrite_passes.

(** * Obliteration Record (Audit Log Entry) *)

Record ObliterationRecord := mkOblitRecord {
  or_id : nat;
  or_content_hash : ContentHash;
  or_timestamp : Timestamp;
  or_reason : nat;              (* Encoded reason *)
  or_legal_basis : nat;         (* Encoded legal basis, e.g., GDPR Art. 17 *)
  or_proof : ObliterationProof
}.

(** * Obliteration State *)

Record ObliterationState := mkOblitState {
  os_store : ContentStoreState;
  os_log : list ObliterationRecord
}.

(** Check if content was obliterated *)
Definition was_obliterated (s : ObliterationState) (h : ContentHash) : Prop :=
  exists r, In r (os_log s) /\ or_content_hash r = h.

(** * Secure Overwrite Model *)

(**
   We model secure overwrite as a function that:
   1. Overwrites content with patterns
   2. Returns proof of overwrite

   The actual cryptographic properties are axiomatized.
*)

(** Axiom: After secure overwrite, original content is unrecoverable *)
Axiom secure_overwrite_unrecoverable : forall store h passes,
  passes >= min_overwrite_passes ->
  content_exists store h = true ->
  content_exists (remove_content store h) h = false.

(** * Obliterate Operation *)

Definition obliterate (s : ObliterationState) (h : ContentHash) (reason legal_basis : nat)
  : option (ObliterationState * ObliterationProof) :=
  if content_exists (os_store s) h then
    let proof := {|
      op_content_hash := h;
      op_timestamp := 0; (* Would be current time *)
      op_nonce := 0;     (* Would be random *)
      op_commitment := h; (* Would be H(h || nonce || ts) *)
      op_overwrite_passes := min_overwrite_passes;
      op_storage_cleared := true
    |} in
    let record := {|
      or_id := length (os_log s);
      or_content_hash := h;
      or_timestamp := 0;
      or_reason := reason;
      or_legal_basis := legal_basis;
      or_proof := proof
    |} in
    let s' := {|
      os_store := remove_content (os_store s) h;
      os_log := os_log s ++ [record]
    |} in
    Some (s', proof)
  else
    None.

(** * Key Lemmas *)

Lemma obliterate_removes_content : forall s h reason lb s' proof,
  obliterate s h reason lb = Some (s', proof) ->
  content_exists (os_store s') h = false.
Proof.
  intros.
  unfold obliterate in H.
  destruct (content_exists (os_store s) h) eqn:E.
  - injection H; intros; subst.
    simpl.
    apply remove_makes_unavailable.
  - discriminate.
Qed.

Lemma obliterate_creates_record : forall s h reason lb s' proof,
  obliterate s h reason lb = Some (s', proof) ->
  was_obliterated s' h.
Proof.
  intros.
  unfold obliterate in H.
  destruct (content_exists (os_store s) h) eqn:E.
  - injection H; intros; subst.
    unfold was_obliterated. simpl.
    exists {|
      or_id := length (os_log s);
      or_content_hash := h;
      or_timestamp := 0;
      or_reason := reason;
      or_legal_basis := lb;
      or_proof := {|
        op_content_hash := h;
        op_timestamp := 0;
        op_nonce := 0;
        op_commitment := h;
        op_overwrite_passes := min_overwrite_passes;
        op_storage_cleared := true
      |}
    |}.
    split.
    + apply in_or_app. right. simpl. left. reflexivity.
    + reflexivity.
  - discriminate.
Qed.

Lemma obliterate_valid_proof : forall s h reason lb s' proof,
  obliterate s h reason lb = Some (s', proof) ->
  valid_obliteration_proof proof.
Proof.
  intros.
  unfold obliterate in H.
  destruct (content_exists (os_store s) h) eqn:E.
  - injection H; intros; subst.
    unfold valid_obliteration_proof. simpl.
    split.
    + reflexivity.
    + unfold min_overwrite_passes. lia.
  - discriminate.
Qed.
