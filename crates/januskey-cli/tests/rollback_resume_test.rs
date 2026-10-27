// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// PROOF-NEEDS T-4: a rollback that fails part-way can be retried to completion.
//
// `jk rollback` undoes a transaction's operations last-first. When one undo
// fails it stops with the transaction still Active and the later operations
// already undone. The retry must skip those and finish; before the fix it
// stopped at the first, already undone, operation, every time.
//
// Driven through the real `jk` binary, because the fault is in `cmd_rollback`.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn jk(dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("jk").expect("jk binary");
    cmd.arg("--yes").arg("--dir").arg(dir);
    cmd
}

/// A repo with an Active transaction holding two operations, oldest first:
/// 1. `copy a.txt b.txt` (undone by deleting b.txt)
/// 2. `modify c.txt` (undone by restoring "one")
///
/// Rollback undoes 2 first, then 1.
fn repo_with_two_operations() -> TempDir {
    let tmp = TempDir::new().expect("temp dir");
    let dir = tmp.path();
    jk(dir).arg("init").assert().success();
    fs::write(dir.join("a.txt"), "alpha").expect("a.txt");
    fs::write(dir.join("c.txt"), "one").expect("c.txt");

    jk(dir).args(["begin", "t"]).assert().success();
    jk(dir).args(["copy", "a.txt", "b.txt"]).assert().success();
    jk(dir)
        .args(["modify", "s/one/two/g", "c.txt"])
        .assert()
        .success();
    assert_eq!(fs::read_to_string(dir.join("c.txt")).unwrap(), "two");
    assert!(dir.join("b.txt").exists());
    tmp
}

#[test]
fn rollback_retry_completes_after_a_partial_failure() {
    let tmp = repo_with_two_operations();
    let dir = tmp.path();

    // Break the second undo: b.txt is gone, so deleting it to undo the copy fails.
    fs::remove_file(dir.join("b.txt")).expect("remove b.txt");
    jk(dir).arg("rollback").assert().failure();

    // The undo of the modify (run first) took effect; the transaction is still Active.
    assert_eq!(fs::read_to_string(dir.join("c.txt")).unwrap(), "one");
    jk(dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Active"));

    // Repair the problem and retry: it must finish, not stop at the modify.
    fs::write(dir.join("b.txt"), "alpha").expect("restore b.txt");
    jk(dir)
        .arg("rollback")
        .assert()
        .success()
        .stdout(predicate::str::contains("Rolled back transaction"));

    assert!(!dir.join("b.txt").exists(), "the copy is undone");
    assert_eq!(fs::read_to_string(dir.join("c.txt")).unwrap(), "one");
    assert_eq!(fs::read_to_string(dir.join("a.txt")).unwrap(), "alpha");

    // The transaction is settled: nothing left to roll back.
    jk(dir)
        .arg("rollback")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No active transaction"));
}

#[test]
fn rollback_does_not_undo_an_operation_twice() {
    let tmp = repo_with_two_operations();
    let dir = tmp.path();

    fs::remove_file(dir.join("b.txt")).expect("remove b.txt");
    jk(dir).arg("rollback").assert().failure();

    // The user edits c.txt after the failed attempt. The retry must leave it
    // alone: its operation was already undone, and re-undoing would clobber it.
    fs::write(dir.join("c.txt"), "edited by hand").expect("hand edit");
    fs::write(dir.join("b.txt"), "alpha").expect("restore b.txt");
    jk(dir).arg("rollback").assert().success();

    assert_eq!(
        fs::read_to_string(dir.join("c.txt")).unwrap(),
        "edited by hand"
    );
}
