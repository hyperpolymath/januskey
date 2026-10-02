// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// dashboard-check — reconcile hand-maintained status dashboards against the
// machine-readable STATE.a2ml (the declared source of truth).
//
// Motivation: the estate's #1 recurring defect is "dashboards that lie" — a
// human-facing surface (TOPOLOGY completion bar, README badge, READINESS
// grade) that claims more than STATE.a2ml records. This tool reads STATE and
// asserts the dashboards agree; run in CI it makes divergence a build failure,
// so the lie cannot be committed silently.
//
// A check that compares nothing is itself a lying dashboard: every surface and
// every field this tool reconciles must be present and parseable, otherwise
// the run fails. (Before this rule, renaming TOPOLOGY.md to TOPOLOGY.adoc made
// the tool print "OK" while comparing nothing.)
//
// `.a2ml` is TOML (the estate parses it with a TOML parser elsewhere), so we
// parse it with the `toml` crate rather than a bespoke reader.
//
// Usage:
//     dashboard-check [--check] [REPO_ROOT]
// Exits 0 only if every reconciled field was found and agrees with STATE;
// non-zero with a report otherwise. `--check` is the default and only mode.

mod claims;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Facts extracted from STATE.a2ml (the source of truth).
#[derive(Debug, Default, PartialEq)]
struct StateFacts {
    completion: Option<u32>,
    grade: Option<String>,
    last_updated: Option<String>,
}

/// Parse STATE.a2ml (TOML). Numbers may be quoted (`"60"`) or bare (`60`), and
/// the grade may live under `[metadata].crg-grade` or `[crg-compliance].tier`.
fn extract_state(toml_src: &str) -> Result<StateFacts, String> {
    let doc: toml::Table = toml_src
        .parse()
        .map_err(|e| format!("STATE.a2ml is not valid TOML: {e}"))?;

    let get = |section: &str, key: &str| -> Option<toml::Value> {
        doc.get(section)
            .and_then(|s| s.as_table())
            .and_then(|t| t.get(key))
            .cloned()
    };

    // completion-percentage may sit under [project-context] or [position].
    let completion = ["project-context", "position", "metadata"]
        .iter()
        .find_map(|sec| get(sec, "completion-percentage"))
        .and_then(|v| value_to_u32(&v));

    // grade: [metadata].crg-grade first, else [crg-compliance].tier.
    let grade = get("metadata", "crg-grade")
        .and_then(|v| v.as_str().map(str::to_string))
        .or_else(|| get("crg-compliance", "tier").and_then(|v| v.as_str().map(str::to_string)));

    let last_updated = get("metadata", "last-updated").and_then(|v| v.as_str().map(str::to_string));

    Ok(StateFacts {
        completion,
        grade,
        last_updated,
    })
}

/// Coerce a TOML value (string `"60"` or integer `60`) into a percentage.
fn value_to_u32(v: &toml::Value) -> Option<u32> {
    match v {
        toml::Value::Integer(i) => u32::try_from(*i).ok(),
        toml::Value::String(s) => s.trim().trim_end_matches('%').parse().ok(),
        _ => None,
    }
}

/// A dashboard file as read from disk: the name it was found under (so the
/// report says `TOPOLOGY.adoc`, not a guess) and its text.
struct Surface {
    name: String,
    text: String,
}

/// Read `<stem>.adoc`, falling back to `<stem>.md`. `None` if neither exists.
fn read_surface(root: &Path, stem: &str) -> Option<Surface> {
    ["adoc", "md"].iter().find_map(|ext| {
        let name = format!("{stem}.{ext}");
        read_opt(&root.join(&name)).map(|text| Surface { name, text })
    })
}

/// Locate STATE.a2ml: the canonical `descriptiles/` directory first, then the
/// retired `6a2/` name that this repo still uses.
fn state_path(root: &Path) -> PathBuf {
    let canonical = root.join(".machine_readable/descriptiles/STATE.a2ml");
    if canonical.exists() {
        canonical
    } else {
        root.join(".machine_readable/6a2/STATE.a2ml")
    }
}

