-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
-- Expected rejection, Lean twin of tests/reject-agda/SettleWithoutSettling.agda.
-- finish_good needs its settling premise. Here the "finish" transition is the
-- identity, so the record it should settle stays active: the log ends with no
-- pointer but one active record. The claim that countActive matches the
-- pointer (0) is false, and the checker must refuse it.
-- expect: is false
import SingleActiveLog

open JanusKey.SingleActiveLog

def natDec (a b : Nat) : PSum (a = b) (a ≠ b) :=
  if h : a = b then .inl h else .inr h

/-- Records are bare keys, and every record is active. -/
def after : Option (Log Nat Nat) :=
  finish natDec (fun k => k) (fun r => r) ⟨[0], some 0⟩

def countAfter : Option Nat :=
  match after with
  | none => none
  | some L => some (countActive (fun _ => Phase.active) L.txs)

example : countAfter = some 0 := by decide
