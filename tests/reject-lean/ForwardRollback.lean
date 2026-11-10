-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
-- Expected rejection, the Lean twin of tests/reject-agda/ForwardRollback.agda.
-- Two real, individually invertible operations (double, then add one) run
-- from 1 (1 -> 2 -> 3). Undoing them in EXECUTION order ends at 0, not 1, so
-- the checker must refuse the claim that it restores 1. This pins that
-- rollback_restores is not vacuous about order.
-- expect: is false

import Rollback

open JanusKey.Rollback

def rollbackForward {S : Type} : List (Op S) → S → S
  | [], z => z
  | o :: os, z => rollbackForward os (o.undo z)

def doubleOp : Op Nat where
  execute x := some (x + x)
  undo y := y / 2
  undoes := by
    intro x y h
    cases h
    omega

def incOp : Op Nat where
  execute x := some (x + 1)
  undo y := y - 1
  undoes := by
    intro x y h
    cases h
    omega

-- The correct order is a theorem (rollback_restores); the forward order is not.
example : rollbackForward [doubleOp, incOp] 3 = 1 := by decide
