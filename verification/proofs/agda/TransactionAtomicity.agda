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
--   * No I/O. The Rust mutates the in-memory log and then calls save()?;
--     a save failure after the mutation leaves memory and disk disagreeing.
--     fs::write is not an atomic replace, so a crash mid-write can corrupt
--     the log. Neither is modelled.
--   * A failed execute is assumed to leave the state unchanged. Part A only
--     speaks about operations that executed; nothing here proves the Rust
--     executor is a no-op on failure.
--   * Rollback is assumed to succeed on every step. cmd_rollback stops at
--     the first undo error (`?`) and leaves the transaction Active with the
--     later operations already undone; a retry then meets "already undone".
--     Part A does not cover that case; see the OPEN ledger rows.
--   * Transaction ids are assumed fresh (Fresh below). The Rust draws a
--     UUID v4; a collision would break Good, and nothing here rules it out.
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
----------------------------------------------------------------------

module Lifecycle where

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

  -- The log and the pointer to the active transaction.
  record Log : Set where
    constructor log
    field
      txs      : List Tx
      activeId : Maybe Nat

  open Log public

  Settled : Tx → Set
  Settled t = txState t ≢ Active

  addOpTx : Nat → Tx → Tx
  addOpTx o t = tx (txId t) (txState t) (txOps t ++ o ∷ [])

  settle : TState → Tx → Tx
  settle s t = tx (txId t) s (txOps t)

  -- `iter_mut().find(|t| t.id == i)` then rewrite: only the FIRST transaction
  -- with that id changes. nothing = no such transaction.
  update : Nat → (Tx → Tx) → List Tx → Maybe (List Tx)
  update i f []       = nothing
  update i f (t ∷ ts) with txId t ≟ i
  ... | inl _ = just (f t ∷ ts)
  ... | inr _ with update i f ts
  ...   | nothing = nothing
  ...   | just us = just (t ∷ us)

  -- TransactionManager::begin(name), with the fresh id as an argument.
  begin : Nat → Log → Maybe Log
  begin i (log ts (just _)) = nothing
  begin i (log ts nothing)  = just (log (ts ++ tx i Active [] ∷ []) (just i))

  -- TransactionManager::add_operation.
  addOp : Nat → Log → Maybe Log
  addOp o (log ts nothing)  = nothing
  addOp o (log ts (just i)) with update i (addOpTx o) ts
  ... | nothing = nothing
  ... | just us = just (log us (just i))

  -- TransactionManager::commit (s = Committed) and ::mark_rolled_back
  -- (s = RolledBack).
  finish : TState → Log → Maybe Log
  finish s (log ts nothing)  = nothing
  finish s (log ts (just i)) with update i (settle s) ts
  ... | nothing = nothing
  ... | just us = just (log us nothing)

  commit : Log → Maybe Log
  commit = finish Committed

  markRolledBack : Log → Maybe Log
  markRolledBack = finish RolledBack

  ----------------------------------------------------------------------
  -- The invariant
  ----------------------------------------------------------------------

  data Good : Log → Set where
    -- No pointer: nothing is Active.
    idle : ∀ {ts} → All Settled ts → Good (log ts nothing)
    -- Pointer to t: t is the first with its id, it is the one Active
    -- transaction, and everything else is Committed or RolledBack.
    busy : ∀ {pre t post}
         → All (λ u → txId u ≢ txId t) pre
         → All Settled pre
         → txState t ≡ Active
         → All Settled post
         → Good (log (pre ++ t ∷ post) (just (txId t)))

  Fresh : Nat → List Tx → Set
  Fresh i = All (λ u → txId u ≢ i)

  good-empty : Good (log [] nothing)
  good-empty = idle []

  ----------------------------------------------------------------------
  -- Only the Active transaction is rewritten (completed ones are frozen)
  ----------------------------------------------------------------------

  update-active : ∀ f pre t post
                → All (λ u → txId u ≢ txId t) pre
                → update (txId t) f (pre ++ t ∷ post) ≡ just (pre ++ f t ∷ post)
  update-active f []        t post []       with txId t ≟ txId t
  ... | inl _ = refl
  ... | inr n = ⊥-elim (n refl)
  update-active f (u ∷ pre) t post (n ∷ ns) with txId u ≟ txId t
  ... | inl e = ⊥-elim (n e)
  ... | inr _ with update (txId t) f (pre ++ t ∷ post) | update-active f pre t post ns
  ...   | .(just (pre ++ f t ∷ post)) | refl = refl

  ----------------------------------------------------------------------
  -- Settled transactions append and concatenate without becoming Active
  ----------------------------------------------------------------------

  all-++ : ∀ {A : Set} {P : A → Set} {xs ys}
         → All P xs → All P ys → All P (xs ++ ys)
  all-++ []       qs = qs
  all-++ (p ∷ ps) qs = p ∷ all-++ ps qs

  ----------------------------------------------------------------------
  -- Theorem B-1: begin preserves Good (given a fresh id), and is refused
  -- while a transaction is active.
  ----------------------------------------------------------------------

  begin-refused-while-active : ∀ i ts j → begin i (log ts (just j)) ≡ nothing
  begin-refused-while-active i ts j = refl

  begin-good : ∀ {i L L'} → Good L → Fresh i (txs L) → begin i L ≡ just L' → Good L'
  begin-good (busy _ _ _ _) _ ()
  begin-good {i} (idle {ts} ss) fr refl =
    busy {pre = ts} {t = tx i Active []} {post = []} fr ss refl []

  ----------------------------------------------------------------------
  -- Theorem B-2: add_operation preserves Good, and is refused when idle.
  ----------------------------------------------------------------------

  addOp-refused-when-idle : ∀ o ts → addOp o (log ts nothing) ≡ nothing
  addOp-refused-when-idle o ts = refl

  addOp-good : ∀ {o L L'} → Good L → addOp o L ≡ just L' → Good L'
  addOp-good (idle _) ()
  addOp-good {o} (busy {pre} {t} {post} np sp act sq) eq
    with update (txId t) (addOpTx o) (pre ++ t ∷ post) | update-active (addOpTx o) pre t post np
  ... | .(just (pre ++ addOpTx o t ∷ post)) | refl with eq
  ... | refl = busy {pre = pre} {t = addOpTx o t} {post = post} np sp act sq

  ----------------------------------------------------------------------
  -- Theorem B-3: commit and mark_rolled_back preserve Good, land idle,
  -- and are refused when idle.
  ----------------------------------------------------------------------

  finish-refused-when-idle : ∀ s ts → finish s (log ts nothing) ≡ nothing
  finish-refused-when-idle s ts = refl

  finish-good : ∀ {s L L'} → s ≢ Active → Good L → finish s L ≡ just L' → Good L'
  finish-good _ (idle _) ()
  finish-good {s} sn (busy {pre} {t} {post} np sp act sq) eq
    with update (txId t) (settle s) (pre ++ t ∷ post) | update-active (settle s) pre t post np
  ... | .(just (pre ++ settle s t ∷ post)) | refl with eq
  ... | refl = idle (all-++ sp ((λ e → sn e) ∷ sq))

  commit-good : ∀ {L L'} → Good L → commit L ≡ just L' → Good L'
  commit-good = finish-good (λ e → Active≢Committed (sym e))

  markRolledBack-good : ∀ {L L'} → Good L → markRolledBack L ≡ just L' → Good L'
  markRolledBack-good = finish-good (λ e → Active≢RolledBack (sym e))

  ----------------------------------------------------------------------
  -- Theorem B-4: at most one transaction is Active, and exactly when the
  -- pointer is set.
  ----------------------------------------------------------------------

  countActive : List Tx → Nat
  countActive []       = 0
  countActive (t ∷ ts) with txState t
  ... | Active     = suc (countActive ts)
  ... | Committed  = countActive ts
  ... | RolledBack = countActive ts

  expected : Maybe Nat → Nat
  expected nothing  = 0
  expected (just _) = 1

  settled-count : ∀ {ts} → All Settled ts → countActive ts ≡ 0
  settled-count []                  = refl
  settled-count {t ∷ ts} (s ∷ ss) with txState t | s
  ... | Active     | s' = ⊥-elim (s' refl)
  ... | Committed  | _  = settled-count ss
  ... | RolledBack | _  = settled-count ss

  count-++ : ∀ xs ys → countActive (xs ++ ys) ≡ countActive xs + countActive ys
  count-++ []       ys = refl
  count-++ (x ∷ xs) ys with txState x
  ... | Active     = cong suc (count-++ xs ys)
  ... | Committed  = count-++ xs ys
  ... | RolledBack = count-++ xs ys

  at-most-one-active : ∀ {L} → Good L → countActive (txs L) ≡ expected (activeId L)
  at-most-one-active (idle ss) = settled-count ss
  at-most-one-active (busy {pre} {t} {post} _ sp act sq)
    rewrite count-++ pre (t ∷ post) | settled-count sp | act | settled-count sq = refl

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
                   → All (λ u → txId u ≢ txId t) pre
                   → update (txId t) f (pre ++ t ∷ post) ≡ just (pre ++ f t ∷ post)

  lifecycle-preserves-good : Preserves
  lifecycle-preserves-good = record
    { onBegin    = begin-good
    ; onAddOp    = addOp-good
    ; onCommit   = commit-good
    ; onRollback = markRolledBack-good
    ; onlyActive = update-active
    }
