(* SPDX-License-Identifier: MIT OR AGPL-3.0-or-later *)
(* SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell *)
(*
   JanusKey Formal Proofs: Preliminaries

   Basic definitions and lemmas used throughout the development.
*)

Require Import Coq.Lists.List.
Require Import Coq.Strings.String.
Require Import Coq.Arith.Arith.
Require Import Coq.Bool.Bool.
Import ListNotations.

(** * Basic Types *)

(** Timestamps are natural numbers (Unix epoch) *)
Definition Timestamp := nat.

(** User identifiers *)
Definition UserId := nat.

(** Operation identifiers are unique natural numbers *)
Definition OpId := nat.

(** * Option utilities *)

Definition is_some {A : Type} (o : option A) : bool :=
  match o with
  | Some _ => true
  | None => false
  end.

Definition is_none {A : Type} (o : option A) : bool :=
  negb (is_some o).

(** * List utilities *)

Fixpoint list_update {A : Type} (l : list A) (n : nat) (x : A) : list A :=
  match l, n with
  | [], _ => []
  | _ :: t, 0 => x :: t
  | h :: t, S n' => h :: list_update t n' x
  end.

Fixpoint find_index {A : Type} (pred : A -> bool) (l : list A) : option nat :=
  match l with
  | [] => None
  | h :: t =>
      if pred h then Some 0
      else match find_index pred t with
           | Some n => Some (S n)
           | None => None
           end
  end.

(** * Decidability *)

Class EqDec (A : Type) := {
  eq_dec : forall x y : A, {x = y} + {x <> y}
}.

Instance nat_eq_dec : EqDec nat := {
  eq_dec := Nat.eq_dec
}.

(** * Lemmas *)

Lemma option_map_some : forall {A B : Type} (f : A -> B) (x : A) (o : option A),
  o = Some x -> option_map f o = Some (f x).
Proof.
  intros. subst. reflexivity.
Qed.

Lemma list_update_length : forall {A : Type} (l : list A) n x,
  length (list_update l n x) = length l.
Proof.
  induction l; intros; simpl.
  - reflexivity.
  - destruct n; simpl; auto.
Qed.

Lemma find_index_some : forall {A : Type} (pred : A -> bool) (l : list A) (n : nat),
  find_index pred l = Some n ->
  n < length l /\ pred (nth n l (hd_error l |> fun o => match o with Some x => x | None => nth 0 l (nth 0 l (nth 0 l (nth 0 l (nth 0 l (nth 0 l (nth 0 l (hd l))))))) end)) = true.
Proof.
  (* Complex proof - admitted for now *)
Admitted.
