{-# OPTIONS --safe --without-K #-}
-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
--
-- The epistemic fragment, rewritten.
--
-- januskey's Idris2 ABI carries a `Knows : Agent -> Type -> Type` modality
-- (src/abi/JanusKey/ABI/Types.idr §Level 12) inherited from the estate's
-- epistemic work. It cannot state what the security claims need:
--
--   * `Knows Adversary a` is uninhabited *by construction* — there is no
--     Adversary constructor — so "an attacker cannot recover ..." (ledger row
--     E-1) reduces to an absence in a datatype, which the repository's own
--     comment calls "a property of this model, not a security proof".
--   * "The system forgets the old key" is not expressible at all: `Knows` has
--     no negative form, so the lemma was deleted rather than repaired.
--
-- This module replaces the modality with one whose ignorance is a proposition.
-- Knowledge is indiscriminability over an observation: an agent knows φ in x
-- when φ holds in every world it cannot distinguish from x. Ignorance is then
-- ¬ Knows, which is provable by exhibiting a world that looks the same and is
-- not; and forgetting is a statement about what an agent can use, which can be
-- false.
--
-- What this does and does not buy is stated at the foot of the file.

module Epistemic where

open import Agda.Builtin.Bool using (Bool; true; false)
open import Agda.Builtin.Equality using (_≡_; refl)
open import Agda.Builtin.List using (List; []; _∷_)
open import Agda.Builtin.Maybe using (Maybe; nothing; just)
open import Agda.Builtin.Nat using (Nat; zero; suc)
open import Agda.Builtin.Sigma using (Σ; _,_)

----------------------------------------------------------------------
-- A kernel-sized prelude: nothing is imported beyond the builtins
----------------------------------------------------------------------

data ⊥ : Set where

_×_ : (A B : Set) → Set
A × B = Σ A (λ _ → B)

⊥-elim : ∀ {A : Set} → ⊥ → A
⊥-elim ()

¬_ : Set → Set
¬ A = A → ⊥

infix 3 ¬_

_≢_ : ∀ {A : Set} → A → A → Set
x ≢ y = ¬ (x ≡ y)

infix 4 _≢_

trans : ∀ {A : Set} {x y z : A} → x ≡ y → y ≡ z → x ≡ z
trans refl q = q

sym : ∀ {A : Set} {x y : A} → x ≡ y → y ≡ x
sym refl = refl

cong : ∀ {A B : Set} (f : A → B) {x y : A} → x ≡ y → f x ≡ f y
cong f refl = refl

just-inj : ∀ {a b : Nat} → just a ≡ just b → a ≡ b
just-inj refl = refl

nothing≢just : ∀ {a : Nat} → nothing ≡ just a → ⊥
nothing≢just ()

just≢nothing : ∀ {a : Nat} → just a ≡ nothing → ⊥
just≢nothing ()

false≢true : false ≡ true → ⊥
false≢true ()

subst : ∀ {A : Set} (P : A → Set) {x y : A} → x ≡ y → P x → P y
subst P refl p = p

if_then_else_ : ∀ {A : Set} → Bool → A → A → A
if true  then a else b = a
if false then a else b = b

-- Decidable equality on Nat, and the only fact about it the proofs need:
-- a number equals itself.
_==_ : Nat → Nat → Bool
zero  == zero  = true
zero  == suc _ = false
suc _ == zero  = false
suc m == suc n = m == n

==-refl : ∀ n → (n == n) ≡ true
==-refl zero    = refl
==-refl (suc n) = ==-refl n

----------------------------------------------------------------------
-- The model
----------------------------------------------------------------------

ContentHash = Nat
Content     = Nat
KeyId       = Nat

data Agent : Set where
  owner system auditor adversary : Agent

-- A world: what the store holds, which keys each agent can use, and what each
-- agent can see. `see` is the observation channel; the Store channel is
-- `content`. They are separate fields on purpose: the interesting theorems are
-- about how they come apart.
record World : Set where
  constructor w
  field
    content : ContentHash → Maybe Content
    usable  : Agent → KeyId → Bool
    see     : Agent → List (ContentHash × Content)

open World public

observe : Agent → World → List (ContentHash × Content)
observe a x = see x a

-- Knowledge as indiscriminability: φ holds in every world the agent cannot
-- tell apart from this one. This is the replacement for the uninhabited `Knows`
-- constructor family — it can be false, which is the point.
Knows : Agent → (World → Set) → World → Set
Knows a φ x = ∀ x' → observe a x' ≡ observe a x → φ x'

CanUse : World → Agent → KeyId → Set
CanUse x a k = usable x a k ≡ true

-- The store holds a recoverable value for this hash.
Recovers : World → ContentHash → Set
Recovers x h = Σ Content (λ c → content x h ≡ just c)

----------------------------------------------------------------------
-- Theorem 1. Knowledge is inhabited: an agent knows its own observation.
-- (The positive control — without it, "the adversary does not know" would be
-- uninteresting. Note what is *not* claimed: knowing that φ holds does not give
-- knowledge of φ; only what the observation fixes is known.)
----------------------------------------------------------------------

knows-observation : ∀ a x → Knows a (λ y → observe a y ≡ observe a x) x
knows-observation a x y eq = eq

----------------------------------------------------------------------
-- Theorem 2. Ignorance is a proposition: if a world looks the same to an agent
-- but its store differs, that agent does not know what the store holds.
----------------------------------------------------------------------

different-store-unknown :
  ∀ a x x' → observe a x' ≡ observe a x → content x' ≢ content x →
  ¬ Knows a (λ y → content y ≡ content x) x
different-store-unknown a x x' eq neq k = neq (k x' eq)

