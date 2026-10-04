use super::*;
use crate::deps::analyzer::{DepEntry, DepResult};
use std::path::PathBuf;

fn make_entry(
    path: &str,
    language: &str,
    fan_in: usize,
    fan_out: usize,
    in_cycle: bool,
) -> DepEntry {
    DepEntry {
        path: PathBuf::from(path),
        language: language.to_string(),
        fan_in,
        fan_out,
        in_cycle,
    }
}

fn make_result(entries: Vec<DepEntry>, cycles: Vec<Vec<PathBuf>>) -> DepResult {
    DepResult {
        entries,
        cycles,
        unsupported: vec![],
    }
}

// ── print_report ────────────────────────────────────────────────────────────

#[test]
fn print_report_empty_entries() {
    let result = make_result(vec![], vec![]);
    print_report(&[], &result);
}

#[test]
fn print_report_no_cycles() {
    let entries = vec![
        make_entry("src/main.rs", "Rust", 0, 2, false),
        make_entry("src/lib.rs", "Rust", 2, 0, false),
    ];
    let result = make_result(
        entries
            .iter()
            .map(|e| {
                make_entry(
                    &e.path.display().to_string(),
                    &e.language,
                    e.fan_in,
                    e.fan_out,
                    e.in_cycle,
                )
            })
            .collect(),
        vec![],
    );
    print_report(&entries, &result);
}

#[test]
fn print_report_with_cycles() {
    let entries = vec![
        make_entry("src/a.rs", "Rust", 1, 1, true),
        make_entry("src/b.rs", "Rust", 1, 1, true),
    ];
    let cycle = vec![PathBuf::from("src/a.rs"), PathBuf::from("src/b.rs")];
    let result = make_result(
        entries
            .iter()
            .map(|e| {
                make_entry(
                    &e.path.display().to_string(),
                    &e.language,
                    e.fan_in,
                    e.fan_out,
                    e.in_cycle,
                )
            })
            .collect(),
        vec![cycle],
    );
    print_report(&entries, &result);
}

#[test]
fn print_report_mixed_cycle_and_clean() {
    let entries = vec![
        make_entry("src/clean.rs", "Rust", 0, 1, false),
        make_entry("src/cycle_a.rs", "Rust", 1, 1, true),
        make_entry("src/cycle_b.rs", "Rust", 1, 1, true),
    ];
    let cycle = vec![
        PathBuf::from("src/cycle_a.rs"),
        PathBuf::from("src/cycle_b.rs"),
    ];
    let result = make_result(
        entries
            .iter()
            .map(|e| {
                make_entry(
                    &e.path.display().to_string(),
                    &e.language,
                    e.fan_in,
                    e.fan_out,
                    e.in_cycle,
                )
            })
            .collect(),
        vec![cycle],
    );
    print_report(&entries, &result);
}

// ── print_json ───────────────────────────────────────────────────────────────

#[test]
fn print_json_empty() {
    let result = make_result(vec![], vec![]);
    print_json(&result).unwrap();
}

#[test]
fn print_json_with_entries_no_cycles() {
    let entries = vec![
        make_entry("src/main.rs", "Rust", 0, 3, false),
        make_entry("src/util.rs", "Rust", 3, 0, false),
    ];
    let result = make_result(entries, vec![]);
    print_json(&result).unwrap();
}

#[test]
fn print_json_with_cycles() {
    let entries = vec![
        make_entry("src/a.rs", "Rust", 1, 1, true),
        make_entry("src/b.rs", "Rust", 1, 1, true),
    ];
    let cycle = vec![PathBuf::from("src/a.rs"), PathBuf::from("src/b.rs")];
    let result = make_result(entries, vec![cycle]);
    print_json(&result).unwrap();
}

// ── unsupported languages ────────────────────────────────────────────────────

fn unsupported(items: &[(&str, usize)]) -> Vec<UnsupportedLanguage> {
    items
        .iter()
        .map(|(language, files)| UnsupportedLanguage {
            language: language.to_string(),
            files: *files,
        })
        .collect()
}

