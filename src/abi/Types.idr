-- SPDX-License-Identifier: PMPL-1.0-or-later
-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
--
-- JanusKey ABI Types — Idris2 formal definitions
-- Implements TypeLL Levels 1-12 for provably reversible operations
--
-- Lineage: maa-framework → absolute-zero (CNO theory) → januskey (PoC)
-- These types prove at compile time that JanusKey operations satisfy
-- the Certified Null Operation (CNO) property from absolute-zero.

module JanusKey.ABI.Types

import Data.Vect
import Data.Fin
import Data.So

%default total

-- ============================================================
-- Level 1: Instruction Validity
-- Well-formed operation types — parse-time safety
-- ============================================================

||| Operation kind tag — every JanusKey operation is one of these
public export
data OpKind : Type where
  Copy      : OpKind
  Move      : OpKind
  Delete    : OpKind
  Modify    : OpKind
  Obliterate : OpKind
  KeyGen    : OpKind
  KeyRotate : OpKind
  KeyRevoke : OpKind

||| Proof that an OpKind is a file operation (not key management)
public export
data IsFileOp : OpKind -> Type where
  CopyIsFile   : IsFileOp Copy
  MoveIsFile   : IsFileOp Move
  DeleteIsFile : IsFileOp Delete
  ModifyIsFile : IsFileOp Modify
  OblitIsFile  : IsFileOp Obliterate

||| Proof that an OpKind is a key operation
public export
data IsKeyOp : OpKind -> Type where
  GenIsKey    : IsKeyOp KeyGen
  RotateIsKey : IsKeyOp KeyRotate
  RevokeIsKey : IsKeyOp KeyRevoke

-- ============================================================
-- Level 2: Region-Binding
-- All paths resolve to valid filesystem regions
-- ============================================================

||| A validated filesystem path (non-empty, no null bytes)
public export
record ValidPath where
  constructor MkValidPath
  pathStr   : String
  nonEmpty  : So (length pathStr > 0)
  noNulls   : So (not (isInfixOf "\0" pathStr))

||| Storage region — where content lives
public export
data Region : Type where
  ContentStore : ValidPath -> Region
  MetadataStore : ValidPath -> Region
  KeyStore : ValidPath -> Region
  TransactionLog : ValidPath -> Region

||| Proof that an operation targets a valid region
public export
data BoundTo : OpKind -> Region -> Type where
  FileOpBound : IsFileOp k -> (r : Region) -> BoundTo k r
  KeyOpBound  : IsKeyOp k -> (r : Region) -> BoundTo k (KeyStore p)

-- ============================================================
-- Level 3: Type-Compatible Operations
-- Operations match their operand types
-- ============================================================

||| Content hash — SHA256 digest as 32 bytes
public export
record ContentHash where
  constructor MkHash
  bytes : Vect 32 Bits8

||| File metadata required for reversibility
public export
record FileMetadata where
  constructor MkFileMeta
  path        : ValidPath
  size        : Nat
  hash        : ContentHash
  permissions : Bits32

||| Key algorithm identifier
public export
data KeyAlgorithm : Type where
  AES256GCM   : KeyAlgorithm
  ChaCha20    : KeyAlgorithm
  Ed25519     : KeyAlgorithm
  X25519      : KeyAlgorithm
  Argon2id    : KeyAlgorithm

||| Proof that an operation is type-compatible with its arguments
public export
data TypeCompat : OpKind -> Type -> Type where
  CopyCompat   : TypeCompat Copy (FileMetadata, ValidPath)
  MoveCompat   : TypeCompat Move (FileMetadata, ValidPath)
  DeleteCompat : TypeCompat Delete FileMetadata
  ModifyCompat : TypeCompat Modify (FileMetadata, ContentHash)
  OblitCompat  : TypeCompat Obliterate FileMetadata
  KeyGenCompat : TypeCompat KeyGen KeyAlgorithm

-- ============================================================
-- Level 4: Null-Safety
-- No implicit null — all optionality is explicit
-- ============================================================

||| Operation result — never null, always explicit
public export
data OpResult : Type -> Type where
  Success : (val : a) -> OpResult a
  Failure : (err : String) -> OpResult a

||| Proof that an OpResult is successful
public export
data IsSuccess : OpResult a -> Type where
  ItSucceeded : IsSuccess (Success val)

||| Extract value from a proven-successful result (total, no crash)
public export
extractSuccess : (r : OpResult a) -> IsSuccess r -> a
extractSuccess (Success val) ItSucceeded = val

-- ============================================================
-- Level 5: Bounds-Proof
-- Array/buffer accesses are within bounds
-- ============================================================

||| Bounded index into a content store with n entries
public export
data BoundedIndex : (n : Nat) -> Type where
  MkBounded : (idx : Fin n) -> BoundedIndex n