----------------------------------------------------------------------
-- Theorem 3. Two worlds, same observation, different content: the adversary
-- does not know the stored value. This is the shape ledger row E-1 could not
-- take in the Idris model, where `Knows Adversary a` had no constructor.
----------------------------------------------------------------------

private
  one≢two : suc zero ≡ suc (suc zero) → ⊥
  one≢two ()

-- Two worlds that differ only in what the store holds, and in nothing the
-- adversary can observe: its channel is empty in both.
x₁ : World
x₁ = w (λ _ → just (suc zero)) (λ _ _ → true) (λ _ → [])

x₂ : World
x₂ = w (λ _ → just (suc (suc zero))) (λ _ _ → true) (λ _ → [])

adversary-ignorant : ¬ Knows adversary (λ y → content y ≡ content x₁) x₁
adversary-ignorant k =
  one≢two (just-inj (sym (cong (λ f → f zero) (k x₂ refl))))

----------------------------------------------------------------------
-- Theorem 4. Obliteration cuts recovery: after obliterating h the store holds
-- nothing at h, so there is nothing for any agent to recover.
----------------------------------------------------------------------

obliterate : ContentHash → World → World
obliterate h x =
  record x { content = λ k → if k == h then nothing else content x k }

obliterate-cuts-recovery : ∀ h x → ¬ Recovers (obliterate h x) h
obliterate-cuts-recovery h x (c , p) =
  nothing≢just (subst (λ b → (if b then nothing else content x h) ≡ just c)
                      (==-refl h) p)

----------------------------------------------------------------------
-- Theorem 5. Obliteration does not make the adversary any wiser: the store
-- changes and its observation does not, so the ignorance of Theorem 3 survives
-- the obliteration. Stated at h = 0 for a closed proof.
----------------------------------------------------------------------

obliterate-ignorance :
  ¬ Knows adversary (λ y → content y ≡ content (obliterate zero x₁))
                   (obliterate zero x₁)
obliterate-ignorance k =
  adversary-so-blind (cong (λ f → f zero) (k x₂ refl))
  where
    -- `content x₂ zero` is `just 2`; after obliterating 0 the store's entry is
    -- `nothing`. Congruence gives an equation between them, which is absurd.
    adversary-so-blind : just (suc (suc zero)) ≡ nothing → ⊥
    adversary-so-blind = just≢nothing

----------------------------------------------------------------------
-- Theorem 6. Rotation forgets: after rotating old -> new with old ≠ new, no agent can use the
-- old key, including the system. This is the statement the Idris model could
-- not make at all, because `Knows` had no negative form.
----------------------------------------------------------------------

rotate : KeyId → KeyId → World → World
rotate old new x =
  record x { usable = λ a k →
               if k == new then true
               else (if k == old then false else usable x a k) }

rotate-keeps-new : ∀ a old new x → usable (rotate old new x) a new ≡ true
rotate-keeps-new a old new x rewrite ==-refl new = refl

rotate-forgets-old :
  ∀ a old new x → (old == new) ≡ false →
  usable (rotate old new x) a old ≡ false
rotate-forgets-old a old new x neq rewrite neq | ==-refl old = refl

rotate-forgets-canUse :
  ∀ a old new x → (old == new) ≡ false → ¬ CanUse (rotate old new x) a old
rotate-forgets-canUse a old new x neq p =
  false≢true (trans (sym (rotate-forgets-old a old new x neq)) p)

----------------------------------------------------------------------
-- What is NOT claimed.
--
--   * This is a model of *channels*, not of cryptography. That the store's
--     `content` channel is the only one an attacker has, that a hash does not
--     leak its content, and that a key that is not `usable` cannot be used to
--     decrypt are assumptions about the implementation, not theorems here.
--     Ledger row X-1 carries them.
--   * The adversary's observation `see` is modelled but not yet *filtered* by
--     obliteration: obliterate changes the store channel and leaves the
--     observation channel alone, which is exactly why Theorem 5 is available
--     and also its limit. Removing the entry from every agent's observation is
--     the next increment, and it needs a list-filtering lemma that this module
--     does not have.
--   * Key ids are Nat and `usable` is a Bool. Nothing here says a usable key
--     actually decrypts anything.
--   * Time, partial knowledge, and probabilistic leakage are out of scope.
----------------------------------------------------------------------