/// The TOPOLOGY completion-dashboard line: the first line containing
/// "OVERALL" that also carries a `<digits>%` figure, so prose that merely
/// mentions "OVERALL" (e.g. the source-of-truth note) does not shadow it.
fn extract_overall_line(topology: &str) -> Option<&str> {
    topology
        .lines()
        .find(|l| l.contains("OVERALL") && first_percent(l).is_some())
}

/// The `OVERALL: ... ~60%` figure inside the TOPOLOGY completion dashboard.
fn extract_overall_pct(topology: &str) -> Option<u32> {
    extract_overall_line(topology).and_then(first_percent)
}

/// The grade printed on the OVERALL line (`... ~60%   Grade D — Alpha`).
/// Read from that line only, so a "Grade" mentioned in prose elsewhere in
/// the file cannot stand in for the dashboard's own grade.
fn extract_overall_grade(topology: &str) -> Option<String> {
    extract_overall_line(topology).and_then(|l| extract_grade_after_token(l, "Grade "))
}

/// First `<digits>%` occurrence in a string, as an integer.
fn first_percent(s: &str) -> Option<u32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'%' {
                return s[start..i].parse().ok();
            }
        } else {
            i += 1;
        }
    }
    None
}

/// The grade letter after a `Grade ` token (e.g. "Grade D — Alpha").
fn extract_grade_after_token(text: &str, token: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(idx) = line.find(token) {
            let rest = line[idx + token.len()..].trim_start();
            let g: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphabetic())
                .collect();
            if !g.is_empty() {
                return Some(g);
            }
        }
    }
    None
}

/// READINESS grade: AsciiDoc `*Current Grade:* D`, Markdown
/// `**Current Grade:** D`, or a `CRG Grade: D (...)` heading.
fn extract_grade_readiness(readiness: &str) -> Option<String> {
    extract_grade_after_token(readiness, "Current Grade:* ")
        .or_else(|| extract_grade_after_token(readiness, "Current Grade:** "))
        .or_else(|| extract_grade_after_token(readiness, "CRG Grade: "))
}

/// The `Last updated: YYYY-MM-DD` date from TOPOLOGY, written either as an
/// AsciiDoc `// Last updated: …` comment or a Markdown `<!-- … -->` comment.
/// Lines that mention "Last updated" without a date (instructions) are skipped.
fn extract_last_updated(topology: &str) -> Option<String> {
    topology.lines().find_map(|line| {
        let idx = line.find("Last updated:")? + "Last updated:".len();
        let date: String = line[idx..]
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        (date.len() >= 8).then_some(date)
    })
}

/// Result of a reconciliation: what was compared (for an auditable "OK")
/// and what was wrong.
#[derive(Debug, Default)]
struct Report {
    observed: Vec<String>,
    problems: Vec<String>,
}

