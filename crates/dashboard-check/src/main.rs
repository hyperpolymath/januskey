// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// dashboard-check — reconcile hand-maintained status dashboards against the
// machine-readable STATE.a2ml (the declared source of truth).
//
// Motivation: the estate's #1 recurring defect is "dashboards that lie" — a
// human-facing surface (TOPOLOGY.md completion bar, README badge, READINESS
// grade) that claims more than STATE.a2ml records. This tool reads STATE and
// asserts the dashboards agree; run in CI it makes divergence a build failure,
// so the lie cannot be committed silently.
//
// `.a2ml` is TOML (the estate parses it with a TOML parser elsewhere), so we
// parse it with the `toml` crate rather than a bespoke reader.
//
// Usage:
//     dashboard-check [--check] [REPO_ROOT]
// Exits 0 if the dashboards match STATE (or the surfaces are absent), non-zero
// with a diff report otherwise. `--check` is the default and only mode today.

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

    let last_updated = get("metadata", "last-updated")
        .and_then(|v| v.as_str().map(str::to_string));

    Ok(StateFacts { completion, grade, last_updated })
}

/// Coerce a TOML value (string `"60"` or integer `60`) into a percentage.
fn value_to_u32(v: &toml::Value) -> Option<u32> {
    match v {
        toml::Value::Integer(i) => u32::try_from(*i).ok(),
        toml::Value::String(s) => s.trim().trim_end_matches('%').parse().ok(),
        _ => None,
    }
}

/// The `OVERALL: ... ~60%` figure inside the TOPOLOGY completion dashboard.
/// Scans every line containing "OVERALL" and returns the first that carries a
/// `<digits>%` figure — so prose that merely mentions "OVERALL" (e.g. the
/// source-of-truth note) does not shadow the real dashboard line.
fn extract_overall_pct(topology: &str) -> Option<u32> {
    topology
        .lines()
        .filter(|l| l.contains("OVERALL"))
        .find_map(first_percent)
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

/// The grade letter after a `Grade ` token (e.g. "Grade D — Alpha"), used for
/// the TOPOLOGY dashboard line.
fn extract_grade_after_token(text: &str, token: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(idx) = line.find(token) {
            let rest = line[idx + token.len()..].trim_start();
            let g: String = rest.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
            if !g.is_empty() {
                return Some(g);
            }
        }
    }
    None
}

/// READINESS grade: `**Current Grade:** D` or `## CRG Grade: D (...)`.
fn extract_grade_readiness(readiness: &str) -> Option<String> {
    extract_grade_after_token(readiness, "Current Grade:** ")
        .or_else(|| extract_grade_after_token(readiness, "CRG Grade: "))
}

/// The `<!-- Last updated: YYYY-MM-DD ... -->` date from TOPOLOGY.
fn extract_last_updated(topology: &str) -> Option<String> {
    let line = topology.lines().find(|l| l.contains("Last updated:"))?;
    let idx = line.find("Last updated:")? + "Last updated:".len();
    let rest = line[idx..].trim_start();
    let date: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    if date.len() >= 8 { Some(date) } else { None }
}

/// Compare STATE against the dashboards; return a list of human-readable
/// mismatch messages (empty = all good). Pure so it is unit-testable.
fn reconcile(
    state: &StateFacts,
    topology: Option<&str>,
    readiness: Option<&str>,
) -> Vec<String> {
    let mut problems = Vec::new();

    if let (Some(pct), Some(topo)) = (state.completion, topology) {
        match extract_overall_pct(topo) {
            Some(dpct) if dpct != pct => problems.push(format!(
                "completion mismatch: STATE says {pct}% but TOPOLOGY.md OVERALL says {dpct}%"
            )),
            None => problems.push(
                "TOPOLOGY.md has no parseable 'OVERALL: …%' line to check against STATE".into(),
            ),
            _ => {}
        }
    }

    if let Some(grade) = &state.grade {
        if let Some(topo) = topology {
            if let Some(g) = extract_grade_after_token(topo, "Grade ") {
                if &g != grade {
                    problems.push(format!(
                        "grade mismatch: STATE says {grade} but TOPOLOGY.md says Grade {g}"
                    ));
                }
            }
        }
        if let Some(read) = readiness {
            match extract_grade_readiness(read) {
                Some(g) if &g != grade => problems.push(format!(
                    "grade mismatch: STATE says {grade} but READINESS.md says Grade {g}"
                )),
                _ => {}
            }
        }
    }

    if let (Some(su), Some(topo)) = (&state.last_updated, topology) {
        if let Some(du) = extract_last_updated(topo) {
            // Lexicographic compare works for ISO YYYY-MM-DD dates.
            if du.as_str() < su.as_str() {
                problems.push(format!(
                    "staleness: TOPOLOGY.md 'Last updated: {du}' predates STATE last-updated {su}"
                ));
            }
        }
    }

    problems
}

