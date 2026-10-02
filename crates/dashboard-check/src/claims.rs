// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// Claims ledger (ULTRAPLAN P1-0): every claim the repo makes about itself is a
// row `claim | status | artefact | command` in the `=== Claims ledger` table of
// PROOF-NEEDS.adoc. Status is one of PROVEN / TESTED / ASSUMED / DESIGNED /
// OPEN. This module checks that each row is discharged by its artefact, and
// that the counts printed on the dashboards equal what was measured.
//
// Rules, each of which has a fixture below:
// - One row per line. A row that wraps onto a second line is rejected,
//   because a parser that guesses where a wrapped row ends can be lied to.
// - TESTED: the artefact is `<file>::<test path>`. The file must exist and
//   name the test, the command must exit 0, and some output line must name the
//   test and say `ok` or `PASS`. A command that runs nothing (a filter that
//   matches no test, or plain `true`) therefore fails.
// - PROVEN: the artefact is `<file>::<theorem>`. The file must exist and name
//   the theorem, and the command (the checker run) must exit 0.
// - ASSUMED / DESIGNED / OPEN: nothing is run, so the command cell must be `-`.
//   The artefact must be an existing file or an issue reference `#N`.
// - Dashboard counts: a number followed (within two words) by "test(s)" must
//   equal the measured test count; one followed by "proof(s)" or "theorem(s)"
//   must equal the number of PROVEN rows.

use std::path::Path;
use std::process::Command;

/// The five statuses a claim may carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Proven,
    Tested,
    Assumed,
    Designed,
    Open,
}

impl Status {
    /// Parse a status cell; `None` for anything outside the taxonomy.
    fn parse(cell: &str) -> Option<Status> {
        match cell {
            "PROVEN" => Some(Status::Proven),
            "TESTED" => Some(Status::Tested),
            "ASSUMED" => Some(Status::Assumed),
            "DESIGNED" => Some(Status::Designed),
            "OPEN" => Some(Status::Open),
            _ => None,
        }
    }
}

/// One ledger row, with the PROOF-NEEDS line it came from for error messages.
#[derive(Debug, Clone, PartialEq)]
pub struct Claim {
    pub line: usize,
    pub claim: String,
    pub status: Status,
    pub artefact: String,
    pub command: String,
}

/// Strip AsciiDoc inline-literal markup (`` `x` `` or `` `+x+` ``) from a cell.
fn unliteral(cell: &str) -> String {
    let c = cell.trim();
    let c = c
        .strip_prefix('`')
        .and_then(|s| s.strip_suffix('`'))
        .unwrap_or(c);
    let c = c
        .strip_prefix('+')
        .and_then(|s| s.strip_suffix('+'))
        .unwrap_or(c);
    c.trim().to_string()
}

/// Parse the `=== Claims ledger` table out of PROOF-NEEDS.adoc. Errors name
/// the offending line; a missing section or table is an error, not an empty
/// ledger.
pub fn parse_ledger(src: &str) -> Result<Vec<Claim>, Vec<String>> {
    let lines: Vec<&str> = src.lines().collect();
    let Some(heading) = lines.iter().position(|l| l.trim() == "=== Claims ledger") else {
        return Err(vec![
            "PROOF-NEEDS.adoc has no '=== Claims ledger' section".into()
        ]);
    };
    let Some(open) = lines[heading..]
        .iter()
        .position(|l| l.trim() == "|===")
        .map(|i| heading + i)
    else {
        return Err(vec!["'=== Claims ledger' has no |=== table".into()]);
    };

    let mut claims = Vec::new();
    let mut errors = Vec::new();
    let mut header_seen = false;
    let mut closed = false;
    for (i, raw) in lines.iter().enumerate().skip(open + 1) {
        let n = i + 1;
        let l = raw.trim();
        if l == "|===" {
            closed = true;
            break;
        }
        if l.is_empty() {
            continue;
        }
        let Some(body) = l.strip_prefix('|') else {
            errors.push(format!(
                "PROOF-NEEDS.adoc:{n}: ledger rows must fit on one line starting with '|' (wrapped row?)"
            ));
            continue;
        };
        let cells: Vec<String> = body.split('|').map(unliteral).collect();
        if cells.len() != 4 {
            errors.push(format!(
                "PROOF-NEEDS.adoc:{n}: expected 4 cells (claim | status | artefact | command), found {}",
                cells.len()
            ));
            continue;
        }
        if !header_seen {
            header_seen = true;
            continue;
        }
        let Some(status) = Status::parse(&cells[1]) else {
            errors.push(format!(
                "PROOF-NEEDS.adoc:{n}: status '{}' is not PROVEN / TESTED / ASSUMED / DESIGNED / OPEN",
                cells[1]
            ));
            continue;
        };
        claims.push(Claim {
            line: n,
            claim: cells[0].clone(),
            status,
            artefact: cells[2].clone(),
            command: cells[3].clone(),
        });
    }
    if !closed {
        errors.push("the claims ledger table is not closed with |===".into());
    }
    if errors.is_empty() && claims.is_empty() {
        errors.push("the claims ledger has no rows".into());
    }
    if errors.is_empty() {
        Ok(claims)
    } else {
        Err(errors)
    }
}