#[test]
fn unsupported_note_is_absent_when_everything_was_analysed() {
    assert_eq!(unsupported_note(&[]), None);
}

#[test]
fn unsupported_note_lists_languages_and_file_counts() {
    let note = unsupported_note(&unsupported(&[("Bourne Shell", 3), ("TOML", 1)]));
    assert_eq!(
        note.as_deref(),
        Some("Not analysed (unsupported language): Bourne Shell 3, TOML 1")
    );
}

#[test]
fn print_report_with_unsupported_languages() {
    let entries = vec![make_entry("src/main.rs", "Rust", 0, 0, false)];
    let mut result = make_result(entries.clone(), vec![]);
    result.unsupported = unsupported(&[("TOML", 1)]);
    print_report(&entries, &result);
    print_report(&[], &result);
    print_short(&result);
}

#[test]
fn json_carries_unsupported_languages() {
    let mut result = make_result(vec![], vec![]);
    result.unsupported = unsupported(&[("TOML", 2)]);
    let json = serde_json::to_value(JsonDepResult::from(&result)).unwrap();
    assert_eq!(json["unsupported"][0]["language"], "TOML");
    assert_eq!(json["unsupported"][0]["files"], 2);
}

// ── render_report ────────────────────────────────────────────────────────────

fn separator_line(width: usize) -> String {
    "\u{2500}".repeat(width)
}

#[test]
fn render_report_without_entries_says_so() {
    let result = make_result(vec![], vec![]);
    assert_eq!(
        render_report(&[], &result),
        "No source files found for dependency analysis.\n"
    );
}

#[test]
fn render_report_lays_out_the_table() {
    let entries = vec![
        make_entry("a.rs", "Rust", 12, 3, false),
        make_entry("b", "Go", 0, 1, true),
    ];
    let result = make_result(entries.clone(), vec![]);
    let sep = separator_line(72);
    let expected = [
        "Dependency Graph",
        &sep,
        " File    Language Fan-In Fan-Out Cycle",
        &sep,
        " a.rs        Rust     12       3    no",
        " b             Go      0       1   yes",
        &sep,
        "No dependency cycles detected.",
        "",
    ]
    .join("\n");
    assert_eq!(render_report(&entries, &result), expected);
}

#[test]
fn render_report_separator_grows_with_the_longest_path() {
    let path = format!("src/{}.rs", "x".repeat(50));
    let entries = vec![make_entry(&path, "Rust", 0, 0, false)];
    let result = make_result(entries.clone(), vec![]);
    let report = render_report(&entries, &result);
    let separator = report.lines().nth(1).unwrap();
    // The path column, the fixed columns and one trailing column of margin.
    assert_eq!(separator.chars().count(), path.len() + 35);
}

#[test]
fn render_report_lists_cycles() {
    let entries = vec![
        make_entry("src/a.rs", "Rust", 1, 1, true),
        make_entry("src/b.rs", "Rust", 1, 1, true),
    ];
    let cycle = vec![PathBuf::from("src/a.rs"), PathBuf::from("src/b.rs")];
    let result = make_result(entries.clone(), vec![cycle]);
    let report = render_report(&entries, &result);
    let tail: Vec<&str> = report.lines().skip(7).collect();
    assert_eq!(
        tail,
        vec![
            "",
            "Dependency cycles: 1",
            "  Cycle 1 (2 files):",
            "    src/a.rs",
            "    src/b.rs",
        ]
    );
    assert!(!report.contains("No dependency cycles detected."));
}

#[test]
fn render_report_ends_with_the_unsupported_note() {
    let entries = vec![make_entry("src/main.rs", "Rust", 0, 0, false)];
    let mut result = make_result(entries.clone(), vec![]);
    result.unsupported = unsupported(&[("TOML", 1)]);
    let note = "Not analysed (unsupported language): TOML 1\n";
    assert!(render_report(&entries, &result).ends_with(note));
    assert_eq!(
        render_report(&[], &result),
        format!("No source files found for dependency analysis.\n{note}")
    );
}
