{-# OPTIONS --safe --without-K #-}
-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
--
-- Transaction atomicity, in two parts that match the two places the Rust
-- keeps it.
--
--   Part A (Rollback). `jk rollback` (crates/januskey-cli/src/main.rs,
--   cmd_rollback) undoes the recorded operations in reverse order
--   (Theorem 3.4, Sequential Reversibility). Theorem `rollback-restores`:
--   if every recorded operation executed successfully, and each one's undo
--   restores the state its execute started from, then undoing the whole
--   list in reverse order restores the state the transaction began in.
--   `rollback-restores` is the sequential generalisation of the one-step
--   premise TESTED as execute_then_undo_is_identity (cf. G-1).
--
--   Part B (Lifecycle). `TransactionManager`
--   (crates/reversible-core/src/transaction.rs) is a log of transactions
--   plus a pointer to the active one. Invariant `Good`: the pointer is set
--   exactly when one transaction is Active; every other transaction is
--   Committed or RolledBack; and the Active one is the first with its id.
--   It is preserved by begin / add_operation / commit / mark_rolled_back,
--   and those operations rewrite only the Active transaction, so a
--   completed transaction is never modified.
--
-- What is NOT claimed (the model is hand-transcribed from the Rust, like
-- G-2, and these are deliberate limits):
--   * No I/O. Saving is not modelled. Some of the Rust's failed-save
--     behaviour is TESTED, not proved (ledger rows T-5 and T-8): a
--     transaction-log mutation, or a metadata-log edit, whose save fails is
--     not applied in memory, while a metadata append whose save fails keeps
--     its record for the next save; both logs are replaced by write, sync and
--     rename. A failure after the temporary file exists is not injected. A
--     file change whose log record fails to persist is OPEN (T-9).
--   * A failed execute is assumed to leave the state unchanged. Part A only
--     speaks about operations that executed; nothing here proves the Rust
--     executor is a no-op on failure (ASSUMED, T-6).
--   * Rollback is assumed to succeed on every step. In the Rust a rollback
--     that fails part-way stops, and a retry skips the operations already
--     undone (TESTED, T-4); Part A does not model the failure or the retry.
--   * Transaction ids are assumed fresh (Fresh below). The Rust draws a
--     UUID v4; a collision would break Good, and nothing here rules it out
--     (ASSUMED, T-7).
--   * Obliterating operations have no undo, so they have no `Op` here
--     (cf. C-3); a transaction containing one is outside this theorem.
--   * Concurrency: one process, no locking, is the whole model.

module TransactionAtomicity where

open import Agda.Builtin.List using (List; []; _∷_)
open import Agda.Builtin.Maybe using (Maybe; nothing; just)
open import ResourceAlgebra.Prelude

----------------------------------------------------------------------
-- Small prelude (the kernel pins builtins only; nothing is imported)
----------------------------------------------------------------------

infixr 5 _++_
_++_ : {A : Set} → List A → List A → List A
[]       ++ ys = ys
(x ∷ xs) ++ ys = x ∷ (xs ++ ys)

data _⊎_ (A B : Set) : Set where
  inl : A → A ⊎ B
  inr : B → A ⊎ B

⊥-elim : {A : Set} → ⊥ → A
⊥-elim ()

just-inj : {A : Set} {a b : A} → just a ≡ just b → a ≡ b
just-inj refl = refl

nothing≢just : {A : Set} {a : A} → nothing ≡ just a → ⊥
nothing≢just ()

suc-inj : ∀ {m n} → suc m ≡ suc n → m ≡ n
suc-inj refl = refl

_≟_ : (m n : Nat) → (m ≡ n) ⊎ (m ≢ n)
zero  ≟ zero  = inl refl
zero  ≟ suc n = inr (λ ())
suc m ≟ zero  = inr (λ ())
suc m ≟ suc n with m ≟ n
... | inl p = inl (cong suc p)
... | inr q = inr (λ r → q (suc-inj r))

data All {A : Set} (P : A → Set) : List A → Set where
  []  : All P []
  _∷_ : ∀ {x xs} → P x → All P xs → All P (x ∷ xs)

----------------------------------------------------------------------
-- Part A. Rollback undoes the recorded operations in reverse order
----------------------------------------------------------------------

module Rollback (S : Set) where

  -- A recorded operation: `execute` may fail (nothing); `undo`, applied to
  -- the state a successful execute produced, restores the state it started
  -- from. In the Rust: OperationExecutor::execute and ::undo
  -- (crates/januskey-cli/src/operations.rs). `undoes` is the per-step
  -- premise, TESTED as execute_then_undo_is_identity; it is not proved here.
  record Op : Set where
    constructor op
    field
      execute : S → Maybe S
      undo    : S → S
      undoes  : ∀ {x y} → execute x ≡ just y → undo y ≡ x

  open Op public

  -- The recorded list `os` (execution order) ran successfully from x to z.
  data Executed : List Op → S → S → Set where
    done : ∀ {x} → Executed [] x x
    next : ∀ {o os x y z} → execute o x ≡ just y → Executed os y z → Executed (o ∷ os) x z

  -- cmd_rollback: undo the last recorded operation first, the first last.
  rollback : List Op → S → S
  rollback []       z = z
  rollback (o ∷ os) z = undo o (rollback os z)

  -- Theorem A-1 (rollback-restores). Undoing a fully executed transaction
  -- in reverse order returns the state it began in.
  rollback-restores : ∀ {os x z} → Executed os x z → rollback os z ≡ x
  rollback-restores done           = refl
  rollback-restores (next {o} e r) =
    trans (cong (undo o) (rollback-restores r)) (undoes o e)

  -- The order matters. Undoing in execution order instead is NOT a
  -- theorem; it is here only so that tests/reject-agda/ForwardRollback.agda
  -- can pin that the checker refuses the claim for it.
  rollbackForward : List Op → S → S
  rollbackForward []       z = z
  rollbackForward (o ∷ os) z = rollbackForward os (undo o z)

----------------------------------------------------------------------
-- Part B. The TransactionManager lifecycle
--
-- The invariant and its theorems are theory, proved once for every
-- single-active log in SingleActiveLog.agda (ledger rows T-2 and T-3). This
-- module is januskey's instance: a transaction is a record keyed by its id,
-- Active or settled by its state, and the four TransactionManager
-- transitions are the abstract ones at that record type. That the Rust
-- matches this instance is engineering (ledger row T-10), not proved here.
----------------------------------------------------------------------

module Lifecycle where

  import SingleActiveLog as SAL

  data TState : Set where
    Active Committed RolledBack : TState

  Active≢Committed : Active ≡ Committed → ⊥
  Active≢Committed ()

  Active≢RolledBack : Active ≡ RolledBack → ⊥
  Active≢RolledBack ()

  -- id, state, operation ids in execution order.
  record Tx : Set where
    constructor tx
    field
      txId    : Nat
      txState : TState
      txOps   : List Nat

  open Tx public

  addOpTx : Nat → Tx → Tx
  addOpTx o t = tx (txId t) (txState t) (txOps t ++ o ∷ [])

  settle : TState → Tx → Tx
  settle s t = tx (txId t) s (txOps t)

  phaseOf : TState → SAL.Phase
  phaseOf Active     = SAL.active
  phaseOf Committed  = SAL.settled
  phaseOf RolledBack = SAL.settled

  _≟ₛ_ : (m n : Nat) → (m ≡ n) SAL.⊎ (m SAL.≢ n)
  m ≟ₛ n with m ≟ n
  ... | inl p = SAL.inl p
  ... | inr q = SAL.inr (λ e → ⊥-elim (q e))

  -- `iter_mut().find(|t| t.id == i)` then rewrite: only the FIRST transaction
  -- with that id changes (SAL.Log.update).
  open SAL.Log Tx Nat _≟ₛ_ txId (λ t → phaseOf (txState t)) public
    hiding (begin-refused-while-active; finish-refused-when-idle)
    renaming (begin to openRecord; finish to settleActive;
              begin-good to openRecord-good; finish-good to settleActive-good)

  -- TransactionManager::begin(name), with the fresh id as an argument.
  begin : Nat → Log → Maybe Log
  begin i = openRecord (tx i Active [])

  -- TransactionManager::add_operation.
  addOp : Nat → Log → Maybe Log
  addOp o = modify (addOpTx o)

  -- TransactionManager::commit (s = Committed) and ::mark_rolled_back
  -- (s = RolledBack).
  finish : TState → Log → Maybe Log
  finish s = settleActive (settle s)

  commit : Log → Maybe Log
  commit = finish Committed

  markRolledBack : Log → Maybe Log
  markRolledBack = finish RolledBack

  ----------------------------------------------------------------------
  -- Theorems B-1 to B-4, as instances of SingleActiveLog
  ----------------------------------------------------------------------

  begin-refused-while-active : ∀ i ts j → begin i (log ts (just j)) ≡ nothing
  begin-refused-while-active i ts j = refl

  begin-good : ∀ {i L L'} → Good L → Fresh i (txs L) → begin i L ≡ just L' → Good L'
  begin-good g fr eq = openRecord-good refl g fr eq

  addOp-refused-when-idle : ∀ o ts → addOp o (log ts nothing) ≡ nothing
  addOp-refused-when-idle o ts = refl

  addOp-good : ∀ {o L L'} → Good L → addOp o L ≡ just L' → Good L'
  addOp-good = modify-good (λ _ → refl) (λ _ → refl)

  finish-refused-when-idle : ∀ s ts → finish s (log ts nothing) ≡ nothing
  finish-refused-when-idle s ts = refl

  settled-phase : ∀ {s} → s ≢ Active → phaseOf s ≡ SAL.settled
  settled-phase {Active}     sn = ⊥-elim (sn refl)
  settled-phase {Committed}  _  = refl
  settled-phase {RolledBack} _  = refl

  finish-good : ∀ {s L L'} → s ≢ Active → Good L → finish s L ≡ just L' → Good L'
  finish-good sn = settleActive-good (λ _ → settled-phase sn)

  commit-good : ∀ {L L'} → Good L → commit L ≡ just L' → Good L'
  commit-good = finish-good (λ e → Active≢Committed (sym e))

  markRolledBack-good : ∀ {L L'} → Good L → markRolledBack L ≡ just L' → Good L'
  markRolledBack-good = finish-good (λ e → Active≢RolledBack (sym e))

  -- Theorem B-4 (at-most-one-active) is SingleActiveLog's, at this instance.

  ----------------------------------------------------------------------
  -- Theorem B-5 (lifecycle-preserves-good): the four transitions together.
  -- One name for the claims ledger: every transition that succeeds keeps
  -- Good, begin additionally needs a fresh id, and each transition rewrites
  -- only the Active transaction, leaving every settled one untouched.
  ----------------------------------------------------------------------

  record Preserves : Set where
    field
      onBegin      : ∀ {i L L'} → Good L → Fresh i (txs L) → begin i L ≡ just L' → Good L'
      onAddOp      : ∀ {o L L'} → Good L → addOp o L ≡ just L' → Good L'
      onCommit     : ∀ {L L'} → Good L → commit L ≡ just L' → Good L'
      onRollback   : ∀ {L L'} → Good L → markRolledBack L ≡ just L' → Good L'
      onlyActive   : ∀ f pre t post
                   → SAL.All (λ u → txId u SAL.≢ txId t) pre
                   → update (txId t) f (pre SAL.++ t ∷ post) ≡ just (pre SAL.++ f t ∷ post)

  lifecycle-preserves-good : Preserves
  lifecycle-preserves-good = record
    { onBegin    = begin-good
    ; onAddOp    = addOp-good
    ; onCommit   = commit-good
    ; onRollback = markRolledBack-good
    ; onlyActive = update-active
    }
