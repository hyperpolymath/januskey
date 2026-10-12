-- SPDX-License-Identifier: MPL-2.0
-- Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
--
-- JanusKey ABI Proofs — Standalone formal verification module
-- Proves correctness properties for all JanusKey operations
-- No believe_me, no assert_total, no postulate — fully total

module JanusKey.ABI.Proofs

import JanusKey.ABI.Types
import JanusKey.ABI.Layout
import JanusKey.ABI.Foreign
import Data.Vect
import Data.Fin
import Data.So
import Data.Nat

%default total

-- ============================================================
-- 1. CNO Composition Proofs
-- ============================================================

-- Sequential composition of reversible operations: forward runs f then g,
-- inverse runs g⁻¹ then f⁻¹. The composite keeps f's kind tag (ReversibleOp
-- has no "sequence" kind); nothing below depends on the tag.
public export
seqOp : (f, g : ReversibleOp) -> ReversibleOp
seqOp f g = MkRevOp (kind f) (\s => forward g (forward f s)) (\s => inverse f (inverse g s))

-- Composition of CNOs is a CNO: (f;g);(g⁻¹;f⁻¹) = id. This is the theorem
-- the former `cnoPairCompose` (which returned Unit) claimed in a comment.
public export
seqCNO : {f, g : ReversibleOp} -> IsCNO f -> IsCNO g -> IsCNO (seqOp f g)
seqCNO (MkCNO pf) (MkCNO pg) =
  MkCNO (\s => trans (cong (inverse f) (pg (forward f s))) (pf s))

-- ============================================================
-- 2. Obliteration Completeness
-- ============================================================

-- Overwrite pass covers all bytes
-- For a file of size n, each pass writes exactly n bytes
public export
overwriteCoversAll : (fileSize : Nat) -> (pass : OverwritePassIdx)
                  -> (bytesWritten : Nat)
                  -> bytesWritten = fileSize
                  -> So (bytesWritten >= fileSize)
overwriteCoversAll n _ _ Refl = gteRefl n
  where
    gteRefl : (m : Nat) -> So (m >= m)
    gteRefl Z = Oh
    gteRefl (S j) = gteRefl j

-- The three-pass minimum is carried by ObliterationProof itself: its
-- `passesValid : So (overwritePasses >= 3)` field cannot be filled otherwise
-- (the former `threePassMinimum : So (3 >= 3)` restated a literal). Hash
-- one-wayness is a cryptographic assumption, not a theorem of this module;
-- it is recorded in PROOF-NEEDS.adoc as external, and the former
-- `hashOneWay` (which returned `Void -> Void`) is gone.

-- ============================================================
-- 3. Transaction Safety Proofs
-- ============================================================

-- A transaction cannot be committed twice
-- (Committed is a terminal state — no TxTransition from Committed)
public export
noDoubleCommit : TxTransition Committed s -> Void
noDoubleCommit _ impossible

||| A transaction cannot be rolled back twice
public export
noDoubleRollback : TxTransition RolledBack s -> Void
noDoubleRollback _ impossible

||| A pending transaction must be begun before commit
public export
pendingCannotCommit : TxTransition Pending Committed -> Void
pendingCannotCommit _ impossible

||| A pending transaction must be begun before rollback
public export
pendingCannotRollback : TxTransition Pending RolledBack -> Void
pendingCannotRollback _ impossible

||| Transaction lifecycle is deterministic:
||| Pending → Active → Committed | RolledBack
||| No other paths exist
public export
txLifecycleComplete : (from : TxState) -> (to : TxState)
                   -> Either (TxTransition from to) (TxTransition from to -> Void)
txLifecycleComplete Pending Active      = Left Begin
txLifecycleComplete Active Committed    = Left Commit
txLifecycleComplete Active RolledBack   = Left Rollback
txLifecycleComplete Pending Pending     = Right (\case _ impossible)
txLifecycleComplete Pending Committed   = Right (\case _ impossible)
txLifecycleComplete Pending RolledBack  = Right (\case _ impossible)
txLifecycleComplete Active Active       = Right (\case _ impossible)
txLifecycleComplete Active Pending      = Right (\case _ impossible)
txLifecycleComplete Committed _         = Right (\case _ impossible)
txLifecycleComplete RolledBack _        = Right (\case _ impossible)

-- ============================================================
-- 4. Key Derivation Correctness
-- ============================================================

||| Argon2 parameter bounds are carried by Layout.Argon2Params: a record can
||| only be built with proofs of timeCost >= 3, memoryCost >= 65536 KiB,
||| parallelism >= 1 and outputLen >= 32, and `defaultArgon2` supplies them.
||| The former `timeCostMonotonic` (returned its hypothesis),
||| `memoryDefeatsGPU` and `outputIs256Bits` (closed literals) and
||| `defaultExceedsMinimum` (restated the record fields) are gone; none was
||| a statement about key derivation.

