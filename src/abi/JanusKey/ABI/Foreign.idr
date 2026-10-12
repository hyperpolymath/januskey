-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
--
-- JanusKey ABI — foreign function interface.
--
-- This module is the Idris2 side of ffi/zig/include/januskey.h. Every
-- `%foreign` declaration below names one exported symbol of libjanuskey
-- (implemented in ffi/zig/src/main.zig) with the C signature the header
-- gives it. The header is hand-maintained; this module must mirror it, and
-- the two are kept honest by the status, op-kind and algorithm codecs here,
-- whose encodings are the header's `#define`s and enum values, each with a
-- total decoder and a machine-checked round-trip lemma.
--
-- What is and is not claimed:
--   * Claimed: the codecs are total and injective (lemmas below); the
--     foreign signatures agree with januskey.h at the level of C types.
--   * Not claimed: that the Zig implementation returns only the codes it
--     documents, or anything about its behaviour. That is ffi/zig's test
--     suite's job (ffi/zig/test/integration_test.zig).
--   * Out-pointer parameters (`jk_handle_t*`, `jk_key_id_t*`,
--     `jk_oblit_proof_t*`) are declared as `AnyPtr` cells the caller owns.
--     Allocation of those cells is a C/Zig-side concern, not modelled here.
--
-- History: until 2026-10-12 this file carried the RSR template FFI
-- (`januskey_init`, `januskey_process`, `januskey_get_string`, …) written
-- against template types that Types.idr never defined; it did not describe
-- ffi/zig at all. See PROOF-NEEDS.adoc, "One ABI tree".

module JanusKey.ABI.Foreign

import JanusKey.ABI.Types
import JanusKey.ABI.Layout
import Data.Vect
import Data.So
import Data.Maybe
import Control.Function

%default total

--------------------------------------------------------------------------------
-- Status codes: januskey.h JK_OK / JK_ERR_* (0..11), ffi/zig `Error`
--------------------------------------------------------------------------------

||| Every status libjanuskey can return. A closed sum: there is no
||| "unknown status" constructor, so a code the header does not define
||| decodes to Nothing and must be handled as a contract violation.
public export
data Status : Type where
  Ok                : Status  -- JK_OK                  0
  ErrNotInitialized : Status  -- JK_ERR_NOT_INITIALIZED 1
  ErrInvalidPath    : Status  -- JK_ERR_INVALID_PATH    2
  ErrIo             : Status  -- JK_ERR_IO              3
  ErrCrypto         : Status  -- JK_ERR_CRYPTO          4
  ErrTxNotActive    : Status  -- JK_ERR_TX_NOT_ACTIVE   5
  ErrTxConflict     : Status  -- JK_ERR_TX_CONFLICT     6
  ErrKeyNotFound    : Status  -- JK_ERR_KEY_NOT_FOUND   7
  ErrKeyRevoked     : Status  -- JK_ERR_KEY_REVOKED     8
  ErrObliteration   : Status  -- JK_ERR_OBLITERATION    9
  ErrAttestation    : Status  -- JK_ERR_ATTESTATION    10
  ErrBufferTooSmall : Status  -- JK_ERR_BUFFER_TOO_SMALL 11

||| Wire value of a status (januskey.h).
public export
statusCode : Status -> Nat
statusCode Ok                = 0
statusCode ErrNotInitialized = 1
statusCode ErrInvalidPath    = 2
statusCode ErrIo             = 3
statusCode ErrCrypto         = 4
statusCode ErrTxNotActive    = 5
statusCode ErrTxConflict     = 6
statusCode ErrKeyNotFound    = 7
statusCode ErrKeyRevoked     = 8
statusCode ErrObliteration   = 9
statusCode ErrAttestation    = 10
statusCode ErrBufferTooSmall = 11

