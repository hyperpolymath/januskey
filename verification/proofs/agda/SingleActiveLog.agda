{-# OPTIONS --safe --without-K #-}
-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
--
-- The single-active log, as theory.
--
-- A log is a list of records, each with a key and a phase (active or
-- settled), plus a pointer to the key of the active record. Three kinds of
-- transition act on it: open a new record, edit the active record, and
-- settle the active record. Invariant `Good`: the pointer is set exactly when
-- one record is active, every other record is settled, and the active record
-- is the first with its key. The theorems below hold for every record type,
-- key type and transition that obey the stated laws. That is what makes them
-- theory in PROOF-POLICY §1's sense: they name no type or function of any
-- one implementation.
--
-- januskey's TransactionManager is one instance (TransactionAtomicity.agda,
-- Part B). That the instance is faithful to the Rust is a separate,
-- engineering obligation (ledger row T-10), for Creusot and Verus.
--
-- Nothing is imported beyond Agda's builtins, so the Rocq and Lean recheck
-- legs can restate every definition here without a library.

module SingleActiveLog where

open import Agda.Builtin.Equality using (_≡_; refl)
open import Agda.Builtin.List using (List; []; _∷_)
open import Agda.Builtin.Maybe using (Maybe; nothing; just)
open import Agda.Builtin.Nat using (Nat; zero; suc; _+_)

----------------------------------------------------------------------
-- A builtins-only prelude
----------------------------------------------------------------------

data ⊥ : Set where

⊥-elim : {A : Set} → ⊥ → A
⊥-elim ()

_≢_ : {A : Set} → A → A → Set
x ≢ y = x ≡ y → ⊥

infix 4 _≢_

sym : {A : Set} {x y : A} → x ≡ y → y ≡ x
sym refl = refl

trans : {A : Set} {x y z : A} → x ≡ y → y ≡ z → x ≡ z
trans refl q = q

cong : {A B : Set} (f : A → B) {x y : A} → x ≡ y → f x ≡ f y
cong f refl = refl

subst : {A : Set} (P : A → Set) {x y : A} → x ≡ y → P x → P y
subst P refl p = p

data _⊎_ (A B : Set) : Set where
  inl : A → A ⊎ B
  inr : B → A ⊎ B

infixr 5 _++_
_++_ : {A : Set} → List A → List A → List A
[]       ++ ys = ys
(x ∷ xs) ++ ys = x ∷ (xs ++ ys)

data All {A : Set} (P : A → Set) : List A → Set where
  []  : All P []
  _∷_ : ∀ {x xs} → P x → All P xs → All P (x ∷ xs)

all-++ : ∀ {A : Set} {P : A → Set} {xs ys} → All P xs → All P ys → All P (xs ++ ys)
all-++ []       qs = qs
all-++ (p ∷ ps) qs = p ∷ all-++ ps qs

----------------------------------------------------------------------
-- Phases
----------------------------------------------------------------------

data Phase : Set where
  active settled : Phase

active≢settled : active ≡ settled → ⊥
active≢settled ()

----------------------------------------------------------------------
-- The log, over any record type with a decidable key
----------------------------------------------------------------------

