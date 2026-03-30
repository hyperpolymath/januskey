-- SPDX-License-Identifier: PMPL-1.0-or-later
-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
--
-- JanusKey ABI Foreign — C FFI declarations
-- Maps Idris2 types to C-compatible foreign function interface
-- Generated C headers go to generated/abi/januskey.h

module JanusKey.ABI.Foreign

import JanusKey.ABI.Types
import JanusKey.ABI.Layout
import Data.Vect
import Data.So

%default total

-- ============================================================
-- C Type Mappings
-- ============================================================

||| C-compatible error codes
public export
data CError : Type where
  JK_OK                  : CError  -- 0
  JK_ERR_NOT_INITIALIZED : CError  -- 1
  JK_ERR_INVALID_PATH    : CError  -- 2
  JK_ERR_IO              : CError  -- 3
  JK_ERR_CRYPTO          : CError  -- 4
  JK_ERR_TX_NOT_ACTIVE   : CError  -- 5
  JK_ERR_TX_CONFLICT     : CError  -- 6
  JK_ERR_KEY_NOT_FOUND   : CError  -- 7
  JK_ERR_KEY_REVOKED     : CError  -- 8
  JK_ERR_OBLITERATION    : CError  -- 9
  JK_ERR_ATTESTATION     : CError  -- 10
  JK_ERR_BUFFER_TOO_SMALL : CError -- 11

||| Convert CError to numeric code for C
public export
errorCode : CError -> Int
errorCode JK_OK                  = 0
errorCode JK_ERR_NOT_INITIALIZED = 1
errorCode JK_ERR_INVALID_PATH    = 2
errorCode JK_ERR_IO              = 3
errorCode JK_ERR_CRYPTO          = 4
errorCode JK_ERR_TX_NOT_ACTIVE   = 5
errorCode JK_ERR_TX_CONFLICT     = 6
errorCode JK_ERR_KEY_NOT_FOUND   = 7
errorCode JK_ERR_KEY_REVOKED     = 8
errorCode JK_ERR_OBLITERATION    = 9
errorCode JK_ERR_ATTESTATION     = 10
errorCode JK_ERR_BUFFER_TOO_SMALL = 11

||| Proof: all error codes are non-negative
public export
errorCodeNonNeg : (e : CError) -> So (errorCode e >= 0)
errorCodeNonNeg JK_OK                  = Oh
errorCodeNonNeg JK_ERR_NOT_INITIALIZED = Oh
errorCodeNonNeg JK_ERR_INVALID_PATH    = Oh
errorCodeNonNeg JK_ERR_IO              = Oh
errorCodeNonNeg JK_ERR_CRYPTO          = Oh
errorCodeNonNeg JK_ERR_TX_NOT_ACTIVE   = Oh
errorCodeNonNeg JK_ERR_TX_CONFLICT     = Oh
errorCodeNonNeg JK_ERR_KEY_NOT_FOUND   = Oh
errorCodeNonNeg JK_ERR_KEY_REVOKED     = Oh
errorCodeNonNeg JK_ERR_OBLITERATION    = Oh
errorCodeNonNeg JK_ERR_ATTESTATION     = Oh
errorCodeNonNeg JK_ERR_BUFFER_TOO_SMALL = Oh

||| Proof: JK_OK is the only success code
public export
onlyOkIsSuccess : (e : CError) -> errorCode e = 0 -> e = JK_OK
onlyOkIsSuccess JK_OK Refl = Refl

-- ============================================================
-- Foreign Function Signatures
-- These map to the C header generated/abi/januskey.h
-- ============================================================

||| Opaque handle to a JanusKey instance
public export
data JKHandle : Type where
  MkJKHandle : (ptr : Int) -> JKHandle

||| FFI: Initialize a JanusKey repository
||| C: int jk_init(const char* path, jk_handle_t* out_handle)
public export
record JKInitArgs where
  constructor MkInitArgs
  repoPath : ValidPath

||| FFI: Open an existing JanusKey repository
||| C: int jk_open(const char* path, jk_handle_t* out_handle)
public export
record JKOpenArgs where
  constructor MkOpenArgs
  repoPath : ValidPath

||| FFI: Execute a file operation
||| C: int jk_execute(jk_handle_t handle, jk_op_kind_t op,
|||                    const char* src, const char* dst)
public export
record JKExecuteArgs where
  constructor MkExecArgs
  handle  : JKHandle
  op      : OpKind
  src     : ValidPath
  dst     : Maybe ValidPath

||| FFI: Undo the last operation
||| C: int jk_undo(jk_handle_t handle)
public export
record JKUndoArgs where
  constructor MkUndoArgs
  handle : JKHandle

||| FFI: Obliterate a file (irreversible secure deletion)
||| C: int jk_obliterate(jk_handle_t handle, const char* path,
|||                       jk_oblit_proof_t* out_proof)
public export
record JKObliterateArgs where
  constructor MkOblitArgs
  handle : JKHandle
  target : ValidPath

||| FFI: Generate a new key
||| C: int jk_generate_key(jk_handle_t handle, jk_algorithm_t algo,
|||                         const char* passphrase, jk_key_id_t* out_id)
public export
record JKKeyGenArgs where
  constructor MkKeyGenArgs
  handle     : JKHandle
  algorithm  : KeyAlgorithm
  passphrase : String

||| FFI: Rotate a key
||| C: int jk_rotate_key(jk_handle_t handle, jk_key_id_t old_id,
|||                       const char* new_passphrase, jk_key_id_t* out_new_id)
public export
record JKKeyRotateArgs where
  constructor MkKeyRotateArgs
  handle        : JKHandle
  oldKeyId      : KeyId
  newPassphrase : String

||| FFI: Begin a transaction
||| C: int jk_tx_begin(jk_handle_t handle, jk_tx_t* out_tx)
||| FFI: Commit a transaction
||| C: int jk_tx_commit(jk_handle_t handle, jk_tx_t tx)
||| FFI: Rollback a transaction
||| C: int jk_tx_rollback(jk_handle_t handle, jk_tx_t tx)

-- ============================================================
-- Safety Contracts for FFI Boundary
-- ============================================================

||| Contract: every FFI call returns a valid error code
public export
data ValidFFIReturn : Int -> Type where
  IsValid : So (code >= 0) -> So (code <= 11) -> ValidFFIReturn code

||| Contract: init must be called before any other operation
public export
data Initialized : JKHandle -> Type where
  WasInitialized : Initialized h

||| Contract: obliterate consumes the file linearly
public export
data ObliterateContract : JKHandle -> ValidPath -> Type where
  ||| After obliterate returns JK_OK, the file at path is
  ||| provably destroyed (3-pass overwrite + verification)
  OblitContract : (h : JKHandle)
               -> (p : ValidPath)
               -> (proof : ObliterationProof)
               -> ObliterateContract h p

||| Contract: key generation produces unique IDs
public export
data KeyGenContract : JKHandle -> KeyAlgorithm -> Type where
  ||| Generated key ID is unique within this repository
  KeyGenUnique : (h : JKHandle) -> (a : KeyAlgorithm) -> (kid : KeyId)
              -> KeyGenContract h a
