{-# OPTIONS --safe --without-K #-}
-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
-- Expected rejection: a trace with a lossy step cannot be certified lossless.
-- The claim below is false (fin 1 +g fin 0 = fin 1), and the checker must
-- refuse it; this fixture pins that Theorem 2 is not vacuous.
module HiddenLoss where

open import ResourceAlgebra.Prelude
open import ResourceAlgebra.Instances.Loss
open import Reversibility

open Steps Nat

lossy : Step
lossy = step (λ x → x) (fin 1)

hidden : traceGrade (lossy ∷ []) ≡ fin 0
hidden = refl
