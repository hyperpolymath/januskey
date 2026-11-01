{-# OPTIONS --safe --without-K #-}
-- SPDX-License-Identifier: MPL-2.0
-- expect: != true of type Agda.Builtin.Bool.Bool
--
-- Ledger row E-1m, the rotation half: "after rotating old -> new the old key is
-- still usable". Epistemic.rotate-forgets-old says it is not. This fixture pins
-- that the checker refuses the claim.
module OldKeyStillUsable where

open import Agda.Builtin.Bool using (true; false)
open import Agda.Builtin.Equality using (_≡_; refl)
open import Epistemic

wrong : ∀ a old new x → (old == new) ≡ false →
        World.usable (rotate old new x) a old ≡ true
wrong a old new x neq = refl