/// Compare STATE against the dashboards. Every field must be present on both
/// sides; a missing surface or an unparseable field is a problem, never a
/// skip. Pure so it is unit-testable.
fn reconcile(
    state: &StateFacts,
    topology: Option<&Surface>,
    readiness: Option<&Surface>,
) -> Report {
    let mut r = Report::default();

    let Some(topo) = topology else {
        r.problems
            .push("no TOPOLOGY.adoc or TOPOLOGY.md to reconcile against STATE".into());
        return finish_without_topology(state, readiness, r);
    };

    match (state.completion, extract_overall_pct(&topo.text)) {
        (None, _) => r
            .problems
            .push("STATE.a2ml has no completion-percentage".into()),
        (_, None) => r.problems.push(format!(
            "{} has no parseable 'OVERALL: …%' line to check against STATE",
            topo.name
        )),
        (Some(pct), Some(dpct)) if dpct != pct => r.problems.push(format!(
            "completion mismatch: STATE says {pct}% but {} OVERALL says {dpct}%",
            topo.name
        )),
        (Some(pct), Some(_)) => r
            .observed
            .push(format!("completion {pct}% ({})", topo.name)),
    }

    check_grade(
        state,
        &topo.name,
        extract_overall_grade(&topo.text),
        "OVERALL line",
        &mut r,
    );

    match (&state.last_updated, extract_last_updated(&topo.text)) {
        (None, _) => r
            .problems
            .push("STATE.a2ml has no [metadata].last-updated".into()),
        (_, None) => r.problems.push(format!(
            "{} has no 'Last updated: YYYY-MM-DD' line, so its staleness cannot be checked",
            topo.name
        )),
        // Lexicographic compare works for ISO YYYY-MM-DD dates.
        (Some(su), Some(du)) if du.as_str() < su.as_str() => r.problems.push(format!(
            "staleness: {} 'Last updated: {du}' predates STATE last-updated {su}",
            topo.name
        )),
        (Some(su), Some(du)) => r
            .observed
            .push(format!("{} last updated {du} >= STATE {su}", topo.name)),
    }

    finish_without_topology(state, readiness, r)
}

/// The READINESS half of [`reconcile`], shared by the path where TOPOLOGY is
/// missing so that both dashboards are always reported on.
fn finish_without_topology(
    state: &StateFacts,
    readiness: Option<&Surface>,
    mut r: Report,
) -> Report {
    match readiness {
        None => r
            .problems
            .push("no READINESS.adoc or READINESS.md to reconcile against STATE".into()),
        Some(read) => check_grade(
            state,
            &read.name,
            extract_grade_readiness(&read.text),
            "Current Grade",
            &mut r,
        ),
    }
    r
}

/// Compare one dashboard's grade with STATE's, recording a problem when
/// either side is missing or they differ.
fn check_grade(
    state: &StateFacts,
    surface: &str,
    found: Option<String>,
    field: &str,
    r: &mut Report,
) {
    match (&state.grade, found) {
        (None, _) => r
            .problems
            .push("STATE.a2ml has no crg-grade / crg-compliance tier".into()),
        (_, None) => r
            .problems
            .push(format!("{surface} has no parseable grade on its {field}")),
        (Some(grade), Some(g)) if &g != grade => r.problems.push(format!(
            "grade mismatch: STATE says {grade} but {surface} says Grade {g}"
        )),
        (Some(grade), Some(_)) => r.observed.push(format!("grade {grade} ({surface})")),
    }
}

/// Check the claims ledger in PROOF-NEEDS (ULTRAPLAN P1-0): run every PROVEN
/// and TESTED row, then compare the counts printed on the dashboards with the
/// measured test count and the number of PROVEN rows.
fn check_claims(root: &Path, r: &mut Report) {
    let Some(ledger) = read_surface(root, "PROOF-NEEDS") else {
        r.problems
            .push("no PROOF-NEEDS.adoc or PROOF-NEEDS.md holding the claims ledger".into());
        return;
    };
    let claims = match claims::parse_ledger(&ledger.text) {
        Ok(c) => c,
        Err(errors) => {
            r.problems.extend(errors);
            return;
        }
    };
    for c in &claims {
        match claims::check_claim(root, c, &claims::run_command) {
            Ok(()) => r
                .observed
                .push(format!("{:?}: {} ({})", c.status, c.claim, c.artefact)),
            Err(e) => r.problems.push(e),
        }
    }
    let proven = claims
        .iter()
        .filter(|c| c.status == claims::Status::Proven)
        .count() as u64;
    let tests = match claims::measure_tests(root) {
        Ok(n) => n,
        Err(e) => {
            r.problems.push(e);
            return;
        }
    };
    r.observed
        .push(format!("{tests} tests measured, {proven} PROVEN claims"));
    for stem in ["README", "EXPLAINME", "TOPOLOGY", "READINESS"] {
        if let Some(s) = read_surface(root, stem) {
            r.problems
                .extend(claims::check_counts(&s.name, &s.text, tests, proven));
        }
    }
}

