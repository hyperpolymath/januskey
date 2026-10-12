{-# OPTIONS --safe --without-K #-}
-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
-- Expected rejection: nothing after an obliterating step restores a lossless
-- grade. The claim below is false (inf +g fin 0 = inf) and must be refused.
module ReviveAfterObliterate where

open import ResourceAlgebra.Prelude
open import ResourceAlgebra.Instances.Loss
open import Reversibility

open Steps Nat

obliterate : Step
obliterate = step (λ x → 0) inf

restore : Step
restore = step (λ x → x) (fin 0)

revived : traceGrade (obliterate ∷ restore ∷ []) ≡ fin 0
revived = refl