||| Total decoder: every natural number decodes, to a status or to Nothing.
public export
statusFromCode : Nat -> Maybe Status
statusFromCode 0  = Just Ok
statusFromCode 1  = Just ErrNotInitialized
statusFromCode 2  = Just ErrInvalidPath
statusFromCode 3  = Just ErrIo
statusFromCode 4  = Just ErrCrypto
statusFromCode 5  = Just ErrTxNotActive
statusFromCode 6  = Just ErrTxConflict
statusFromCode 7  = Just ErrKeyNotFound
statusFromCode 8  = Just ErrKeyRevoked
statusFromCode 9  = Just ErrObliteration
statusFromCode 10 = Just ErrAttestation
statusFromCode 11 = Just ErrBufferTooSmall
statusFromCode _  = Nothing

||| Round trip: decoding an encoded status gives it back. Checked by cases,
||| one `Refl` per constructor; a wrong table entry fails to typecheck.
export
statusRoundTrip : (s : Status) -> statusFromCode (statusCode s) = Just s
statusRoundTrip Ok                = Refl
statusRoundTrip ErrNotInitialized = Refl
statusRoundTrip ErrInvalidPath    = Refl
statusRoundTrip ErrIo             = Refl
statusRoundTrip ErrCrypto         = Refl
statusRoundTrip ErrTxNotActive    = Refl
statusRoundTrip ErrTxConflict     = Refl
statusRoundTrip ErrKeyNotFound    = Refl
statusRoundTrip ErrKeyRevoked     = Refl
statusRoundTrip ErrObliteration   = Refl
statusRoundTrip ErrAttestation    = Refl
statusRoundTrip ErrBufferTooSmall = Refl

||| Codes are injective: two statuses with the same wire value are the same
||| status. Follows from the round trip, so it cannot drift from the table.
export
statusCodeInjective : (s, t : Status) -> statusCode s = statusCode t -> s = t
statusCodeInjective s t prf =
  injective (trans (sym (statusRoundTrip s))
                       (trans (cong statusFromCode prf) (statusRoundTrip t)))

||| The C `int` seen at the boundary. Negative values and values above 11
||| decode to Nothing: the caller must treat them as a broken contract, never
||| as success.
public export
statusFromInt : Int -> Maybe Status
statusFromInt x = if x < 0 then Nothing else statusFromCode (cast x)

--------------------------------------------------------------------------------
-- Operation kinds and algorithms: januskey.h enums, in Types.idr's order
--------------------------------------------------------------------------------

||| jk_op_kind_t. The order is Types.idr's constructor order, which is the
||| header's enum order; the round trip below is what pins that.
public export
opKindCode : OpKind -> Nat
opKindCode Copy       = 0
opKindCode Move       = 1
opKindCode Delete     = 2
opKindCode Modify     = 3
opKindCode Obliterate = 4
opKindCode KeyGen     = 5
opKindCode KeyRotate  = 6
opKindCode KeyRevoke  = 7

public export
opKindFromCode : Nat -> Maybe OpKind
opKindFromCode 0 = Just Copy
opKindFromCode 1 = Just Move
opKindFromCode 2 = Just Delete
opKindFromCode 3 = Just Modify
opKindFromCode 4 = Just Obliterate
opKindFromCode 5 = Just KeyGen
opKindFromCode 6 = Just KeyRotate
opKindFromCode 7 = Just KeyRevoke
opKindFromCode _ = Nothing

export
opKindRoundTrip : (k : OpKind) -> opKindFromCode (opKindCode k) = Just k
opKindRoundTrip Copy       = Refl
opKindRoundTrip Move       = Refl
opKindRoundTrip Delete     = Refl
opKindRoundTrip Modify     = Refl
opKindRoundTrip Obliterate = Refl
opKindRoundTrip KeyGen     = Refl
opKindRoundTrip KeyRotate  = Refl
opKindRoundTrip KeyRevoke  = Refl

||| jk_algorithm_t.
public export
algorithmCode : KeyAlgorithm -> Nat
algorithmCode AES256GCM = 0
algorithmCode ChaCha20  = 1
algorithmCode Ed25519   = 2
algorithmCode X25519    = 3
algorithmCode Argon2id  = 4

public export
algorithmFromCode : Nat -> Maybe KeyAlgorithm
algorithmFromCode 0 = Just AES256GCM
algorithmFromCode 1 = Just ChaCha20
algorithmFromCode 2 = Just Ed25519
algorithmFromCode 3 = Just X25519
algorithmFromCode 4 = Just Argon2id
algorithmFromCode _ = Nothing

