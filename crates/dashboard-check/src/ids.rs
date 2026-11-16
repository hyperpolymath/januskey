// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// Ledger row IDs: one registry, checked (owner decision 2026-11-16).
//
// PROOF-NEEDS.adoc has two tables. The status table (`| ID | Claim | Status |
// Artefact`) holds one row per obligation ID, and the claims ledger holds one
// row per check, each starting `ID: …`. The status table is the registry of
// IDs. Two sessions that each add "T-10" with a different meaning on parallel
// branches used to merge cleanly into two T-10 rows; these rules make that
// collision fail CI instead of landing.
//
// Rules, each of which has a fixture below:
// - Every status-table ID has the form `<LETTERS>-<digits>[<lowercase>]`
//   (`T-10`, `E-1m`) and appears exactly once.
// - Every claims-ledger row that starts with `ID:` names a registered ID.
// - Every registered ID has at least one claims-ledger row.
// - A registered ID's status is one of its ledger rows' statuses, so the two
//   tables cannot disagree about what an obligation has reached.
// Ledger rows with no `ID:` prefix are not checked here; the caller reports
// how many there are.

use crate::claims::Claim;
use std::collections::BTreeMap;

/// One row of the status table.
#[derive(Debug, Clone, PartialEq)]
pub struct Registered {
    pub line: usize,
    pub id: String,
    pub status: String,
}

const HEADER: &str = "| ID | Claim | Status | Artefact";

/// True for `T-10`, `E-1m`, `A-1`: capital letters, a hyphen, digits, then
/// optionally lowercase letters.
pub fn is_row_id(s: &str) -> bool {
    let Some((prefix, rest)) = s.split_once('-') else {
        return false;
    };
    let digits = rest.trim_end_matches(|c: char| c.is_ascii_lowercase());
    !prefix.is_empty()
        && prefix.chars().all(|c| c.is_ascii_uppercase())
        && !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
}

/// The ID a claims-ledger row starts with (`T-8: …`), if any.
pub fn claim_id(claim: &str) -> Option<&str> {
    let (head, _) = claim.split_once(':')?;
    let head = head.trim();
    is_row_id(head).then_some(head)
}

/// A head before `:` that looks like an ID but is not one (`T5`, `T-5M`): one
/// word, starting with a capital letter, containing a digit. A typo like this
/// must not pass as a row with no ID.
pub fn malformed_id(claim: &str) -> Option<&str> {
    let (head, _) = claim.split_once(':')?;
    let head = head.trim();
    let looks_like_id = !head.is_empty()
        && !head.contains(char::is_whitespace)
        && head.starts_with(|c: char| c.is_ascii_uppercase())
        && head.contains(|c: char| c.is_ascii_digit());
    (looks_like_id && !is_row_id(head)).then_some(head)
}

/// Parse the status table, reading it the way AsciiDoc does, so that the
/// registry this check sees is the table a reader sees: the line just above
/// the header is the table's opening `|===`; every row is one line that starts
/// (untrimmed) with exactly one `|` and has exactly four cells; and the table
/// is closed by `|===`. Blank lines inside the table are allowed. A missing
/// table is an error, not an empty registry.
pub fn parse_status_table(src: &str) -> Result<Vec<Registered>, Vec<String>> {
    let lines: Vec<&str> = src.lines().collect();
    let Some(header) = lines.iter().position(|l| l.trim_end() == HEADER) else {
        return Err(vec![format!(
            "PROOF-NEEDS.adoc has no status table (header '{HEADER}')"
        )]);
    };
    if header == 0 || lines[header - 1].trim_end() != "|===" {
        return Err(vec![format!(
            "PROOF-NEEDS.adoc:{}: the status table header must directly follow the table's opening |===",
            header + 1
        )]);
    }
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    let mut closed = false;
    for (i, raw) in lines.iter().enumerate().skip(header + 1) {
        let n = i + 1;
        // Only trailing whitespace is ignored: an indented line is not a table
        // row to AsciiDoc.
        let l = raw.trim_end();
        if l == "|===" {
            closed = true;
            break;
        }
        if l.is_empty() {
            continue;
        }
        // Exactly one leading delimiter: AsciiDoc reads a row without one as
        // continuation text and `||` as an empty first cell, so either would
        // let the registry diverge from the table a reader sees.
        let Some(body) = l.strip_prefix('|').filter(|b| !b.starts_with('|')) else {
            errors.push(format!(
                "PROOF-NEEDS.adoc:{n}: status-table rows must start with exactly one '|'"
            ));
            continue;
        };
        let cells: Vec<&str> = body.split('|').map(str::trim).collect();
        if cells.len() != 4 {
            errors.push(format!(
                "PROOF-NEEDS.adoc:{n}: status-table rows need exactly four cells on one line (ID | Claim | Status | Artefact), found {}",
                cells.len()
            ));
            continue;
        }
        if !is_row_id(cells[0]) {
            errors.push(format!(
                "PROOF-NEEDS.adoc:{n}: '{}' is not a row ID (expected e.g. T-10 or E-1m)",
                cells[0]
            ));
            continue;
        }
        rows.push(Registered {
            line: n,
            id: cells[0].to_string(),
            status: cells[2].to_string(),
        });
    }
    if !closed {
        errors.push("the status table is not closed with |===".into());
    }
    if errors.is_empty() {
        Ok(rows)
    } else {
        Err(errors)
    }
}

