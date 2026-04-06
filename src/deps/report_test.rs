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
    DepResult { entries, cycles }
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
