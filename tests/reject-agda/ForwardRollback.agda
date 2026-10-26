{-# OPTIONS --safe --without-K #-}
-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
-- Expected rejection: rollback must undo in reverse execution order. Two
-- real, individually invertible operations (double, then add one) are
-- executed from 1 (1 -> 2 -> 3); undoing them in EXECUTION order instead
-- ends at 0, not 1, so the checker must refuse the claim that it restores 1.
-- This pins that `rollback-restores` is not vacuous about order.
-- expect: 0 != 1
module ForwardRollback where

open import Agda.Builtin.List using (List; []; _∷_)
open import Agda.Builtin.Maybe using (Maybe; nothing; just)
open import ResourceAlgebra.Prelude
open import TransactionAtomicity

open Rollback Nat

half : Nat → Nat
half zero          = zero
half (suc zero)    = zero
half (suc (suc n)) = suc (half n)

half-double : ∀ x → half (x + x) ≡ x
half-double zero    = refl
half-double (suc y) rewrite +-suc y y = cong suc (half-double y)

pred : Nat → Nat
pred zero    = zero
pred (suc n) = n

doubleOp : Op
doubleOp = op (λ x → just (x + x)) half
              (λ {x} e → trans (cong half (sym (just-inj e))) (half-double x))

incOp : Op
incOp = op (λ x → just (suc x)) pred
           (λ {x} e → cong pred (sym (just-inj e)))

-- The correct order is a theorem (rollback-restores); the forward order is not.
forward-restores : rollbackForward (doubleOp ∷ incOp ∷ []) 3 ≡ 1
forward-restores = refl