/// Check the registry against the claims ledger. Returns the problems found
/// and the number of ledger rows that carry no ID.
pub fn check_ids(registry: &[Registered], claims: &[Claim]) -> (Vec<String>, usize) {
    let mut problems = Vec::new();

    let mut first: BTreeMap<&str, usize> = BTreeMap::new();
    for r in registry {
        if let Some(prev) = first.insert(r.id.as_str(), r.line) {
            problems.push(format!(
                "PROOF-NEEDS.adoc:{}: row ID {} is registered twice (also line {prev}); \
                 give one of them a new ID",
                r.line, r.id
            ));
        }
    }

    let mut statuses: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut unprefixed = 0;
    for c in claims {
        match claim_id(&c.claim) {
            Some(id) if first.contains_key(id) => statuses
                .entry(id)
                .or_default()
                .push(format!("{:?}", c.status).to_uppercase()),
            Some(id) => problems.push(format!(
                "PROOF-NEEDS.adoc:{}: ledger row names {id}, which the status table does not register",
                c.line
            )),
            None => match malformed_id(&c.claim) {
                Some(head) => problems.push(format!(
                    "PROOF-NEEDS.adoc:{}: ledger row starts with '{head}:', which is not a row ID (expected e.g. T-10 or E-1m)",
                    c.line
                )),
                None => unprefixed += 1,
            },
        }
    }

    for r in registry {
        match statuses.get(r.id.as_str()) {
            None => problems.push(format!(
                "PROOF-NEEDS.adoc:{}: {} is registered but has no claims-ledger row",
                r.line, r.id
            )),
            Some(s) if !s.contains(&r.status) => problems.push(format!(
                "PROOF-NEEDS.adoc:{}: {} is {} in the status table but its ledger rows are {}",
                r.line,
                r.id,
                r.status,
                s.join(", ")
            )),
            Some(_) => {}
        }
    }
    (problems, unprefixed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claims::parse_ledger;

    const DOC: &str = "\
[cols=\"1,3,1,2\",options=\"header\"]
|===
| ID | Claim | Status | Artefact
| T-1 | Rollback restores | PROVEN | `x`
| E-1m | Model ignorance | OPEN | `y`
|===

=== Claims ledger

|===
|Claim |Status |Artefact |Command
|T-1: rollback restores |PROVEN |`a.agda::r` |`true`
|E-1m: ignorance |OPEN |`#1` |-
|An unregistered property test |TESTED |`src/a.rs::t` |`cargo test t`
|===
";

    fn run(doc: &str) -> (Vec<String>, usize) {
        let reg = parse_status_table(doc).unwrap();
        let claims = parse_ledger(doc).unwrap();
        check_ids(&reg, &claims)
    }

    #[test]
    fn row_id_shape() {
        for ok in ["T-1", "T-10", "E-1m", "A-12"] {
            assert!(is_row_id(ok), "{ok}");
        }
        for bad in ["t-1", "T1", "T-", "-1", "T-1M", "T-x"] {
            assert!(!is_row_id(bad), "{bad}");
        }
        assert_eq!(claim_id("T-8: a log edit"), Some("T-8"));
        assert_eq!(claim_id("Execute then undo: holds"), None);
    }

    #[test]
    fn a_consistent_ledger_passes_and_counts_unprefixed_rows() {
        let (p, unprefixed) = run(DOC);
        assert!(p.is_empty(), "{p:?}");
        assert_eq!(unprefixed, 1);
    }

    #[test]
    fn an_id_registered_twice_is_rejected() {
        let doc = DOC.replace(
            "| E-1m | Model ignorance | OPEN | `y`",
            "| E-1m | Model ignorance | OPEN | `y`\n| T-1 | Something else | OPEN | `z`",
        );
        let (p, _) = run(&doc);
        assert!(
            p.iter().any(|e| e.contains("T-1 is registered twice")),
            "{p:?}"
        );
    }

    #[test]
    fn a_ledger_row_with_an_unregistered_id_is_rejected() {
        let doc = DOC.replace("|T-1: rollback", "|T-9: rollback");
        let (p, _) = run(&doc);
        assert!(p.iter().any(|e| e.contains("names T-9")), "{p:?}");
        assert!(
            p.iter().any(|e| e.contains("T-1 is registered but has no")),
            "{p:?}"
        );
    }

    #[test]
    fn disagreeing_statuses_are_rejected() {
        let doc = DOC.replace(
            "| T-1 | Rollback restores | PROVEN",
            "| T-1 | Rollback restores | TESTED",
        );
        let (p, _) = run(&doc);
        assert!(
            p.iter()
                .any(|e| e
                    .contains("T-1 is TESTED in the status table but its ledger rows are PROVEN")),
            "{p:?}"
        );
    }

    #[test]
    fn a_typoed_ledger_prefix_is_rejected_not_counted_as_unprefixed() {
        for typo in ["T5", "T-5M", "T-1X"] {
            let doc = DOC.replace(
                "|An unregistered property test",
                &format!("|{typo}: a typo"),
            );
            let (p, unprefixed) = run(&doc);
            assert!(
                p.iter()
                    .any(|e| e.contains(&format!("'{typo}:', which is not a row ID"))),
                "{typo}: {p:?}"
            );
            assert_eq!(unprefixed, 0, "{typo}");
        }
        assert_eq!(malformed_id("Execute then undo: holds"), None);
        assert_eq!(malformed_id("src/abi uses no proof bypass: x"), None);
    }

    #[test]
    fn a_registry_row_without_exactly_one_leading_bar_is_rejected() {
        for bad in [
            "T-1 | Rollback restores | PROVEN | `x`",
            "|| T-1 | Rollback restores | PROVEN | `x`",
        ] {
            let doc = DOC.replace("| T-1 | Rollback restores | PROVEN | `x`", bad);
            let e = parse_status_table(&doc).unwrap_err();
            assert!(
                e.iter().any(|e| e.contains("exactly one '|'")),
                "{bad}: {e:?}"
            );
        }
    }

    #[test]
    fn the_table_must_be_read_as_asciidoc_reads_it() {
        let base = "| T-1 | Rollback restores | PROVEN | `x`";
        // An indented row, a row with three cells, a row with five.
        for (bad, msg) in [
            (
                " | T-1 | Rollback restores | PROVEN | `x`",
                "exactly one '|'",
            ),
            ("| T-1 | Rollback restores | PROVEN", "found 3"),
            (
                "| T-1 | Rollback restores | PROVEN | `x` | extra",
                "found 5",
            ),
        ] {
            let e = parse_status_table(&DOC.replace(base, bad)).unwrap_err();
            assert!(e.iter().any(|e| e.contains(msg)), "{bad}: {e:?}");
        }
        // The opening |=== displaced from just above the header.
        let displaced = DOC.replacen("|===\n| ID |", "|===\n\n| ID |", 1);
        assert!(parse_status_table(&displaced).unwrap_err()[0].contains("directly follow"));
        let missing = DOC.replacen("|===\n| ID |", "| ID |", 1);
        assert!(parse_status_table(&missing).unwrap_err()[0].contains("directly follow"));
    }

    #[test]
    fn a_malformed_id_or_missing_table_is_an_error() {
        let doc = DOC.replace("| T-1 | Rollback", "| T1 | Rollback");
        assert!(parse_status_table(&doc).unwrap_err()[0].contains("'T1' is not a row ID"));
        assert!(parse_status_table("= nothing\n").is_err());
    }
}