/// Read a file to a string, `None` if it is absent or unreadable.
fn read_opt(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Entry point: reconcile the dashboards under REPO_ROOT (default `.`) and
/// exit 0 on agreement, 1 on divergence, 2 if STATE cannot be read or parsed.
fn main() -> ExitCode {
    // Skip the binary name; ignore the `--check` flag (default mode).
    let mut root = PathBuf::from(".");
    for arg in std::env::args().skip(1) {
        if arg == "--check" {
            continue;
        }
        root = PathBuf::from(arg);
    }

    let state_path = state_path(&root);
    let state_src = match read_opt(&state_path) {
        Some(s) => s,
        None => {
            eprintln!("dashboard-check: cannot read {}", state_path.display());
            return ExitCode::from(2);
        }
    };

    let state = match extract_state(&state_src) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("dashboard-check: {e}");
            return ExitCode::from(2);
        }
    };

    let topology = read_surface(&root, "TOPOLOGY");
    let readiness = read_surface(&root, "READINESS");

    let mut report = reconcile(&state, topology.as_ref(), readiness.as_ref());
    check_claims(&root, &mut report);

    if report.problems.is_empty() {
        println!(
            "dashboard-check: OK — dashboards agree with {}:",
            state_path.display()
        );
        for o in &report.observed {
            println!("  ✓ {o}");
        }
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "dashboard-check: {} divergence(s) from {} (the source of truth):",
            report.problems.len(),
            state_path.display()
        );
        for p in &report.problems {
            eprintln!("  ✗ {p}");
        }
        eprintln!("Fix the dashboard to match STATE, or update STATE if it is stale.");
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATE_60_D: &str = r#"
[metadata]
last-updated = "2026-06-12"
crg-grade = "D"
[project-context]
completion-percentage = "60"
"#;

    // maa-framework-style: bare integer, drift vs a 60% dashboard.
    const STATE_50_BARE: &str = r#"
[metadata]
last-updated = "2026-06-12"
crg-grade = "D"
[project-context]
completion-percentage = 50
"#;

    // Shapes copied from the real januskey TOPOLOGY.adoc / READINESS.adoc.
    const TOPOLOGY_ADOC: &str = "\
// Last updated: 2026-07-02 (completion dashboard reconciled to STATE.a2ml)
CRG grade *D*) and `+READINESS.md+` (Grade *D — Alpha, Unstable*). This
if the `+OVERALL+` percentage or grade here drifts from STATE.a2ml.
                                                        chaos, compatibility (Grade D)
OVERALL:                            ██████░░░░  ~60%   Grade D — Alpha, Unstable (not v1.0)
. *Date*: Update the `+Last updated+` comment at the top of this file
";

    const READINESS_ADOC: &str = "\
*Current Grade:* D