/// True when `needle` occurs in `hay` with no identifier character on either
/// side, so `foo` does not match inside `foo_bar` or `xfoo`.
fn contains_word(hay: &str, needle: &str) -> bool {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    hay.match_indices(needle).any(|(i, _)| {
        let before = hay[..i].chars().next_back();
        let after = hay[i + needle.len()..].chars().next();
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

/// Split a `<file>::<name>` artefact; `None` if it has no `::`.
fn split_artefact(artefact: &str) -> Option<(&str, &str)> {
    artefact
        .split_once("::")
        .filter(|(f, n)| !f.is_empty() && !n.is_empty())
}

/// Check that the artefact file exists under `root` and names the last
/// `::` segment of `name` as a whole word.
fn artefact_present(root: &Path, file: &str, name: &str) -> Result<(), String> {
    let leaf = name.rsplit("::").next().unwrap_or(name);
    let text = std::fs::read_to_string(root.join(file))
        .map_err(|e| format!("artefact file {file} cannot be read: {e}"))?;
    if contains_word(&text, leaf) {
        Ok(())
    } else {
        Err(format!("artefact file {file} does not mention '{leaf}'"))
    }
}

/// Run a ledger command with `sh -c` in `root`. Returns whether it exited 0
/// and its combined stdout and stderr. Failing to start is an error.
pub fn run_command(root: &Path, command: &str) -> Result<(bool, String), String> {
    let out = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot start `{command}`: {e}"))?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok((out.status.success(), text))
}

/// True when some output line names the test and reports it passed
/// (`test <name> ... ok` from libtest, `[PASS] <name>` from shell harnesses).
fn output_shows_pass(output: &str, name: &str) -> bool {
    output
        .lines()
        .any(|l| contains_word(l, name) && (contains_word(l, "ok") || contains_word(l, "PASS")))
}

/// Check one claim against its artefact, running its command when the status
/// requires it. `run` is injected so the rules can be tested without cargo.
pub fn check_claim<F>(root: &Path, c: &Claim, run: &F) -> Result<(), String>
where
    F: Fn(&Path, &str) -> Result<(bool, String), String>,
{
    let at = format!("PROOF-NEEDS.adoc:{} ({})", c.line, c.claim);
    match c.status {
        Status::Tested | Status::Proven => {
            let (file, name) = split_artefact(&c.artefact)
                .ok_or_else(|| format!("{at}: artefact '{}' must be <file>::<name>", c.artefact))?;
            artefact_present(root, file, name).map_err(|e| format!("{at}: {e}"))?;
            if c.command.is_empty() || c.command == "-" {
                return Err(format!("{at}: a {:?} claim needs a command", c.status));
            }
            let (ok, output) = run(root, &c.command).map_err(|e| format!("{at}: {e}"))?;
            if !ok {
                return Err(format!("{at}: `{}` failed", c.command));
            }
            if c.status == Status::Tested && !output_shows_pass(&output, name) {
                return Err(format!(
                    "{at}: `{}` exited 0 but no output line shows '{name}' passing (did it run the test at all?)",
                    c.command
                ));
            }
            Ok(())
        }
        Status::Assumed | Status::Designed | Status::Open => {
            if c.command != "-" {
                return Err(format!(
                    "{at}: a {:?} claim runs nothing, so its command cell must be '-'",
                    c.status
                ));
            }
            let a = c.artefact.as_str();
            let is_issue = a
                .strip_prefix('#')
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
            let file = split_artefact(a).map_or(a, |(f, _)| f);
            if is_issue || root.join(file).is_file() {
                Ok(())
            } else {
                Err(format!(
                    "{at}: artefact '{a}' is neither an existing file nor an issue reference #N"
                ))
            }
        }
    }
}

/// What a dashboard number counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountKind {
    Tests,
    Proofs,
}

/// Find `<number> [word] tests|proofs|theorems` claims in a dashboard. A
/// number must be a token of its own, so `Idris2 proofs` is not a claim of 2.
pub fn scan_counts(text: &str) -> Vec<(usize, u64, CountKind)> {
    let norm = |t: &str| -> String {
        t.trim_matches(|c: char| !c.is_ascii_alphanumeric())
            .to_ascii_lowercase()
    };
    let kind = |w: &str| match w {
        "test" | "tests" => Some(CountKind::Tests),
        "proof" | "proofs" | "theorem" | "theorems" => Some(CountKind::Proofs),
        _ => None,
    };
    let mut found = Vec::new();
    for (i, line) in text.lines().enumerate() {
        // Table cells are scanned separately, so a row number in one cell
        // cannot pair with a "Proof …" heading in the next.
        for cell in line.split('|') {
            // A percentage is not a count, so `%` tokens are blanked out.
            let toks: Vec<String> = cell
                .split_whitespace()
                .map(|t| {
                    if t.contains('%') {
                        String::new()
                    } else {
                        norm(t)
                    }
                })
                .collect();
            let is_num = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
            for (j, t) in toks.iter().enumerate() {
                if !is_num(t) {
                    continue;
                }
                let Ok(n) = t.parse::<u64>() else { continue };
                // Look at most two words ahead, stopping at the next number.
                let k = toks[j + 1..]
                    .iter()
                    .take(2)
                    .take_while(|w| !is_num(w))
                    .find_map(|w| kind(w));
                if let Some(k) = k {
                    found.push((i + 1, n, k));
                }
            }
        }
    }
    found
}

/// Compare every count on a dashboard with the measured values.
pub fn check_counts(
    surface: &str,
    text: &str,
    measured_tests: u64,
    proven_rows: u64,
) -> Vec<String> {
    scan_counts(text)
        .into_iter()
        .filter_map(|(line, n, k)| {
            let (want, what) = match k {
                CountKind::Tests => (
                    measured_tests,
                    "tests measured by `cargo test --workspace --locked -- --list`",
                ),
                CountKind::Proofs => (proven_rows, "PROVEN rows in the claims ledger"),
            };
            (n != want).then(|| format!("{surface}:{line}: claims {n} but there are {want} {what}"))
        })
        .collect()
}

/// Count the workspace's tests: the lines of `cargo test --workspace --locked
/// -- --list` that end in `: test`. Zero is an error, since a workspace with
/// tests that lists none means the instrument failed.
pub fn measure_tests(root: &Path) -> Result<u64, String> {
    let out = Command::new("cargo")
        .args(["test", "--workspace", "--locked", "--", "--list"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run cargo to measure the test count: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`cargo test --workspace --locked -- --list` failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let n = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.ends_with(": test"))
        .count() as u64;
    if n == 0 {
        return Err(
            "`cargo test -- --list` listed 0 tests; refusing to treat that as a measurement".into(),
        );
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEDGER: &str = "\
=== Claims ledger

[cols=\"3,1,3,3\",options=\"header\"]
|===
|Claim |Status |Artefact |Command
|Undo restores content |TESTED |`src/a.rs::undo_restores` |`cargo test undo_restores`
|Attestation is unforgeable |OPEN |#145 |-
|===
";

    /// A fake runner that returns a fixed exit status and output.
    fn fake(ok: bool, out: &'static str) -> impl Fn(&Path, &str) -> Result<(bool, String), String> {
        move |_: &Path, _: &str| Ok((ok, out.to_string()))
    }

    /// A temp dir holding `src/a.rs` with a test named `undo_restores`.
    fn repo(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("claims-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("src/a.rs"), "#[test]\nfn undo_restores() {}\n").unwrap();
        d
    }

    /// The TESTED row of [`LEDGER`].
    fn tested_row() -> Claim {
        parse_ledger(LEDGER).unwrap().remove(0)
    }

    #[test]
    fn parses_rows_and_strips_literals() {
        let c = parse_ledger(LEDGER).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].status, Status::Tested);
        assert_eq!(c[0].artefact, "src/a.rs::undo_restores");
        assert_eq!(c[0].command, "cargo test undo_restores");
        assert_eq!(c[1].status, Status::Open);
    }

    #[test]
    fn wrapped_row_is_rejected() {
        let wrapped = LEDGER.replace(
            "|`cargo test undo_restores`",
            "\n`cargo test undo_restores`",
        );
        let e = parse_ledger(&wrapped).unwrap_err();
        assert!(e.iter().any(|e| e.contains("wrapped row")), "{e:?}");
    }

    #[test]
    fn unknown_status_and_missing_section_are_rejected() {
        let e = parse_ledger(&LEDGER.replace("|OPEN |", "|DONE |")).unwrap_err();
        assert!(e.iter().any(|e| e.contains("'DONE'")), "{e:?}");
        assert!(parse_ledger("= nothing here\n").is_err());
    }

    #[test]
    fn tested_row_passes_when_output_shows_the_test() {
        let d = repo("pass");
        let r = check_claim(
            &d,
            &tested_row(),
            &fake(true, "test undo_restores ... ok\n"),
        );
        std::fs::remove_dir_all(&d).unwrap();
        assert_eq!(r, Ok(()));
    }

    // PROVEN needs the theorem in the file and a passing checker run; unlike
    // TESTED it needs no per-name output, since provers print nothing on success.
    #[test]
    fn proven_row_needs_theorem_and_passing_checker() {
        let d = repo("proven");
        std::fs::write(
            d.join("src/P.idr"),
            "undoInverse : (x : S) -> undo (op x) = x\n",
        )
        .unwrap();
        let mut c = tested_row();
        c.status = Status::Proven;
        c.artefact = "src/P.idr::undoInverse".into();
        c.command = "idris2 --check src/P.idr".into();
        let ok = check_claim(&d, &c, &fake(true, ""));
        let rejected = check_claim(&d, &c, &fake(false, "error"));
        c.artefact = "src/P.idr::undoInverseAll".into();
        let absent = check_claim(&d, &c, &fake(true, ""));
        std::fs::remove_dir_all(&d).unwrap();
        assert_eq!(ok, Ok(()));
        assert!(rejected.unwrap_err().contains("failed"));
        assert!(absent.unwrap_err().contains("does not mention"));
    }

    // LyingVerifier: the command exits 0 but never ran the test.
    #[test]
    fn lying_verifier_fails() {
        let d = repo("liar");
        let r = check_claim(&d, &tested_row(), &fake(true, ""));
        std::fs::remove_dir_all(&d).unwrap();
        assert!(r.unwrap_err().contains("no output line shows"));
    }

    // UncheckedSkip: a filter that matches nothing is `running 0 tests`, rc 0.
    #[test]
    fn unchecked_skip_fails() {
        let d = repo("skip");
        let out = "running 0 tests\n\ntest result: ok. 0 passed; 0 failed\n";
        let r = check_claim(&d, &tested_row(), &fake(true, out));
        std::fs::remove_dir_all(&d).unwrap();
        assert!(r.unwrap_err().contains("no output line shows"));
    }

    #[test]
    fn failing_command_and_missing_artefact_fail() {
        let d = repo("fail");
        let failed = check_claim(&d, &tested_row(), &fake(false, "test undo_restores ... ok"));
        let mut gone = tested_row();
        gone.artefact = "src/a.rs::no_such_test".into();
        let absent = check_claim(&d, &gone, &fake(true, "test no_such_test ... ok"));
        std::fs::remove_dir_all(&d).unwrap();
        assert!(failed.unwrap_err().contains("failed"));
        assert!(absent.unwrap_err().contains("does not mention"));
    }

    #[test]
    fn real_shell_runner_reports_status_and_output() {
        let d = std::env::temp_dir();
        assert_eq!(run_command(&d, "echo hi").unwrap(), (true, "hi\n".into()));
        assert!(!run_command(&d, "exit 3").unwrap().0);
    }

    #[test]
    fn open_rows_run_nothing_and_need_a_real_artefact() {
        let d = repo("open");
        let mut c = parse_ledger(LEDGER).unwrap().remove(1);
        let ok = check_claim(&d, &c, &fake(false, ""));
        c.command = "cargo test".into();
        let ran = check_claim(&d, &c, &fake(true, ""));
        c.command = "-".into();
        c.artefact = "docs/missing.adoc".into();
        let missing = check_claim(&d, &c, &fake(true, ""));
        std::fs::remove_dir_all(&d).unwrap();
        assert_eq!(ok, Ok(()));
        assert!(ran.unwrap_err().contains("must be '-'"));
        assert!(missing.unwrap_err().contains("neither an existing file"));
    }

    #[test]
    fn scans_counts_but_not_digits_inside_words() {
        let t = "67 tests + 5 benchmark groups + 30 Idris2 proofs\nIdris2 proofs not checked\n";
        assert_eq!(
            scan_counts(t),
            vec![(1, 67, CountKind::Tests), (1, 30, CountKind::Proofs)]
        );
        assert!(scan_counts("|16 |Proof regression |✓ |`just test-proofs`").is_empty());
    }

    // Inflated count: the dashboard says 67, the instrument measured 107.
    #[test]
    fn inflated_counts_fail_and_true_counts_pass() {
        let p = check_counts(
            "TOPOLOGY.adoc",
            "~60%   67 tests + 5 benches\n30 Idris2 proofs\n",
            107,
            0,
        );
        assert_eq!(p.len(), 2, "{p:?}");
        assert!(p[0].contains("TOPOLOGY.adoc:1: claims 67 but there are 107"));
        assert!(p[1].contains("claims 30 but there are 0 PROVEN rows"));
        assert!(check_counts("T", "107 tests; 0 Idris2 proofs PROVEN\n", 107, 0).is_empty());
    }
}
