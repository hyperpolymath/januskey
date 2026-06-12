// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// JanusKey C FFI implementation (Zig).
//
// This module is the concrete implementation of the C ABI declared in
// `ffi/zig/include/januskey.h`, which is itself generated from the Idris2
// specification in `src/abi/{Foreign,Types,Layout}.idr`. The names, the
// struct byte-layouts and the integer error codes are all fixed by that
// header; the compile-time assertions below pin them so the build fails
// loudly if the layout ever drifts from the spec.
//
// Behaviour is an honest scaffold: the lifecycle, transaction-state and
// null-guard semantics that the conformance suite
// (`ffi/zig/test/integration_test.zig`) exercises are implemented for real,
// while the cryptographic / CNO product internals are stubbed and marked
// `TODO(product):`. Every public function returns a documented `JK_OK` /
// `JK_ERR_*` code from `Error` below.

const std = @import("std");
const builtin = @import("builtin");

// ============================================================
// Version
// ============================================================

const VERSION = "0.1.0";

// ============================================================
// Error codes (must match januskey.h / Foreign.idr CError)
//
// Backed by c_int so `@intFromEnum` yields the exact wire values the C
// header `#define`s (JK_OK = 0, JK_ERR_* = 1..11). The integration suite
// asserts each discriminant.
// ============================================================

pub const Error = enum(c_int) {
    ok = 0,
    not_initialized = 1,
    invalid_path = 2,
    io_error = 3,
    crypto_error = 4,
    tx_not_active = 5,
    tx_conflict = 6,
    key_not_found = 7,
    key_revoked = 8,
    obliteration_error = 9,
    attestation_error = 10,
    buffer_too_small = 11,
};

/// Convenience: return the `c_int` wire value of an `Error`.
inline fn code(e: Error) c_int {
    return @intFromEnum(e);
}

// ============================================================
// Operation / algorithm enums (must match januskey.h)
// ============================================================

pub const OpKind = enum(c_int) {
    copy = 0,
    move = 1,
    delete = 2,
    modify = 3,
    obliterate = 4,
    key_gen = 5,
    key_rotate = 6,
    key_revoke = 7,
};

pub const Algorithm = enum(c_int) {
    aes256gcm = 0,
    chacha20 = 1,
    ed25519 = 2,
    x25519 = 3,
    argon2id = 4,
};

// ============================================================
// C-compatible value types (must match januskey.h / Layout.idr)
//
// `extern struct` guarantees C layout. The comptime asserts below lock the
// sizes/alignments the header documents and the test re-checks.
// ============================================================

/// SHA-256 content digest — exactly 32 bytes (Layout.idr: contentHashSize).
pub const ContentHash = extern struct {
    bytes: [32]u8 = [_]u8{0} ** 32,
};

/// Key identifier (UUID) — exactly 16 bytes (Layout.idr: keyIdSize).
pub const KeyId = extern struct {
    bytes: [16]u8 = [_]u8{0} ** 16,
};

/// Obliteration proof — 112 bytes, 8-byte aligned (Layout.idr: oblitProofSize).
///
/// Field order matches januskey.h exactly:
///   content_hash 32 + nonce 32 + commitment 32 + overwrite_passes 8 (u64)
///   + passes_valid 1 (u8). Raw payload is 105 bytes; the u64 forces 8-byte
///   alignment so the struct tail-pads to 112.
pub const OblitProof = extern struct {
    content_hash: ContentHash = .{},
    nonce: [32]u8 = [_]u8{0} ** 32,
    commitment: ContentHash = .{},
    overwrite_passes: u64 = 0,
    passes_valid: u8 = 0,
};