export
algorithmRoundTrip : (a : KeyAlgorithm) -> algorithmFromCode (algorithmCode a) = Just a
algorithmRoundTrip AES256GCM = Refl
algorithmRoundTrip ChaCha20  = Refl
algorithmRoundTrip Ed25519   = Refl
algorithmRoundTrip X25519    = Refl
algorithmRoundTrip Argon2id  = Refl

--------------------------------------------------------------------------------
-- Opaque handles: jk_handle_t and jk_tx_t are `void*`
--------------------------------------------------------------------------------

||| Library-handle lifecycle state.
public export
data LibState = Open | Closed

||| A library instance handle (jk_handle_t) indexed by its state. Operations
||| take an Open handle unrestricted; `closeOnce` takes it at multiplicity 1
||| and returns the Closed witness, so in any context that holds the handle
||| linearly a second close is a type error (tests/reject-idris/DoubleClose.idr)
||| and forgetting to close, where a Closed witness is demanded, is too.
||| Scope, stated plainly: linearity is a property of the binding. A handle
||| bound by an IO `<-` is unrestricted; the discipline holds for handles
||| passed as `(1 h : LibHandle Open)` parameters, which is how callers that
||| want the guarantee must receive it.
public export
data LibHandle : LibState -> Type where
  Opened : (ptr : AnyPtr) -> LibHandle Open
  Shut   : LibHandle Closed

||| The raw pointer of an open handle (wire form for the %foreign calls).
public export
libPtr : LibHandle Open -> AnyPtr
libPtr (Opened p) = p

||| A transaction token (jk_tx_t).
public export
record TxToken where
  constructor MkTxToken
  ptr : AnyPtr

--------------------------------------------------------------------------------
-- Foreign declarations: one per exported symbol, C types as in januskey.h
--------------------------------------------------------------------------------

-- Library lifecycle --------------------------------------------------------

||| int jk_init(const char* path, jk_handle_t* out_handle)
export
%foreign "C:jk_init, libjanuskey"
prim__jk_init : String -> AnyPtr -> PrimIO Int

||| int jk_open(const char* path, jk_handle_t* out_handle)
export
%foreign "C:jk_open, libjanuskey"
prim__jk_open : String -> AnyPtr -> PrimIO Int

||| void jk_close(jk_handle_t handle)
export
%foreign "C:jk_close, libjanuskey"
prim__jk_close : AnyPtr -> PrimIO ()

-- File operations ----------------------------------------------------------

||| int jk_execute(jk_handle_t handle, jk_op_kind_t op, const char* src, const char* dst)
export
%foreign "C:jk_execute, libjanuskey"
prim__jk_execute : AnyPtr -> Int -> String -> String -> PrimIO Int

||| int jk_undo(jk_handle_t handle)
export
%foreign "C:jk_undo, libjanuskey"
prim__jk_undo : AnyPtr -> PrimIO Int

||| int jk_obliterate(jk_handle_t handle, const char* path, jk_oblit_proof_t* out_proof)
||| out_proof is a caller-owned cell of oblitProofSize bytes (Layout.idr: 112).
export
%foreign "C:jk_obliterate, libjanuskey"
prim__jk_obliterate : AnyPtr -> String -> AnyPtr -> PrimIO Int

-- Key management -----------------------------------------------------------

||| int jk_generate_key(jk_handle_t handle, jk_algorithm_t algo, const char* passphrase, jk_key_id_t* out_id)
export
%foreign "C:jk_generate_key, libjanuskey"
prim__jk_generate_key : AnyPtr -> Int -> String -> AnyPtr -> PrimIO Int

||| int jk_rotate_key(jk_handle_t handle, const jk_key_id_t* old_id, const char* new_passphrase, jk_key_id_t* out_new_id)
export
%foreign "C:jk_rotate_key, libjanuskey"
prim__jk_rotate_key : AnyPtr -> AnyPtr -> String -> AnyPtr -> PrimIO Int

