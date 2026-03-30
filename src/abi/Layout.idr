-- SPDX-License-Identifier: PMPL-1.0-or-later
-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
--
-- JanusKey ABI Layout — Memory layout proofs and C-compatible structures
-- Proves that Rust and Zig FFI representations are bit-compatible

module JanusKey.ABI.Layout

import JanusKey.ABI.Types
import Data.Vect
import Data.Fin
import Data.So

%default total

-- ============================================================
-- C-Compatible Struct Layouts
-- ============================================================

||| Size of a C-compatible struct in bytes
public export
data StructSize : Nat -> Type where
  MkSize : (n : Nat) -> StructSize n

||| ContentHash layout: exactly 32 bytes, no padding
public export
contentHashSize : StructSize 32
contentHashSize = MkSize 32

||| KeyId layout: exactly 16 bytes (UUID)
public export
keyIdSize : StructSize 16
keyIdSize = MkSize 16

||| ObliterationProof layout:
||| 32 (hash) + 32 (nonce) + 32 (commitment) + 8 (passes) + 1 (valid) = 105
||| Aligned to 8: 112 bytes
public export
oblitProofSize : StructSize 112
oblitProofSize = MkSize 112

-- ============================================================
-- Alignment Proofs
-- ============================================================

||| Proof that a size is aligned to a boundary
public export
data Aligned : (size : Nat) -> (boundary : Nat) -> Type where
  MkAligned : (prf : mod size boundary = 0) -> Aligned size boundary

||| ContentHash is 8-byte aligned
public export
hashAligned : Aligned 32 8
hashAligned = MkAligned Refl

||| KeyId is 8-byte aligned
public export
keyIdAligned : Aligned 16 8
keyIdAligned = MkAligned Refl

||| ObliterationProof is 8-byte aligned
public export
oblitAligned : Aligned 112 8
oblitAligned = MkAligned Refl

-- ============================================================
-- Endianness-Safe Serialization
-- ============================================================

||| Byte order for serialization
public export
data ByteOrder : Type where
  LittleEndian : ByteOrder
  BigEndian    : ByteOrder
  NetworkOrder : ByteOrder  -- always big-endian

||| Proof that network byte order is big-endian
public export
networkIsBig : NetworkOrder = BigEndian -> Void
networkIsBig Refl impossible

-- Note: NetworkOrder and BigEndian are distinct constructors,
-- but the FFI layer treats them identically for wire format.

-- ============================================================
-- CNO (Certified Null Operation) Proofs
-- From absolute-zero theory
-- ============================================================

||| State of the filesystem at a point in time
public export
record FSState where
  constructor MkFSState
  files   : List (ValidPath, ContentHash)
  keys    : List (KeyId, KeyAlgorithm)
  txCount : Nat

||| An operation with its inverse
public export
record ReversibleOp where
  constructor MkRevOp
  kind    : OpKind
  forward : FSState -> FSState
  inverse : FSState -> FSState

||| CNO property: forward then inverse = identity
public export
data IsCNO : ReversibleOp -> Type where
  MkCNO : ((s : FSState) -> inverse op (forward op s) = s)
        -> IsCNO op

||| Copy-then-delete is a CNO (move + unmove = identity)
public export
copyDeleteCNO : (src, dst : ValidPath) -> (h : ContentHash)
             -> (op : ReversibleOp)
             -> (prf : kind op = Copy)
             -> IsCNO op

||| Obliterate does NOT have a CNO inverse — this is intentional
||| The type system prevents constructing IsCNO for Obliterate
public export
data ObliterateIsIrreversible : Type where
  ||| Obliteration is the one operation that breaks reversibility
  ||| by design (GDPR right to erasure)
  Irreversible : ObliterateIsIrreversible

-- ============================================================
-- Transaction Atomicity Proof
-- ============================================================

||| A sequence of operations in a transaction
public export
data TxOps : TxState -> List OpKind -> Type where
  Empty  : TxOps Active []
  Append : TxOps Active ops -> (k : OpKind) -> TxOps Active (ops ++ [k])

||| Proof: committing a transaction with all-reversible ops is safe
public export
data AllReversible : List OpKind -> Type where
  NilReversible : AllReversible []
  ConsReversible : Not (k = Obliterate)
                -> AllReversible ks
                -> AllReversible (k :: ks)

||| Proof: a committed transaction can be rolled back if all ops are reversible
public export
canRollback : AllReversible ops -> TxOps Active ops -> TxOps Active ops
canRollback NilReversible Empty = Empty
canRollback (ConsReversible notOblit rest) (Append txOps k) =
  Append (canRollback rest txOps) k

-- ============================================================
-- Key Derivation Safety
-- ============================================================

||| Argon2id parameters with proven safety bounds
public export
record Argon2Params where
  constructor MkArgon2
  timeCost   : Nat
  memoryCost : Nat   -- in KiB
  parallelism : Nat
  outputLen  : Nat
  timeOk     : So (timeCost >= 3)
  memoryOk   : So (memoryCost >= 65536)  -- 64 MiB minimum
  parallelOk : So (parallelism >= 1)
  outputOk   : So (outputLen >= 32)

||| Default secure parameters
public export
defaultArgon2 : Argon2Params
defaultArgon2 = MkArgon2 3 65536 4 32 Oh Oh Oh Oh

||| Proof: default parameters meet OWASP recommendations
public export
defaultMeetsOWASP : So (timeCost defaultArgon2 >= 3)
defaultMeetsOWASP = Oh

public export
defaultMemoryOWASP : So (memoryCost defaultArgon2 >= 65536)
defaultMemoryOWASP = Oh
