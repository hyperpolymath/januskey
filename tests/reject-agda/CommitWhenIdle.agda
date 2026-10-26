{-# OPTIONS --safe --without-K #-}
-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
-- Expected rejection: with no active transaction, commit is an error
-- (NoActiveTransaction in the Rust), not a state change. The claim below
-- is false and the checker must refuse it; this pins the "refused in the
-- wrong state" half of the lifecycle model.
-- expect: nothing != just
module CommitWhenIdle where

open import Agda.Builtin.List using (List; []; _∷_)
open import Agda.Builtin.Maybe using (Maybe; nothing; just)
open import ResourceAlgebra.Prelude
open import TransactionAtomicity

open Lifecycle

commit-when-idle : commit (log [] nothing) ≡ just (log [] nothing)
commit-when-idle = refl