fn read_opt(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn main() -> ExitCode {
    // Skip the binary name; ignore the `--check` flag (default mode).
    let mut root = PathBuf::from(".");
    for arg in std::env::args().skip(1) {
        if arg == "--check" {
            continue;
        }
        root = PathBuf::from(arg);
    }

    let state_path = root.join(".machine_readable/6a2/STATE.a2ml");
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

    let topology = read_opt(&root.join("TOPOLOGY.md"));
    let readiness = read_opt(&root.join("READINESS.md"));

    let problems = reconcile(&state, topology.as_deref(), readiness.as_deref());

    if problems.is_empty() {
        println!(
            "dashboard-check: OK — dashboards agree with STATE.a2ml (completion={:?}, grade={:?})",
            state.completion, state.grade
        );
        ExitCode::SUCCESS
    } else {
        eprintln!("dashboard-check: {} divergence(s) from STATE.a2ml (the source of truth):", problems.len());
        for p in &problems {
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
[project-context]
completion-percentage = 50
"#;

    const TOPOLOGY_60_D: &str = "\
<!-- Last updated: 2026-07-02 -->
OVERALL:                            ██████░░░░  ~60%   Grade D — Alpha, Unstable
";

    const READINESS_D: &str = "\
**Current Grade:** D
## CRG Grade: D (Alpha — Unstable)
";

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
    fn extracts_dashboard_signals() {
        assert_eq!(extract_overall_pct(TOPOLOGY_60_D), Some(60));
        assert_eq!(extract_grade_after_token(TOPOLOGY_60_D, "Grade ").as_deref(), Some("D"));
        assert_eq!(extract_grade_readiness(READINESS_D).as_deref(), Some("D"));
        assert_eq!(extract_last_updated(TOPOLOGY_60_D).as_deref(), Some("2026-07-02"));
    }

    #[test]
    fn passes_when_aligned() {
        let s = extract_state(STATE_60_D).unwrap();
        let problems = reconcile(&s, Some(TOPOLOGY_60_D), Some(READINESS_D));
        assert!(problems.is_empty(), "expected no problems, got {problems:?}");
    }

    #[test]
    fn fails_on_completion_drift() {
        // The historical disease: STATE 50, dashboard 60.
        let s = extract_state(STATE_50_BARE).unwrap();
        let problems = reconcile(&s, Some(TOPOLOGY_60_D), None);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("completion mismatch"), "{problems:?}");
    }

    #[test]
    fn fails_on_grade_drift() {
        let s = extract_state(STATE_60_D).unwrap();
        let bad_topology = "OVERALL: ~60% Grade A — Production Ready\n";
        let problems = reconcile(&s, Some(bad_topology), None);
        assert!(problems.iter().any(|p| p.contains("grade mismatch")), "{problems:?}");
    }

    #[test]
    fn prose_mentioning_overall_does_not_shadow_dashboard() {
        // A source-of-truth note that mentions "OVERALL" in prose (no %) must
        // not be picked instead of the real "OVERALL: ~60%" dashboard line.
        let topo = "\
> agreement is enforced if the OVERALL percentage drifts from STATE.
OVERALL:                            ██████░░░░  ~60%   Grade D
";
        assert_eq!(extract_overall_pct(topo), Some(60));
        let s = extract_state(STATE_60_D).unwrap();
        assert!(reconcile(&s, Some(topo), None).is_empty());
    }

    #[test]
    fn fails_on_stale_dashboard() {
        let s = extract_state(STATE_60_D).unwrap();
        let stale = "\
<!-- Last updated: 2026-01-01 -->
OVERALL: ~60% Grade D
";
        let problems = reconcile(&s, Some(stale), None);
        assert!(problems.iter().any(|p| p.contains("staleness")), "{problems:?}");
    }
}
