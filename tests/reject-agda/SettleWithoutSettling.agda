{-# OPTIONS --safe --without-K #-}
-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
-- Expected rejection: SingleActiveLog.finish-good needs its settling premise
-- (every finish transition lands in the settled phase). Here the "finish"
-- transition is the identity, so the record it should settle stays active:
-- the log ends with no pointer but one active record. The claim that
-- countActive matches the pointer (0) is false, and the checker must refuse
-- it. This pins that at-most-one-active is not vacuous about the premise.
-- expect: 1 != 0
module SettleWithoutSettling where

open import Agda.Builtin.Equality using (_≡_; refl)
open import Agda.Builtin.List using (List; []; _∷_)
open import Agda.Builtin.Maybe using (Maybe; nothing; just)
open import Agda.Builtin.Nat using (Nat; zero; suc)
open import SingleActiveLog

-- Records are bare keys, and every record is active.
_≟_ : (m n : Nat) → (m ≡ n) ⊎ (m ≢ n)
zero  ≟ zero  = inl refl
zero  ≟ suc n = inr (λ ())
suc m ≟ zero  = inr (λ ())
suc m ≟ suc n with m ≟ n
... | inl refl = inl refl
... | inr q    = inr (λ { refl → q refl })

open Log Nat Nat _≟_ (λ k → k) (λ _ → active)

-- One record, key 0, active and pointed to; then "finish" with the identity.
after : Maybe Log
after = finish (λ r → r) (log (0 ∷ []) (just 0))

count-after : Maybe Nat
count-after with after
... | nothing = nothing
... | just L  = just (countActive (txs L))

no-active-left : count-after ≡ just 0
no-active-left = refl