comptime {
    // Layout invariants — these mirror src/abi/Layout.idr and the comments
    // in januskey.h. If any fires, the C ABI and the Idris spec disagree.
    std.debug.assert(@sizeOf(ContentHash) == 32);
    std.debug.assert(@sizeOf(KeyId) == 16);
    std.debug.assert(@sizeOf(OblitProof) == 112);
    std.debug.assert(@alignOf(OblitProof) >= 8);
    std.debug.assert(@offsetOf(OblitProof, "content_hash") == 0);
    std.debug.assert(@offsetOf(OblitProof, "nonce") == 32);
    std.debug.assert(@offsetOf(OblitProof, "commitment") == 64);
    std.debug.assert(@offsetOf(OblitProof, "overwrite_passes") == 96);
    std.debug.assert(@offsetOf(OblitProof, "passes_valid") == 104);
    // Error code wire values must match the C header #defines.
    std.debug.assert(code(Error.ok) == 0);
    std.debug.assert(code(Error.buffer_too_small) == 11);
}

// ============================================================
// Internal handle
//
// `jk_handle_t` is `void*` on the C side. On the Zig side it is a sized,
// heap-allocated struct (NOT an `opaque` type — those cannot carry fields)
// so we can track repository state and the in-flight transaction. The
// pointer is round-tripped through `?*anyopaque` at the C boundary.
//
// Default number of secure-overwrite passes; matches OVERWRITE_PASSES in
// crates/januskey-cli/src/obliteration.rs and Types.idr (>= 3).
// ============================================================

const OVERWRITE_PASSES: u64 = 3;

const Handle = struct {
    /// Owning allocator (libc malloc/free via std.heap.c_allocator).
    allocator: std.mem.Allocator,
    /// Repository initialised flag.
    initialized: bool,
    /// Exactly one transaction may be active at a time. We store its
    /// heap-allocated token pointer so the test's `tx` round-trips.
    active_tx: ?*Transaction,
};

const Transaction = struct {
    /// Monotonic id (scaffold — a real impl threads this into the log).
    id: u64,
};

/// Reinterpret the C `?*anyopaque` handle as a `*Handle`, or null.
inline fn asHandle(h: ?*anyopaque) ?*Handle {
    return @ptrCast(@alignCast(h));
}

inline fn asTx(t: ?*anyopaque) ?*Transaction {
    return @ptrCast(@alignCast(t));
}

// ============================================================
// Repository lifecycle
// ============================================================

/// Initialise (create) a repository at `path` and return a fresh handle in
/// `out_handle`. Returns JK_ERR_INVALID_PATH if `path`/`out_handle` is null,
/// JK_ERR_IO on allocation failure, JK_OK otherwise.
pub export fn jk_init(path: ?[*:0]const u8, out_handle: ?*?*anyopaque) c_int {
    const out = out_handle orelse return code(.invalid_path);
    const p = path orelse {
        out.* = null;
        return code(.invalid_path);
    };
    // An empty path is not a valid repository location.
    if (std.mem.len(p) == 0) {
        out.* = null;
        return code(.invalid_path);
    }

    // TODO(product): create the on-disk `.januskey/` layout, content store,
    // metadata log and transaction directory for `path` (mirrors
    // crates/reversible-core JanusKey::init). The scaffold only allocates an
    // in-memory handle so the FFI lifecycle is exercisable end-to-end.
    const allocator = std.heap.c_allocator;
    const handle = allocator.create(Handle) catch {
        out.* = null;
        return code(.io_error);
    };
    handle.* = .{
        .allocator = allocator,
        .initialized = true,
        .active_tx = null,
    };
    out.* = handle;
    return code(.ok);
}

/// Open an existing repository. Scaffold shares `jk_init`'s behaviour.
pub export fn jk_open(path: ?[*:0]const u8, out_handle: ?*?*anyopaque) c_int {
    // TODO(product): require the repository to already exist and load its
    // state, rather than creating it.
    return jk_init(path, out_handle);
}

/// Close a handle. Null is a safe no-op (the test relies on this). Any
/// dangling active transaction token is freed first.
pub export fn jk_close(handle: ?*anyopaque) void {
    const h = asHandle(handle) orelse return;
    if (h.active_tx) |tx| {
        h.allocator.destroy(tx);
        h.active_tx = null;
    }
    h.initialized = false;
    h.allocator.destroy(h);
}

// ============================================================
// File operations
// ============================================================

