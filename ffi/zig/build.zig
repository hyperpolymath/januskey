// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // Static library for C/Rust consumers
    const lib = b.addStaticLibrary(.{
        .name = "januskey-ffi",
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    // main.zig uses std.heap.c_allocator, which requires libc; the library
    // is consumed from C anyway.
    lib.linkLibC();
    b.installArtifact(lib);

    // Install C header
    lib.installHeader(b.path("include/januskey.h"), "januskey.h");

    // Integration tests
    const tests = b.addTest(.{
        .root_source_file = b.path("test/integration_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    tests.linkLibC();
    // Expose src/main.zig to the tests as @import("januskey") — relative
    // imports outside test/ are rejected by the module system.
    tests.root_module.addAnonymousImport("januskey", .{
        .root_source_file = b.path("src/main.zig"),
    });
    const run_tests = b.addRunArtifact(tests);
    const test_step = b.step("test", "Run integration tests");
    test_step.dependOn(&run_tests.step);
}
