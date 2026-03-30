// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
//
// Integration tests for JanusKey Zig FFI
// Tests: init/close, execute/undo, transactions, obliteration, error codes

const std = @import("std");
const jk = @import("../src/main.zig");

// ============================================================
// Layout verification (compile-time, matches Layout.idr)
// ============================================================

test "ContentHash is 32 bytes" {
    try std.testing.expectEqual(@as(usize, 32), @sizeOf(jk.ContentHash));
}

test "KeyId is 16 bytes" {
    try std.testing.expectEqual(@as(usize, 16), @sizeOf(jk.KeyId));
}

test "OblitProof is 112 bytes, 8-byte aligned" {
    try std.testing.expectEqual(@as(usize, 112), @sizeOf(jk.OblitProof));
    try std.testing.expect(@alignOf(jk.OblitProof) >= 8);
}

// ============================================================
// Error code verification (matches Foreign.idr)
// ============================================================

test "error codes match C header constants" {
    try std.testing.expectEqual(@as(c_int, 0), @intFromEnum(jk.Error.ok));
    try std.testing.expectEqual(@as(c_int, 1), @intFromEnum(jk.Error.not_initialized));
    try std.testing.expectEqual(@as(c_int, 2), @intFromEnum(jk.Error.invalid_path));
    try std.testing.expectEqual(@as(c_int, 3), @intFromEnum(jk.Error.io_error));
    try std.testing.expectEqual(@as(c_int, 4), @intFromEnum(jk.Error.crypto_error));
    try std.testing.expectEqual(@as(c_int, 5), @intFromEnum(jk.Error.tx_not_active));
    try std.testing.expectEqual(@as(c_int, 6), @intFromEnum(jk.Error.tx_conflict));
    try std.testing.expectEqual(@as(c_int, 7), @intFromEnum(jk.Error.key_not_found));
    try std.testing.expectEqual(@as(c_int, 8), @intFromEnum(jk.Error.key_revoked));
    try std.testing.expectEqual(@as(c_int, 9), @intFromEnum(jk.Error.obliteration_error));
    try std.testing.expectEqual(@as(c_int, 10), @intFromEnum(jk.Error.attestation_error));
    try std.testing.expectEqual(@as(c_int, 11), @intFromEnum(jk.Error.buffer_too_small));
}

// ============================================================
// Init/close lifecycle
// ============================================================

test "init with valid path succeeds" {
    var handle: ?*anyopaque = null;
    const result = jk.jk_init("/tmp/jk-test-init", &handle);
    try std.testing.expectEqual(@as(c_int, 0), result);
    try std.testing.expect(handle != null);
    jk.jk_close(handle);
}

test "init with null path fails" {
    var handle: ?*anyopaque = null;
    const result = jk.jk_init(null, &handle);
    try std.testing.expectEqual(@as(c_int, 2), result); // JK_ERR_INVALID_PATH
}

test "close null handle is safe" {
    jk.jk_close(null); // should not crash
}

// ============================================================
// Transaction lifecycle
// ============================================================

test "transaction begin-commit lifecycle" {
    var handle: ?*anyopaque = null;
    _ = jk.jk_init("/tmp/jk-test-tx", &handle);
    defer jk.jk_close(handle);

    var tx: ?*anyopaque = null;
    const begin_result = jk.jk_tx_begin(handle, &tx);
    try std.testing.expectEqual(@as(c_int, 0), begin_result);

    const commit_result = jk.jk_tx_commit(handle, tx);
    try std.testing.expectEqual(@as(c_int, 0), commit_result);
}

test "transaction begin-rollback lifecycle" {
    var handle: ?*anyopaque = null;
    _ = jk.jk_init("/tmp/jk-test-tx-rb", &handle);
    defer jk.jk_close(handle);

    var tx: ?*anyopaque = null;
    _ = jk.jk_tx_begin(handle, &tx);

    const rollback_result = jk.jk_tx_rollback(handle, tx);
    try std.testing.expectEqual(@as(c_int, 0), rollback_result);
}

test "double begin fails with tx_conflict" {
    var handle: ?*anyopaque = null;
    _ = jk.jk_init("/tmp/jk-test-tx-double", &handle);
    defer jk.jk_close(handle);

    var tx: ?*anyopaque = null;
    _ = jk.jk_tx_begin(handle, &tx);

    var tx2: ?*anyopaque = null;
    const second = jk.jk_tx_begin(handle, &tx2);
    try std.testing.expectEqual(@as(c_int, 6), second); // JK_ERR_TX_CONFLICT
}

test "commit without begin fails" {
    var handle: ?*anyopaque = null;
    _ = jk.jk_init("/tmp/jk-test-tx-nobegin", &handle);
    defer jk.jk_close(handle);

    const result = jk.jk_tx_commit(handle, null);
    try std.testing.expectEqual(@as(c_int, 5), result); // JK_ERR_TX_NOT_ACTIVE
}

// ============================================================
// Null handle guards
// ============================================================

test "execute with null handle returns not_initialized" {
    const result = jk.jk_execute(null, 0, "src", "dst");
    try std.testing.expectEqual(@as(c_int, 1), result);
}

test "undo with null handle returns not_initialized" {
    const result = jk.jk_undo(null);
    try std.testing.expectEqual(@as(c_int, 1), result);
}

test "obliterate with null handle returns not_initialized" {
    const result = jk.jk_obliterate(null, "/tmp/x", null);
    try std.testing.expectEqual(@as(c_int, 1), result);
}

// ============================================================
// Version
// ============================================================

test "version returns non-null string" {
    const ver = jk.jk_version();
    try std.testing.expect(ver != null);
    const slice = std.mem.span(ver);
    try std.testing.expect(slice.len > 0);
}
