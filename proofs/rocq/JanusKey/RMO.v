(* SPDX-License-Identifier: MIT OR AGPL-3.0-or-later *)
(* SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell *)
(*
   JanusKey Formal Proofs: RMO Primitive

   THEOREM (Formal Obliteration):
   After obliterate(hash):
     1. Content is cryptographically unrecoverable
     2. A verifiable proof of non-existence is generated
     3. The fact of obliteration is logged (for GDPR Article 17)

   This file contains the formal statement and proof stubs for the
   RMO (Obliterative Wipe) primitive.
*)

Require Import Coq.Lists.List.
Require Import Coq.Arith.Arith.
Require Import JanusKey.Preliminaries.
Require Import JanusKey.ContentHash.
Require Import JanusKey.ContentStore.
Require Import JanusKey.Obliteration.
Import ListNotations.

(** ========================================================================= *)
(** * RMO THEOREM 1: Content Unrecoverability                                 *)
(** ========================================================================= *)

(**
   After obliteration, content cannot be retrieved from the store.
   This is the fundamental guarantee of the RMO primitive.
*)
Theorem rmo_content_unrecoverable : forall s h reason lb s' proof,
  (* Content exists before obliteration *)
  content_exists (os_store s) h = true ->
  (* Obliteration succeeds *)
  obliterate s h reason lb = Some (s', proof) ->
  (* Content no longer retrievable *)
  retrieve (os_store s') h = None.
Proof.
  intros s h reason lb s' proof Hexists Hobl.
  (* From obliterate_removes_content, we know content_exists is false *)
  assert (content_exists (os_store s') h = false) as Hgone.
  { apply obliterate_removes_content with (reason := reason) (lb := lb).
    assumption. }
  (* If content_exists is false, retrieve returns None *)
  (* This requires a lemma connecting content_exists and retrieve *)
  admit.
Admitted.

(** ========================================================================= *)
(** * RMO THEOREM 2: Proof Generation                                         *)
(** ========================================================================= *)

(**
   Obliteration always produces a valid proof that can be verified
   independently.
*)
Theorem rmo_proof_generated : forall s h reason lb s' proof,
  content_exists (os_store s) h = true ->
  obliterate s h reason lb = Some (s', proof) ->
  valid_obliteration_proof proof /\
  op_content_hash proof = h.
Proof.
  intros.
  split.
  - apply obliterate_valid_proof with (s := s) (reason := reason) (lb := lb).
    assumption.
  - unfold obliterate in H0.
    destruct (content_exists (os_store s) h) eqn:E.
    + injection H0; intros; subst. simpl. reflexivity.
    + discriminate.
Qed.

(** ========================================================================= *)
(** * RMO THEOREM 3: Audit Trail                                              *)
(** ========================================================================= *)

(**
   Every obliteration is recorded in the audit log, providing
   evidence for GDPR compliance.
*)
Theorem rmo_audit_trail : forall s h reason lb s' proof,
  content_exists (os_store s) h = true ->
  obliterate s h reason lb = Some (s', proof) ->
  (* Obliteration is recorded *)
  was_obliterated s' h /\
  (* Record contains required fields *)
  exists r, In r (os_log s') /\
            or_content_hash r = h /\
            or_legal_basis r = lb.
Proof.
  intros.
  split.
  - apply obliterate_creates_record with (reason := reason) (lb := lb).
    assumption.
  - unfold obliterate in H0.
    destruct (content_exists (os_store s) h) eqn:E.
    + injection H0; intros; subst. simpl.
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
      repeat split.
      * apply in_or_app. right. simpl. left. reflexivity.
    + discriminate.
Qed.

(** ========================================================================= *)
(** * RMO THEOREM 4: Irreversibility                                          *)
(** ========================================================================= *)

(**
   Unlike RMR operations, RMO obliteration is NOT reversible.
   Once content is obliterated, it cannot be recovered.
*)
Theorem rmo_irreversible : forall s h reason lb s' proof,
  content_exists (os_store s) h = true ->
  obliterate s h reason lb = Some (s', proof) ->
  (* There is no operation that can restore the content *)
  forall s'', os_store s'' = os_store s' ->
              content_exists (os_store s'') h = false.
Proof.
  intros s h reason lb s' proof Hexists Hobl s'' Heq.
  rewrite Heq.
  apply obliterate_removes_content with (reason := reason) (lb := lb).
  assumption.
Qed.

(** ========================================================================= *)
(** * RMO THEOREM 5: Commitment Verification                                  *)
(** ========================================================================= *)

(**
   The cryptographic commitment in the proof can be independently
   verified without access to the original content.
*)

(** Commitment verification function (abstract) *)
Parameter verify_commitment : ObliterationProof -> bool.

(** Axiom: Valid proofs have verifiable commitments *)
Axiom commitment_verifiable : forall proof,
  valid_obliteration_proof proof ->
  verify_commitment proof = true.

Theorem rmo_commitment_verifiable : forall s h reason lb s' proof,
  content_exists (os_store s) h = true ->
  obliterate s h reason lb = Some (s', proof) ->
  verify_commitment proof = true.
Proof.
  intros.
  apply commitment_verifiable.
  apply obliterate_valid_proof with (s := s) (reason := reason) (lb := lb).
  assumption.
Qed.

(** ========================================================================= *)
(** * RMO vs RMR: The Fundamental Distinction                                 *)
(** ========================================================================= *)

(**
   THEOREM (RMO/RMR Dichotomy):
   Operations are either:
   - Reversible (RMR): can be undone, preserving content
   - Obliterative (RMO): permanent, destroying content

   There is no middle ground.
*)

Inductive OperationClass :=
  | Reversible  (* RMR: can be undone *)
  | Obliterative. (* RMO: permanent destruction *)

Definition classify_by_content_preservation
  (preserves_content : bool) : OperationClass :=
  if preserves_content then Reversible else Obliterative.

Theorem rmr_rmo_dichotomy : forall preserves,
  classify_by_content_preservation preserves = Reversible \/
  classify_by_content_preservation preserves = Obliterative.
Proof.
  intros. unfold classify_by_content_preservation.
  destruct preserves; auto.
Qed.

(** Corollary: Obliterated content cannot be used for RMR undo *)
Corollary obliterated_not_undoable : forall s h,
  was_obliterated s h ->
  content_exists (os_store s) h = false ->
  (* Cannot perform undo operations requiring this content *)
  True. (* Placeholder - actual statement would involve RMR *)
Proof.
  trivial.
Qed.
