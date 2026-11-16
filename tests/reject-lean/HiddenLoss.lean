-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
-- Expected rejection, Lean twin of tests/reject-agda/HiddenLoss.agda: a trace
-- with a lossy step cannot be certified lossless. The claim below is false
-- (fin 1 +g fin 0 = fin 1), and the checker must refuse it. The grade is read
-- back as a number (inf as 99) so that `decide` can evaluate the claim.
-- expect: is false
import Reversibility

open JanusKey.Reversibility

def lossy : Step Nat := ⟨fun x => x, .fin 1⟩

def loss : Grade → Nat
  | .fin n => n
  | .inf => 99

example : loss (traceGrade (.cons lossy .nil)) = 0 := by decide