=== CRG Grade: D (Alpha — Unstable)
";

    /// Build a [`Surface`] for tests.
    fn surf(name: &str, text: &str) -> Surface {
        Surface {
            name: name.into(),
            text: text.into(),
        }
    }

    /// Reconcile the real-shaped fixtures, with one of them optionally
    /// replaced, and return the problem list.
    fn problems_with(state: &str, topo: Option<&str>, read: Option<&str>) -> Vec<String> {
        let s = extract_state(state).unwrap();
        let t = topo.map(|t| surf("TOPOLOGY.adoc", t));
        let r = read.map(|r| surf("READINESS.adoc", r));
        reconcile(&s, t.as_ref(), r.as_ref()).problems
    }

    #[test]
    fn parses_quoted_state_fields() {
        let s = extract_state(STATE_60_D).unwrap();
        assert_eq!(s.completion, Some(60));
        assert_eq!(s.grade.as_deref(), Some("D"));
        assert_eq!(s.last_updated.as_deref(), Some("2026-06-12"));
    }

    #[test]
    fn parses_bare_integer_completion() {
        let s = extract_state(STATE_50_BARE).unwrap();
        assert_eq!(s.completion, Some(50));
    }

    #[test]
    fn extracts_signals_from_real_adoc_shapes() {
        assert_eq!(extract_overall_pct(TOPOLOGY_ADOC), Some(60));
        assert_eq!(extract_overall_grade(TOPOLOGY_ADOC).as_deref(), Some("D"));
        assert_eq!(
            extract_grade_readiness(READINESS_ADOC).as_deref(),
            Some("D")
        );
        assert_eq!(
            extract_last_updated(TOPOLOGY_ADOC).as_deref(),
            Some("2026-07-02")
        );
    }

    #[test]
    fn reads_adoc_current_grade_without_a_crg_heading() {
        assert_eq!(
            extract_grade_readiness("*Current Grade:* C\n").as_deref(),
            Some("C")
        );
    }

    #[test]
    fn read_surface_finds_adoc_then_md() {
        let dir = std::env::temp_dir().join(format!("dashboard-check-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("TOPOLOGY.adoc"), "adoc").unwrap();
        std::fs::write(dir.join("READINESS.md"), "md").unwrap();
        let t = read_surface(&dir, "TOPOLOGY").expect("TOPOLOGY.adoc must be found");
        let r = read_surface(&dir, "READINESS").expect("READINESS.md fallback must be found");
        assert_eq!(
            (t.name.as_str(), t.text.as_str()),
            ("TOPOLOGY.adoc", "adoc")
        );
        assert_eq!((r.name.as_str(), r.text.as_str()), ("READINESS.md", "md"));
        assert!(read_surface(&dir, "ABSENT").is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn state_path_prefers_descriptiles_over_6a2() {
        let dir =
            std::env::temp_dir().join(format!("dashboard-check-state-{}", std::process::id()));
        let legacy = dir.join(".machine_readable/6a2");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("STATE.a2ml"), "").unwrap();
        assert!(state_path(&dir).ends_with("6a2/STATE.a2ml"));
        let canonical = dir.join(".machine_readable/descriptiles");
        std::fs::create_dir_all(&canonical).unwrap();
        std::fs::write(canonical.join("STATE.a2ml"), "").unwrap();
        assert!(state_path(&dir).ends_with("descriptiles/STATE.a2ml"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn extracts_markdown_shapes_too() {
        let md = "<!-- Last updated: 2026-07-02 -->\nOVERALL: ~60%   Grade D\n";
        assert_eq!(extract_last_updated(md).as_deref(), Some("2026-07-02"));
        assert_eq!(
            extract_grade_readiness("**Current Grade:** C\n").as_deref(),
            Some("C")
        );
    }

    #[test]
    fn passes_when_aligned_and_reports_what_it_compared() {
        let s = extract_state(STATE_60_D).unwrap();
        let t = surf("TOPOLOGY.adoc", TOPOLOGY_ADOC);
        let r = surf("READINESS.adoc", READINESS_ADOC);
        let report = reconcile(&s, Some(&t), Some(&r));
        assert!(
            report.problems.is_empty(),
            "expected no problems, got {:?}",
            report.problems
        );
        // completion, TOPOLOGY grade, staleness, READINESS grade.
        assert_eq!(report.observed.len(), 4, "{:?}", report.observed);
    }

    #[test]
    fn fails_on_completion_drift() {
        // The historical disease: STATE 50, dashboard 60.
        let p = problems_with(STATE_50_BARE, Some(TOPOLOGY_ADOC), Some(READINESS_ADOC));
        assert_eq!(p.len(), 1, "{p:?}");
        assert!(p[0].contains("completion mismatch"), "{p:?}");
    }

    #[test]
    fn fails_on_topology_grade_drift() {
        let bad = TOPOLOGY_ADOC.replace("~60%   Grade D", "~60%   Grade A");
        let p = problems_with(STATE_60_D, Some(&bad), Some(READINESS_ADOC));
        assert!(
            p.iter()
                .any(|p| p.contains("grade mismatch") && p.contains("TOPOLOGY")),
            "{p:?}"
        );
    }

    #[test]
    fn prose_grade_does_not_stand_in_for_the_overall_grade() {
        // OVERALL line without a grade; prose elsewhere still says "Grade D".
        let no_grade = TOPOLOGY_ADOC.replace("Grade D — Alpha, Unstable (not v1.0)", "");
        let p = problems_with(STATE_60_D, Some(&no_grade), Some(READINESS_ADOC));
        assert!(
            p.iter()
                .any(|p| p.contains("no parseable grade on its OVERALL line")),
            "{p:?}"
        );
    }

    #[test]
    fn fails_on_readiness_grade_drift() {
        let bad = READINESS_ADOC.replace(": D", ": B").replace("* D", "* B");
        let p = problems_with(STATE_60_D, Some(TOPOLOGY_ADOC), Some(&bad));
        assert!(
            p.iter()
                .any(|p| p.contains("grade mismatch") && p.contains("READINESS")),
            "{p:?}"
        );
    }

    #[test]
    fn prose_mentioning_overall_does_not_shadow_dashboard() {
        let topo = "\
// Last updated: 2026-07-02
> agreement is enforced if the OVERALL percentage drifts from STATE.
OVERALL:                            ██████░░░░  ~60%   Grade D
";
        assert_eq!(extract_overall_pct(topo), Some(60));
        assert!(problems_with(STATE_60_D, Some(topo), Some(READINESS_ADOC)).is_empty());
    }

    #[test]
    fn fails_on_stale_dashboard() {
        let stale = TOPOLOGY_ADOC.replace("2026-07-02", "2026-01-01");
        let p = problems_with(STATE_60_D, Some(&stale), Some(READINESS_ADOC));
        assert!(p.iter().any(|p| p.contains("staleness")), "{p:?}");
    }

    // A check that compares nothing must fail: one test per field that used
    // to be skipped silently when absent.

    #[test]
    fn missing_surfaces_fail() {
        let p = problems_with(STATE_60_D, None, None);
        assert!(p.iter().any(|p| p.contains("no TOPOLOGY")), "{p:?}");
        assert!(p.iter().any(|p| p.contains("no READINESS")), "{p:?}");
    }

    #[test]
    fn missing_last_updated_fails() {
        let undated: String = TOPOLOGY_ADOC
            .lines()
            .skip(1)
            .map(|l| format!("{l}\n"))
            .collect();
        let p = problems_with(STATE_60_D, Some(&undated), Some(READINESS_ADOC));
        assert!(p.iter().any(|p| p.contains("no 'Last updated")), "{p:?}");
    }

    #[test]
    fn missing_overall_fails() {
        let no_overall = TOPOLOGY_ADOC.replace("OVERALL:", "TOTAL:");
        let p = problems_with(STATE_60_D, Some(&no_overall), Some(READINESS_ADOC));
        assert!(
            p.iter().any(|p| p.contains("no parseable 'OVERALL")),
            "{p:?}"
        );
    }

    #[test]
    fn unparseable_readiness_grade_fails() {
        let p = problems_with(STATE_60_D, Some(TOPOLOGY_ADOC), Some("no grade here\n"));
        assert!(
            p.iter()
                .any(|p| p.contains("READINESS.adoc has no parseable grade")),
            "{p:?}"
        );
    }

    #[test]
    fn missing_state_fields_fail() {
        let p = problems_with("[metadata]\n", Some(TOPOLOGY_ADOC), Some(READINESS_ADOC));
        assert!(
            p.iter().any(|p| p.contains("no completion-percentage")),
            "{p:?}"
        );
        assert!(p.iter().any(|p| p.contains("no crg-grade")), "{p:?}");
        assert!(
            p.iter().any(|p| p.contains("no [metadata].last-updated")),
            "{p:?}"
        );
    }
}