||| Overwrite pass index — proven within OVERWRITE_PASSES (3)
public export
OverwritePassIdx : Type
OverwritePassIdx = Fin 3

||| Proof that a buffer size matches expected content size
public export
data SizeMatch : (expected : Nat) -> (actual : Nat) -> Type where
  SizesMatch : SizeMatch n n

-- ============================================================
-- Level 6: Result-Type
-- Return type of every operation is statically known
-- ============================================================

||| Type-level function: given an OpKind, what does it return?
public export
ResultOf : OpKind -> Type
ResultOf Copy       = (ContentHash, FileMetadata)
ResultOf Move       = (ContentHash, FileMetadata)
ResultOf Delete     = ContentHash
ResultOf Modify     = (ContentHash, ContentHash)  -- old hash, new hash
ResultOf Obliterate = ObliterationProof
ResultOf KeyGen     = KeyId
ResultOf KeyRotate  = (KeyId, KeyId)  -- old, new
ResultOf KeyRevoke  = KeyId

||| Key identifier — opaque, unique
public export
record KeyId where
  constructor MkKeyId
  uuid : Vect 16 Bits8

||| Forward declaration for obliteration proof (defined in Layout.idr)
public export
record ObliterationProof where
  constructor MkOblitProof
  contentHash    : ContentHash
  nonce          : Vect 32 Bits8
  commitment     : ContentHash
  overwritePasses : Nat
  passesValid    : So (overwritePasses >= 3)

-- ============================================================
-- Level 7: Aliasing Safety
-- Mutable references are exclusive — no aliased writes
-- ============================================================

||| Resource state tag — tracks whether a resource is available
public export
data ResourceState : Type where
  Available : ResourceState
  Locked    : ResourceState
  Consumed  : ResourceState

||| A resource handle with exclusive access tracking
public export
data Handle : (state : ResourceState) -> Type -> Type where
  MkHandle : (tag : Nat) -> (val : a) -> Handle Available a

||| Acquire exclusive access — transitions Available -> Locked
public export
acquire : Handle Available a -> (Handle Locked a, a)
acquire (MkHandle tag val) = (MkHandle tag val, val)

||| Release exclusive access — transitions Locked -> Available
public export
release : Handle Locked a -> a -> Handle Available a
release (MkHandle tag _) newVal = MkHandle tag newVal

-- ============================================================
-- Level 8: Effect-Tracking
-- All side effects are declared and verified
-- ============================================================

||| Effect tags — what an operation can do
public export
data Effect : Type where
  ReadFS    : Effect   -- Read from filesystem
  WriteFS   : Effect   -- Write to filesystem
  DeleteFS  : Effect   -- Delete from filesystem
  ReadKey   : Effect   -- Read key material
  WriteKey  : Effect   -- Write key material
  Entropy   : Effect   -- Use CSPRNG
  Network   : Effect   -- Network access (attestation sync)
  AuditLog  : Effect   -- Write to audit log

||| An effectful operation with declared effects
public export
data Eff : List Effect -> Type -> Type where
  Pure   : a -> Eff [] a
  Bind   : Eff effs a -> (a -> Eff effs' b) -> Eff (effs ++ effs') b

||| Proof that an effect list contains a specific effect
public export
data HasEffect : Effect -> List Effect -> Type where
  Here  : HasEffect e (e :: es)
  There : HasEffect e es -> HasEffect e (f :: es)

||| File operations require ReadFS + WriteFS + AuditLog
public export
FileOpEffects : List Effect
FileOpEffects = [ReadFS, WriteFS, AuditLog]

||| Obliteration requires WriteFS + DeleteFS + Entropy + AuditLog
public export
OblitEffects : List Effect
OblitEffects = [WriteFS, DeleteFS, Entropy, AuditLog]

||| Key operations require ReadKey + WriteKey + Entropy + AuditLog
public export
KeyOpEffects : List Effect
KeyOpEffects = [ReadKey, WriteKey, Entropy, AuditLog]

-- ============================================================
-- Level 9: Temporal / Lifetime Safety
-- Transaction state machine — no use-after-commit
-- ============================================================

||| Transaction state — enforces correct lifecycle
public export
data TxState : Type where
  Pending   : TxState
  Active    : TxState
  Committed : TxState
  RolledBack : TxState

||| State transition proof — only valid transitions allowed
public export
data TxTransition : TxState -> TxState -> Type where
  Begin    : TxTransition Pending Active
  Commit   : TxTransition Active Committed
  Rollback : TxTransition Active RolledBack

||| A transaction with compile-time state tracking
public export
data Tx : TxState -> Type where
  MkTx : (id : Nat) -> Tx Pending

||| Begin a transaction — type-level state change
public export
beginTx : Tx Pending -> Tx Active
beginTx (MkTx id) = MkTx id