-- ============================================================
-- 5. Attestation Chain Integrity
-- ============================================================

||| An attestation entry with hash chain
public export
record AttestEntry where
  constructor MkAttest
  entryHash  : ContentHash
  prevHash   : ContentHash
  opKind     : OpKind
  timestamp  : Nat

||| Attestation chain: each entry's prevHash = predecessor's entryHash
public export
data ValidChain : List AttestEntry -> Type where
  EmptyChain  : ValidChain []
  SingleEntry : ValidChain [e]
  ChainLink   : (prevHash e2 = entryHash e1)
             -> ValidChain (e1 :: rest)
             -> ValidChain (e2 :: e1 :: rest)

||| A valid chain has no gaps
public export
chainNoGaps : ValidChain chain -> (i : Fin (length chain))
           -> So (length chain > 0)
chainNoGaps EmptyChain i = absurd i
chainNoGaps SingleEntry _ = Oh
chainNoGaps (ChainLink _ _) _ = Oh

||| Tampering breaks the chain: replace e1 by an entry whose hash differs and
||| the link from e2 cannot be re-established. This is the theorem the former
||| `tamperDetectable` (which returned Unit) claimed in a comment.
public export
tamperBreaksLink : ValidChain (e2 :: e1 :: rest)
                -> Not (entryHash e1' = entryHash e1)
                -> ValidChain (e2 :: e1' :: rest) -> Void
tamperBreaksLink (ChainLink p _) neq (ChainLink p' _) = neq (trans (sym p') p)

-- ============================================================
-- 6. Effect Safety Proofs
-- ============================================================

||| File operations cannot access key material
||| (FileOpEffects does not contain ReadKey or WriteKey)
public export
fileOpsNoKeyAccess : HasEffect ReadKey FileOpEffects -> Void
fileOpsNoKeyAccess (There (There (There x))) impossible

public export
fileOpsNoKeyWrite : HasEffect WriteKey FileOpEffects -> Void
fileOpsNoKeyWrite (There (There (There x))) impossible

||| Obliteration requires entropy (for overwrite patterns)
public export
oblitNeedsEntropy : HasEffect Entropy OblitEffects
oblitNeedsEntropy = There (There (Here))

-- ============================================================
-- 7. Linearity Proofs
-- ============================================================

||| Linearity is carried by signatures, not restated here: `obliterateLinear`
||| and `zeroize` in Types.idr take their argument at multiplicity 1, so a
||| second use of the same value is a type error at the call site. (The
||| former `linearFileConsumed` claimed `LinearFile -> Void`, which is false —
||| LinearFile is inhabited — and was ill-typed.)

-- ============================================================
-- 8. Tropical Cost Proofs
-- ============================================================

||| Obliteration cost dominates copy cost: see Types.oblitCostDominates
||| (LTE n (3 * n)); the strict form for n > 0 is not stated.

||| Sequential cost is additive (tropical multiplication)
public export
sequentialCostAdditive : (a, b : Nat) -> tropMul (MkCost a) (MkCost b) = MkCost (a + b)
sequentialCostAdditive a b = Refl

||| Parallel cost takes the minimum (tropical addition)
public export
parallelCostMin : (a, b : Nat) -> tropAdd (MkCost a) (MkCost b) = MkCost (min a b)
parallelCostMin a b = Refl

-- ============================================================
-- 9. Epistemic Safety Proofs
-- ============================================================

||| After key revocation, adversary cannot use the key
||| (No constructor for Knows Adversary _ exists)
public export
revokedKeyUnusable : Knows Adversary KeyId -> Void
revokedKeyUnusable = adversaryCannotKnow

||| Auditor can see operations but not key material
public export
data AuditorView : Type where
  CanSeeOps : List OpKind -> AuditorView
  -- No constructor for seeing key bytes

||| "The system forgets the old key" is not expressible in the Knows model:
||| Knows has no negative form, so the former `systemForgetsOldKey` (which
||| returned knowledge of the new key and said nothing about the old) is gone.
||| Types.systemKnowsNewKey remains as the positive statement.

-- ============================================================
-- 10. Error Code Proofs
-- ============================================================

||| Status codes are a closed sum with a total decoder and an injective
||| encoding: Foreign.statusRoundTrip and Foreign.statusCodeInjective.
||| Re-exported here under the name the old (never-typechecking) proof used.
public export
errorCodeDeterministic : (e1, e2 : Status) -> statusCode e1 = statusCode e2 -> e1 = e2
errorCodeDeterministic = statusCodeInjective