module Log (R I : Set)
           (_≟_   : (a b : I) → (a ≡ b) ⊎ (a ≢ b))
           (key   : R → I)
           (phase : R → Phase) where

  record Log : Set where
    constructor log
    field
      txs      : List R
      activeId : Maybe I

  open Log public

  Settled : R → Set
  Settled r = phase r ≡ settled

  Fresh : I → List R → Set
  Fresh i = All (λ u → key u ≢ i)

  -- Rewrite the FIRST record with key i; nothing if there is none.
  update : I → (R → R) → List R → Maybe (List R)
  update i f []       = nothing
  update i f (t ∷ ts) with key t ≟ i
  ... | inl _ = just (f t ∷ ts)
  ... | inr _ with update i f ts
  ...   | nothing = nothing
  ...   | just us = just (t ∷ us)

  -- Open a new record; refused while a record is active.
  begin : R → Log → Maybe Log
  begin r (log ts (just _)) = nothing
  begin r (log ts nothing)  = just (log (ts ++ r ∷ []) (just (key r)))

  -- Edit the active record; refused when idle.
  modify : (R → R) → Log → Maybe Log
  modify f (log ts nothing)  = nothing
  modify f (log ts (just i)) with update i f ts
  ... | nothing = nothing
  ... | just us = just (log us (just i))

  -- Settle the active record; refused when idle.
  finish : (R → R) → Log → Maybe Log
  finish g (log ts nothing)  = nothing
  finish g (log ts (just i)) with update i g ts
  ... | nothing = nothing
  ... | just us = just (log us nothing)

  data Good : Log → Set where
    idle : ∀ {ts} → All Settled ts → Good (log ts nothing)
    busy : ∀ {pre t post}
         → All (λ u → key u ≢ key t) pre
         → All Settled pre
         → phase t ≡ active
         → All Settled post
         → Good (log (pre ++ t ∷ post) (just (key t)))

  good-empty : Good (log [] nothing)
  good-empty = idle []

  -- Only the active record is rewritten; everything before and after it is
  -- untouched.
  update-active : ∀ f pre t post
                → All (λ u → key u ≢ key t) pre
                → update (key t) f (pre ++ t ∷ post) ≡ just (pre ++ f t ∷ post)
  update-active f []        t post []       with key t ≟ key t
  ... | inl _ = refl
  ... | inr n = ⊥-elim (n refl)
  update-active f (u ∷ pre) t post (n ∷ ns) with key u ≟ key t
  ... | inl e = ⊥-elim (n e)
  ... | inr _ with update (key t) f (pre ++ t ∷ post) | update-active f pre t post ns
  ...   | .(just (pre ++ f t ∷ post)) | refl = refl

  -- A key-preserving edit keeps every record before the active one distinct
  -- from it.
  distinct-after : ∀ {k k' : I} pre → k' ≡ k
                 → All (λ u → key u ≢ k) pre
                 → All (λ u → key u ≢ k') pre
  distinct-after []        e []       = []
  distinct-after (u ∷ pre) e (n ∷ ns) =
    (λ q → n (trans q e)) ∷ distinct-after pre e ns

  ----------------------------------------------------------------------
  -- The transitions preserve Good
  ----------------------------------------------------------------------

  begin-refused-while-active : ∀ r ts j → begin r (log ts (just j)) ≡ nothing
  begin-refused-while-active r ts j = refl

  begin-good : ∀ {r L L'} → phase r ≡ active → Good L → Fresh (key r) (txs L)
             → begin r L ≡ just L' → Good L'
  begin-good _  (busy _ _ _ _) _ ()
  begin-good {r} act (idle {ts} ss) fr refl =
    busy {pre = ts} {t = r} {post = []} fr ss act []

  modify-refused-when-idle : ∀ f ts → modify f (log ts nothing) ≡ nothing
  modify-refused-when-idle f ts = refl

  -- An edit must keep the key and the phase of the record it edits.
  modify-good : ∀ {f L L'}
              → (∀ r → key (f r) ≡ key r) → (∀ r → phase (f r) ≡ phase r)
              → Good L → modify f L ≡ just L' → Good L'
  modify-good _ _ (idle _) ()
  modify-good {f} kf pf (busy {pre} {t} {post} np sp act sq) eq
    with update (key t) f (pre ++ t ∷ post) | update-active f pre t post np
  ... | .(just (pre ++ f t ∷ post)) | refl with eq
  ... | refl =
    subst (λ k → Good (log (pre ++ f t ∷ post) (just k))) (kf t)
      (busy {pre = pre} {t = f t} {post = post}
            (distinct-after pre (kf t) np) sp (trans (pf t) act) sq)

  finish-refused-when-idle : ∀ g ts → finish g (log ts nothing) ≡ nothing
  finish-refused-when-idle g ts = refl

  -- A settling transition must keep the key and land in the settled phase.
  finish-good : ∀ {g L L'}
              → (∀ r → phase (g r) ≡ settled)
              → Good L → finish g L ≡ just L' → Good L'
  finish-good _ (idle _) ()
  finish-good {g} pg (busy {pre} {t} {post} np sp act sq) eq
    with update (key t) g (pre ++ t ∷ post) | update-active g pre t post np
  ... | .(just (pre ++ g t ∷ post)) | refl with eq
  ... | refl = idle (all-++ sp (pg t ∷ sq))

  ----------------------------------------------------------------------
  -- At most one record is active, and exactly when the pointer is set
  ----------------------------------------------------------------------

  countActive : List R → Nat
  countActive []       = 0
  countActive (t ∷ ts) with phase t
  ... | active  = suc (countActive ts)
  ... | settled = countActive ts

  expected : Maybe I → Nat
  expected nothing  = 0
  expected (just _) = 1

  settled-count : ∀ {ts} → All Settled ts → countActive ts ≡ 0
  settled-count []                  = refl
  settled-count {t ∷ ts} (s ∷ ss) with phase t
  ... | active  = ⊥-elim (active≢settled s)
  ... | settled = settled-count ss

  count-++ : ∀ xs ys → countActive (xs ++ ys) ≡ countActive xs + countActive ys
  count-++ []       ys = refl
  count-++ (x ∷ xs) ys with phase x
  ... | active  = cong suc (count-++ xs ys)
  ... | settled = count-++ xs ys

  at-most-one-active : ∀ {L} → Good L → countActive (txs L) ≡ expected (activeId L)
  at-most-one-active (idle ss) = settled-count ss
  at-most-one-active (busy {pre} {t} {post} _ sp act sq)
    rewrite count-++ pre (t ∷ post) | settled-count sp | act | settled-count sq = refl