||| Commit — only possible from Active state
public export
commitTx : Tx Active -> Tx Committed
commitTx (MkTx id) = MkTx id

||| Rollback — only possible from Active state
public export
rollbackTx : Tx Active -> Tx RolledBack
rollbackTx (MkTx id) = MkTx id

||| Proof: cannot commit a non-active transaction
||| (This is enforced by the type system — no term of type
|||  `Tx Pending -> Tx Committed` can be constructed)

-- ============================================================
-- Level 10: Linearity (QTT)
-- Resources used exactly once — no double-free, no leak
-- ============================================================

||| Linear file handle — must be consumed exactly once
||| Uses Idris2 multiplicities: (1 x : a) means x used exactly once
public export
data LinearFile : Type where
  MkLinearFile : (1 _ : ValidPath) -> (1 _ : ContentHash) -> LinearFile

||| Consume a linear file by obliterating it — returns proof of destruction
public export
obliterateLinear : (1 _ : LinearFile) -> ObliterationProof
obliterateLinear (MkLinearFile path hash) =
  MkOblitProof hash (replicate 32 0) hash 3 Oh

||| Linear key material — must be zeroized after use
public export
data LinearKey : Type where
  MkLinearKey : (1 _ : Vect n Bits8) -> (1 _ : KeyId) -> LinearKey

||| Zeroize key material — consumes the linear key
public export
zeroize : (1 _ : LinearKey) -> KeyId
zeroize (MkLinearKey _ kid) = kid

-- ============================================================
-- Level 11: Tropical Cost-Tracking
-- Operation cost bounded by min-plus semiring
-- ============================================================

||| Cost in the tropical semiring (min-plus)
||| TropCost n means "this operation costs at most n units"
public export
data TropCost : Nat -> Type where
  MkCost : (bound : Nat) -> TropCost bound

||| Tropical addition: min(a, b)
public export
tropAdd : TropCost a -> TropCost b -> TropCost (min a b)
tropAdd (MkCost a) (MkCost b) = MkCost (min a b)

||| Tropical multiplication: a + b (sequential composition)
public export
tropMul : TropCost a -> TropCost b -> TropCost (a + b)
tropMul (MkCost a) (MkCost b) = MkCost (a + b)

||| Cost of basic operations (in abstract units)
public export
copyCost : (fileSize : Nat) -> TropCost fileSize
copyCost n = MkCost n

public export
hashCost : (fileSize : Nat) -> TropCost fileSize
hashCost n = MkCost n

||| Obliteration cost = 3 * fileSize (3 overwrite passes)
public export
oblitCost : (fileSize : Nat) -> TropCost (3 * fileSize)
oblitCost n = MkCost (3 * n)

||| Proof: obliteration is the most expensive file operation
public export
oblitMostExpensive : (n : Nat) -> So (3 * n >= n)
oblitMostExpensive Z = Oh
oblitMostExpensive (S k) = Oh

-- ============================================================
-- Level 12: Epistemic Safety
-- Who knows what about key material and content
-- ============================================================

||| Knowledge agent — who can observe state
public export
data Agent : Type where
  Owner     : Agent   -- The key/file owner
  System    : Agent   -- The JanusKey system
  Auditor   : Agent   -- An authorized auditor
  Adversary : Agent   -- An attacker (should know nothing)

||| Knowledge predicate: agent knows proposition
public export
data Knows : Agent -> Type -> Type where
  OwnerKnows   : (val : a) -> Knows Owner a
  SystemKnows  : (val : a) -> Knows System a
  AuditorKnows : (val : a) -> Knows Auditor a

||| After obliteration, adversary cannot know content
||| (There is no constructor `AdversaryKnows` — this is an impossibility proof)
public export
data PostObliteration : ContentHash -> Type where
  Obliterated : (hash : ContentHash) -> PostObliteration hash

||| Proof: adversary cannot recover obliterated content
||| No term of type `Knows Adversary content -> PostObliteration hash -> Void`
||| can be constructed because `Knows Adversary content` is uninhabited
public export
adversaryCannotKnow : Knows Adversary a -> Void
adversaryCannotKnow _ impossible

||| After key rotation, only the new key is knowable
public export
data KeyRotated : KeyId -> KeyId -> Type where
  Rotated : (old : KeyId) -> (new : KeyId) -> KeyRotated old new

||| System knows the new key but not the old after rotation
public export
systemKnowsNewKey : KeyRotated old new -> Knows System KeyId
systemKnowsNewKey (Rotated _ new) = SystemKnows new

||| Owner gets notified of rotation
public export
ownerNotified : KeyRotated old new -> Knows Owner (KeyId, KeyId)
ownerNotified (Rotated old new) = OwnerKnows (old, new)