||| int jk_revoke_key(jk_handle_t handle, const jk_key_id_t* key_id)
export
%foreign "C:jk_revoke_key, libjanuskey"
prim__jk_revoke_key : AnyPtr -> AnyPtr -> PrimIO Int

-- Transactions -------------------------------------------------------------

||| int jk_tx_begin(jk_handle_t handle, jk_tx_t* out_tx)
export
%foreign "C:jk_tx_begin, libjanuskey"
prim__jk_tx_begin : AnyPtr -> AnyPtr -> PrimIO Int

||| int jk_tx_commit(jk_handle_t handle, jk_tx_t tx)
export
%foreign "C:jk_tx_commit, libjanuskey"
prim__jk_tx_commit : AnyPtr -> AnyPtr -> PrimIO Int

||| int jk_tx_rollback(jk_handle_t handle, jk_tx_t tx)
export
%foreign "C:jk_tx_rollback, libjanuskey"
prim__jk_tx_rollback : AnyPtr -> AnyPtr -> PrimIO Int

-- Version --------------------------------------------------------------------

||| const char* jk_version(void)
export
%foreign "C:jk_version, libjanuskey"
prim__jk_version : PrimIO String

--------------------------------------------------------------------------------
-- Typed wrappers. Each returns `Either Int Status`: Right for a code the
-- header defines, Left carrying the raw int for one it does not. Nothing is
-- ever read as success by default.
--------------------------------------------------------------------------------

||| Decode a raw status; keep the raw value on failure so the caller can log it.
public export
decodeStatus : Int -> Either Int Status
decodeStatus x = maybe (Left x) Right (statusFromInt x)

||| Destroy a handle at most once: consumes it (multiplicity 1) and returns
||| the Closed witness.
export
closeOnce : (1 h : LibHandle Open) -> IO (LibHandle Closed)
closeOnce (Opened p) = do
  primIO (prim__jk_close p)
  pure Shut

export
undo : LibHandle Open -> IO (Either Int Status)
undo h = map decodeStatus (primIO (prim__jk_undo (libPtr h)))

||| Execute a file operation. The op kind crosses the boundary as its
||| header enum value (opKindCode), so an OpKind that Zig does not know is
||| unrepresentable here rather than an invalid int there.
export
execute : LibHandle Open -> OpKind -> (src : String) -> (dst : String) -> IO (Either Int Status)
execute h k src dst =
  map decodeStatus (primIO (prim__jk_execute (libPtr h) (cast (opKindCode k)) src dst))

||| Obliterate `path`, writing the proof into a caller-owned 112-byte cell.
export
obliterate : LibHandle Open -> (path : String) -> (outProof : AnyPtr) -> IO (Either Int Status)
obliterate h path cell = map decodeStatus (primIO (prim__jk_obliterate (libPtr h) path cell))

export
generateKey : LibHandle Open -> KeyAlgorithm -> (passphrase : String) -> (outId : AnyPtr) -> IO (Either Int Status)
generateKey h a pass cell =
  map decodeStatus (primIO (prim__jk_generate_key (libPtr h) (cast (algorithmCode a)) pass cell))

export
rotateKey : LibHandle Open -> (oldId : AnyPtr) -> (newPassphrase : String) -> (outNewId : AnyPtr) -> IO (Either Int Status)
rotateKey h old pass cell =
  map decodeStatus (primIO (prim__jk_rotate_key (libPtr h) old pass cell))

export
revokeKey : LibHandle Open -> (keyId : AnyPtr) -> IO (Either Int Status)
revokeKey h kid = map decodeStatus (primIO (prim__jk_revoke_key (libPtr h) kid))

export
txCommit : LibHandle Open -> TxToken -> IO (Either Int Status)
txCommit h t = map decodeStatus (primIO (prim__jk_tx_commit (libPtr h) t.ptr))

export
txRollback : LibHandle Open -> TxToken -> IO (Either Int Status)
txRollback h t = map decodeStatus (primIO (prim__jk_tx_rollback (libPtr h) t.ptr))

export
version : IO String
version = primIO prim__jk_version
