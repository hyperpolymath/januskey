{-# OPTIONS --safe --without-K #-}
-- SPDX-License-Identifier: MPL-2.0
-- expect: 2 != 1
--
-- Ledger row E-1m: "the two worlds hold the same thing". Epistemic.x₁ and
-- Epistemic.x₂ differ only in what the store holds, and the epistemic model is
-- built so the adversary cannot tell them apart — which is exactly why the
-- stores cannot be identified. This fixture pins that the checker refuses it.
module AdversaryKnows where

open import Agda.Builtin.Equality using (_≡_; refl)
open import Epistemic

wrong : World.content x₂ ≡ World.content x₁
wrong = refl