/// Execute a file operation. With a null handle the repository is not
/// initialised, so JK_ERR_NOT_INITIALIZED is returned (asserted by the test).
///
/// `op` is typed `c_int` (the wire type of the C `jk_op_kind_t` enum) rather
/// than the Zig `OpKind` enum: the conformance suite passes a bare integer
/// literal (`jk_execute(null, 0, …)`), which is exactly how a C caller passes
/// an enum value. We map it back to `OpKind` internally.
pub export fn jk_execute(
    handle: ?*anyopaque,
    op: c_int,
    src: ?[*:0]const u8,
    dst: ?[*:0]const u8,
) c_int {
    const h = asHandle(handle) orelse return code(.not_initialized);
    if (!h.initialized) return code(.not_initialized);
    const kind: OpKind = std.meta.intToEnum(OpKind, op) catch return code(.invalid_path);
    _ = kind;
    _ = src;
    _ = dst;
    // TODO(product): dispatch on `op` and execute the reversible operation
    // (copy/move/delete/modify/obliterate/key-*) recording inverse metadata,
    // delegating to the reversible-core executor.
    return code(.ok);
}

/// Undo the most recent operation. Null handle => not initialised.
pub export fn jk_undo(handle: ?*anyopaque) c_int {
    const h = asHandle(handle) orelse return code(.not_initialized);
    if (!h.initialized) return code(.not_initialized);
    // TODO(product): pop the last operation from the log and apply its
    // stored inverse (Theorem 3.4: Sequential Reversibility).
    return code(.ok);
}

/// Obliterate the content at `path`, optionally emitting a proof. Null handle
/// => not initialised. When `out_proof` is non-null it is filled with a
/// well-formed (scaffold) proof carrying the standard overwrite-pass count.
pub export fn jk_obliterate(
    handle: ?*anyopaque,
    path: ?[*:0]const u8,
    out_proof: ?*OblitProof,
) c_int {
    const h = asHandle(handle) orelse return code(.not_initialized);
    if (!h.initialized) return code(.not_initialized);
    if (path == null) return code(.invalid_path);
    // TODO(product): perform the DoD 5220.22-M secure-overwrite passes and
    // compute the real commitment H(content_hash || nonce || timestamp)
    // (mirrors crates/januskey-cli/src/obliteration.rs). The scaffold returns
    // a structurally-valid proof so consumers can exercise the ABI.
    if (out_proof) |proof| {
        proof.* = .{};
        proof.overwrite_passes = OVERWRITE_PASSES;
        proof.passes_valid = 1;
    }
    return code(.ok);
}

// ============================================================
// Key management
// ============================================================

/// Generate a key. Scaffold returns a zeroed id and JK_OK for a valid handle.
pub export fn jk_generate_key(
    handle: ?*anyopaque,
    algo: Algorithm,
    passphrase: ?[*:0]const u8,
    out_id: ?*KeyId,
) c_int {
    const h = asHandle(handle) orelse return code(.not_initialized);
    if (!h.initialized) return code(.not_initialized);
    _ = algo;
    _ = passphrase;
    // TODO(product): derive key material (Argon2id KDF + AEAD), persist the
    // wrapped key, and return its UUID.
    if (out_id) |id| id.* = .{};
    return code(.ok);
}

/// Rotate a key. Scaffold returns a zeroed new id and JK_OK.
pub export fn jk_rotate_key(
    handle: ?*anyopaque,
    old_id: ?*const KeyId,
    new_passphrase: ?[*:0]const u8,
    out_new_id: ?*KeyId,
) c_int {
    const h = asHandle(handle) orelse return code(.not_initialized);
    if (!h.initialized) return code(.not_initialized);
    if (old_id == null) return code(.key_not_found);
    _ = new_passphrase;
    // TODO(product): re-wrap content under the new key and retire the old one.
    if (out_new_id) |id| id.* = .{};
    return code(.ok);
}

/// Revoke a key. Scaffold returns JK_OK for a valid handle + id.
pub export fn jk_revoke_key(handle: ?*anyopaque, key_id: ?*const KeyId) c_int {
    const h = asHandle(handle) orelse return code(.not_initialized);
    if (!h.initialized) return code(.not_initialized);
    if (key_id == null) return code(.key_not_found);
    // TODO(product): mark the key revoked in the key store / audit log.
    return code(.ok);
}

