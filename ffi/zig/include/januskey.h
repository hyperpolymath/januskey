/* SPDX-License-Identifier: PMPL-1.0-or-later */
/* Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) */
/* JanusKey C FFI Header — generated from src/abi/Foreign.idr */

#ifndef JANUSKEY_H
#define JANUSKEY_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Error codes (from Foreign.idr CError) */
#define JK_OK                    0
#define JK_ERR_NOT_INITIALIZED   1
#define JK_ERR_INVALID_PATH      2
#define JK_ERR_IO                3
#define JK_ERR_CRYPTO            4
#define JK_ERR_TX_NOT_ACTIVE     5
#define JK_ERR_TX_CONFLICT       6
#define JK_ERR_KEY_NOT_FOUND     7
#define JK_ERR_KEY_REVOKED       8
#define JK_ERR_OBLITERATION      9
#define JK_ERR_ATTESTATION      10
#define JK_ERR_BUFFER_TOO_SMALL 11

/* Types (from Types.idr, Layout.idr) */
typedef struct { uint8_t bytes[32]; } jk_content_hash_t;
typedef struct { uint8_t bytes[16]; } jk_key_id_t;
typedef void* jk_handle_t;
typedef void* jk_tx_t;

typedef enum {
    JK_OP_COPY       = 0,
    JK_OP_MOVE       = 1,
    JK_OP_DELETE      = 2,
    JK_OP_MODIFY      = 3,
    JK_OP_OBLITERATE  = 4,
    JK_OP_KEY_GEN     = 5,
    JK_OP_KEY_ROTATE  = 6,
    JK_OP_KEY_REVOKE  = 7,
} jk_op_kind_t;

typedef enum {
    JK_ALGO_AES256GCM = 0,
    JK_ALGO_CHACHA20  = 1,
    JK_ALGO_ED25519   = 2,
    JK_ALGO_X25519    = 3,
    JK_ALGO_ARGON2ID  = 4,
} jk_algorithm_t;

typedef struct {
    jk_content_hash_t content_hash;
    uint8_t           nonce[32];
    jk_content_hash_t commitment;
    uint64_t          overwrite_passes;
    uint8_t           passes_valid;
} jk_oblit_proof_t;  /* 112 bytes, 8-byte aligned */

/* Repository lifecycle */
int jk_init(const char* path, jk_handle_t* out_handle);
int jk_open(const char* path, jk_handle_t* out_handle);
void jk_close(jk_handle_t handle);

/* File operations */
int jk_execute(jk_handle_t handle, jk_op_kind_t op,
               const char* src, const char* dst);
int jk_undo(jk_handle_t handle);
int jk_obliterate(jk_handle_t handle, const char* path,
                  jk_oblit_proof_t* out_proof);

/* Key management */
int jk_generate_key(jk_handle_t handle, jk_algorithm_t algo,
                    const char* passphrase, jk_key_id_t* out_id);
int jk_rotate_key(jk_handle_t handle, const jk_key_id_t* old_id,
                  const char* new_passphrase, jk_key_id_t* out_new_id);
int jk_revoke_key(jk_handle_t handle, const jk_key_id_t* key_id);

/* Transactions */
int jk_tx_begin(jk_handle_t handle, jk_tx_t* out_tx);
int jk_tx_commit(jk_handle_t handle, jk_tx_t tx);
int jk_tx_rollback(jk_handle_t handle, jk_tx_t tx);

/* Version */
const char* jk_version(void);

#ifdef __cplusplus
}
#endif

#endif /* JANUSKEY_H */
