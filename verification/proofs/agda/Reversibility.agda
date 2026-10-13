{-# OPTIONS --safe --without-K #-}
-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
--
-- Reversibility as a grade.
--
-- januskey's operations are graded in resource-algebra's Loss instance (the
-- echo loss grade): a reversible step costs 𝟙 = fin 0, information loss is
-- fin n, obliteration is inf = 𝟘. Everything below is about the grade
-- algebra and the trace shape; the one operational fact it rests on — that
-- unwrap after wrap is the identity — is a *premise* carried by the
-- ReversiblePair record, never assumed here. In the Rust, wrap is
-- `ReversibleExecutor::execute` and unwrap is `undo`
-- (crates/reversible-core/src/lib.rs); the premise is tested by
-- crates/januskey-cli/tests/property_tests.rs::execute_then_undo_is_identity,
-- a TESTED row of the claims ledger in PROOF-NEEDS.adoc.
--
-- Pinned sibling: hyperpolymath/resource-algebra, see scripts/fetch-siblings.sh.
-- Kernel ruling (2026-10-12): Agda --safe --without-K is the proof kernel;
-- the Idris2 ABI (src/abi/JanusKey/ABI) is the runtime layer. The Idris2
-- IsCNO / ReversibleOp pair corresponds to ReversiblePair below by
-- transcription (ASSUMED; no mechanical link is claimed).

module Reversibility where

open import ResourceAlgebra.Prelude
open import ResourceAlgebra.Core
open import ResourceAlgebra.Instances.Loss

open IsSeqMonoid (ResourceAlgebra.seq Loss) renaming (unitˡ to ⊗-unitˡ; unitʳ to ⊗-unitʳ; assoc to ⊗-assoc)
open IsAnnihilating Loss-annihilating

----------------------------------------------------------------------
-- Steps and traces over an abstract state
----------------------------------------------------------------------

module Steps (S : Set) where

  -- A step: what it does to the state, and what it costs in Loss.
  record Step : Set where
    constructor step
    field
      run   : S → S
      grade : Grade

  open Step public

  -- A trace is a list of steps; its grade is the sequence product.
  data Trace : Set where
    []  : Trace
    _∷_ : Step → Trace → Trace

  infixr 5 _∷_

  traceGrade : Trace → Grade
  traceGrade []       = fin 0
  traceGrade (s ∷ ts) = grade s +g traceGrade ts

  -- Lossless: grade 𝟙.
  Lossless : Step → Set
  Lossless s = grade s ≡ fin 0

  data AllLossless : Trace → Set where
    []  : AllLossless []
    _∷_ : ∀ {s ts} → Lossless s → AllLossless ts → AllLossless (s ∷ ts)

  ----------------------------------------------------------------------
  -- Theorem 1. A trace of lossless steps is lossless (the ⊗ unit law).
  ----------------------------------------------------------------------

  all-lossless-trace : ∀ ts → AllLossless ts → traceGrade ts ≡ fin 0
  all-lossless-trace []       []       = refl
  all-lossless-trace (s ∷ ts) (p ∷ ps) =
    trans (cong (_+g traceGrade ts) p)
          (trans (⊗-unitˡ (traceGrade ts)) (all-lossless-trace ts ps))

  ----------------------------------------------------------------------
  -- Theorem 2. A lossless trace hides no lossy step: if the product is
  -- fin 0 then every factor is fin 0. This is what makes "reversible by
  -- construction" checkable — a hidden loss cannot be papered over by
  -- composition.
  ----------------------------------------------------------------------

  private
    +g-zero-left : ∀ a b → a +g b ≡ fin 0 → a ≡ fin 0
    +g-zero-left (fin zero)    (fin n) _  = refl
    +g-zero-left (fin (suc m)) (fin n) ()
    +g-zero-left (fin m)       inf     ()
    +g-zero-left inf           _       ()

    +g-zero-right : ∀ a b → a +g b ≡ fin 0 → b ≡ fin 0
    +g-zero-right (fin m)       (fin zero)    _  = refl
    +g-zero-right (fin zero)    (fin (suc n)) ()
    +g-zero-right (fin (suc m)) (fin (suc n)) ()
    +g-zero-right (fin m)       inf           ()
    +g-zero-right inf           _             ()

  lossless-trace-all : ∀ ts → traceGrade ts ≡ fin 0 → AllLossless ts
  lossless-trace-all []       _ = []
  lossless-trace-all (s ∷ ts) p =
    +g-zero-left (grade s) (traceGrade ts) p
      ∷ lossless-trace-all ts (+g-zero-right (grade s) (traceGrade ts) p)

  ----------------------------------------------------------------------
  -- Theorem 3. Obliteration absorbs: a trace containing an inf step has
  -- grade inf, whatever comes before or after (annihilation, both sides).
  ----------------------------------------------------------------------

  data Contains (s : Step) : Trace → Set where
    here  : ∀ {ts}   → Contains s (s ∷ ts)
    there : ∀ {t ts} → Contains s ts → Contains s (t ∷ ts)

  Obliterating : Step → Set
  Obliterating s = grade s ≡ inf

  obliterate-absorbs : ∀ {s} ts → Obliterating s → Contains s ts → traceGrade ts ≡ inf
  obliterate-absorbs {s} (.s ∷ ts) o here =
    trans (cong (_+g traceGrade ts) o) (annihilˡ (traceGrade ts))
  obliterate-absorbs (t ∷ ts) o (there c) =
    trans (cong (grade t +g_) (obliterate-absorbs ts o c)) (annihilʳ (grade t))

  -- And so no trace with an obliteration is lossless.
  inf≢fin0 : inf ≡ fin 0 → ⊥
  inf≢fin0 ()

  obliterated-not-lossless : ∀ {s} ts → Obliterating s → Contains s ts → traceGrade ts ≡ fin 0 → ⊥
  obliterated-not-lossless ts o c p = inf≢fin0 (trans (sym (obliterate-absorbs ts o c)) p)

  ----------------------------------------------------------------------
  -- The reversible pair: wrap then unwrap. The cancellation law is the
  -- operational premise (execute_then_undo_is_identity, cited above);
  -- given it, both steps are graded 𝟙 and the two-step trace is lossless.
  ----------------------------------------------------------------------

  record ReversiblePair : Set where
    constructor pair
    field
      wrap    : S → S
      unwrap  : S → S
      cancels : ∀ x → unwrap (wrap x) ≡ x

  open ReversiblePair public

  wrapStep : ReversiblePair → Step
  wrapStep p = step (wrap p) (fin 0)

  unwrapStep : ReversiblePair → Step
  unwrapStep p = step (unwrap p) (fin 0)

  wrap-unwrap-𝟙 : ∀ p → traceGrade (wrapStep p ∷ unwrapStep p ∷ []) ≡ fin 0
  wrap-unwrap-𝟙 p = refl

  -- Running the pair really is the identity: the grade annotation above is
  -- justified by this, not the other way round.
  wrap-unwrap-id : ∀ p x → run (unwrapStep p) (run (wrapStep p) x) ≡ x
  wrap-unwrap-id p x = cancels p x

----------------------------------------------------------------------
-- What is NOT claimed. The grade of a step is an annotation; this module
-- proves what follows from the annotations under the Loss algebra. It does
-- not derive a step's grade from its code, does not model file contents,
-- and says nothing cryptographic about obliteration. The bridge obligation
-- — that the Zig/Rust FFI wrapper of a grade-𝟙 operation is grade-𝟙 — is a
-- homomorphism statement that resource-algebra lists as OPEN; it is filed,
-- not proved here.
----------------------------------------------------------------------