// ============================================================
// Transactions
//
// Invariant exercised by the suite: at most one active transaction per
// handle. A second `jk_tx_begin` while one is active => JK_ERR_TX_CONFLICT;
// commit/rollback of a null/foreign token => JK_ERR_TX_NOT_ACTIVE.
// ============================================================

pub export fn jk_tx_begin(handle: ?*anyopaque, out_tx: ?*?*anyopaque) c_int {
    const h = asHandle(handle) orelse return code(.not_initialized);
    if (!h.initialized) return code(.not_initialized);
    const out = out_tx orelse return code(.invalid_path);
    if (h.active_tx != null) {
        out.* = null;
        return code(.tx_conflict);
    }
    const tx = h.allocator.create(Transaction) catch {
        out.* = null;
        return code(.io_error);
    };
    tx.* = .{ .id = 1 };
    h.active_tx = tx;
    out.* = tx;
    return code(.ok);
}

pub export fn jk_tx_commit(handle: ?*anyopaque, tx: ?*anyopaque) c_int {
    const h = asHandle(handle) orelse return code(.not_initialized);
    if (!h.initialized) return code(.not_initialized);
    const active = h.active_tx orelse return code(.tx_not_active);
    const given = asTx(tx) orelse return code(.tx_not_active);
    if (given != active) return code(.tx_not_active);
    // TODO(product): durably commit the staged operations.
    h.allocator.destroy(active);
    h.active_tx = null;
    return code(.ok);
}

pub export fn jk_tx_rollback(handle: ?*anyopaque, tx: ?*anyopaque) c_int {
    const h = asHandle(handle) orelse return code(.not_initialized);
    if (!h.initialized) return code(.not_initialized);
    const active = h.active_tx orelse return code(.tx_not_active);
    const given = asTx(tx) orelse return code(.tx_not_active);
    if (given != active) return code(.tx_not_active);
    // TODO(product): undo the staged operations in reverse order.
    h.allocator.destroy(active);
    h.active_tx = null;
    return code(.ok);
}

// ============================================================
// Version
// ============================================================

/// Return a static, null-terminated version string. Typed as a C pointer
/// (`[*c]const u8`, i.e. C's nullable `const char*`) so the conformance suite
/// can both null-check it and `std.mem.span` it; the scaffold always returns
/// the non-null static `VERSION`.
pub export fn jk_version() [*c]const u8 {
    return VERSION.ptr;
}

// ============================================================
// In-module unit tests (kept; the conformance suite lives in
// test/integration_test.zig and imports this module as "januskey").
// ============================================================

test "layout sizes match the C ABI" {
    try std.testing.expectEqual(@as(usize, 32), @sizeOf(ContentHash));
    try std.testing.expectEqual(@as(usize, 16), @sizeOf(KeyId));
    try std.testing.expectEqual(@as(usize, 112), @sizeOf(OblitProof));
    try std.testing.expect(@alignOf(OblitProof) >= 8);
}

test "error codes match header" {
    try std.testing.expectEqual(@as(c_int, 0), code(Error.ok));
    try std.testing.expectEqual(@as(c_int, 2), code(Error.invalid_path));
    try std.testing.expectEqual(@as(c_int, 11), code(Error.buffer_too_small));
}

test "init then close round-trips a handle" {
    var handle: ?*anyopaque = null;
    try std.testing.expectEqual(@as(c_int, 0), jk_init("/tmp/jk-unit", &handle));
    try std.testing.expect(handle != null);
    jk_close(handle);
}

test "transaction conflict on double begin" {
    var handle: ?*anyopaque = null;
    _ = jk_init("/tmp/jk-unit-tx", &handle);
    defer jk_close(handle);

    var tx: ?*anyopaque = null;
    try std.testing.expectEqual(@as(c_int, 0), jk_tx_begin(handle, &tx));

    var tx2: ?*anyopaque = null;
    try std.testing.expectEqual(@as(c_int, 6), jk_tx_begin(handle, &tx2));

    try std.testing.expectEqual(@as(c_int, 0), jk_tx_commit(handle, tx));
}
