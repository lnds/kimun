use super::*;
use crate::knowledge::analyzer::{AuthorSummary, BusFactor, BusFactorEntry, RiskLevel};
use std::path::PathBuf;

fn sample_files() -> Vec<FileOwnership> {
    vec![
        FileOwnership {
            path: PathBuf::from("src/foo.rs"),
            language: "Rust".to_string(),
            total_lines: 731,
            primary_owner: "Lautaro".to_string(),
            primary_email: "lautaro@ruca.mapu".to_string(),
            ownership_pct: 94.0,
            contributors: 2,
            risk: RiskLevel::Critical,
            knowledge_loss: true,
        },
        FileOwnership {
            path: PathBuf::from("src/bar.rs"),
            language: "Rust".to_string(),
            total_lines: 241,
            primary_owner: "Caupolicán".to_string(),
            primary_email: "caupolican@ruca.mapu".to_string(),
            ownership_pct: 78.0,
            contributors: 3,
            risk: RiskLevel::High,
            knowledge_loss: false,
        },
    ]
}

#[test]
fn print_report_does_not_panic() {
    print_report(&sample_files());
}

#[test]
fn print_report_empty() {
    print_report(&[]);
}

#[test]
fn print_json_does_not_panic() {
    print_json(&sample_files()).unwrap();
}

#[test]
fn print_json_empty() {
    print_json(&[]).unwrap();
}

// ── print_summary_report ───────────────────────────────────────────────

fn sample_authors() -> Vec<AuthorSummary> {
    vec![
        AuthorSummary {
            author: "Alice".to_string(),
            files_owned: 5,
            total_lines: 1000,
            languages: vec!["Rust".to_string()],
            worst_risk: RiskLevel::Critical,
            knowledge_loss_files: 2,
        },
        AuthorSummary {
            author: "Bob".to_string(),
            files_owned: 3,
            total_lines: 500,
            languages: vec!["Python".to_string(), "Rust".to_string()],
            worst_risk: RiskLevel::Medium,
            knowledge_loss_files: 0,
        },
    ]
}

#[test]
fn print_summary_report_empty() {
    print_summary_report(&[]);
}

#[test]
fn print_summary_report_does_not_panic() {
    print_summary_report(&sample_authors());
}

#[test]
fn print_summary_json_empty() {
    print_summary_json(&[]).unwrap();
}

#[test]
fn print_summary_json_does_not_panic() {
    print_summary_json(&sample_authors()).unwrap();
}

// ── print_bus_factor_report ────────────────────────────────────────────────

fn make_bus_factor(
    factor: usize,
    threshold: f64,
    total_lines: usize,
    contributors: Vec<BusFactorEntry>,
) -> BusFactor {
    BusFactor {
        factor,
        threshold,
        total_lines,
        contributors,
    }
}

fn make_entry(
    author: &str,
    lines: usize,
    pct: f64,
    cumulative_pct: f64,
    is_critical: bool,
) -> BusFactorEntry {
    BusFactorEntry {
        author: author.to_string(),
        lines,
        pct,
        cumulative_pct,
        is_critical,
    }
}

#[test]
fn print_bus_factor_report_no_data() {
    let bf = make_bus_factor(0, 80.0, 0, vec![]);
    print_bus_factor_report(&bf);
}

#[test]
fn print_bus_factor_critical_single() {
    let bf = make_bus_factor(
        1,
        80.0,
        1000,
        vec![
            make_entry("Alice", 900, 90.0, 90.0, true),
            make_entry("Bob", 100, 10.0, 100.0, false),
        ],
    );
    print_bus_factor_report(&bf);
}

#[test]
fn print_bus_factor_high_two() {
    let bf = make_bus_factor(
        2,
        80.0,
        1000,
        vec![
            make_entry("Alice", 500, 50.0, 50.0, true),
            make_entry("Bob", 350, 35.0, 85.0, true),
            make_entry("Carol", 150, 15.0, 100.0, false),
        ],
    );
    print_bus_factor_report(&bf);
}

#[test]
fn print_bus_factor_low_distributed() {
    let bf = make_bus_factor(
        4,
        80.0,
        1000,
        vec![
            make_entry("Alice", 250, 25.0, 25.0, true),
            make_entry("Bob", 220, 22.0, 47.0, true),
            make_entry("Carol", 200, 20.0, 67.0, true),
            make_entry("Dave", 180, 18.0, 85.0, true),
            make_entry("Eve", 150, 15.0, 100.0, false),
        ],
    );
    print_bus_factor_report(&bf);
}

#[test]
fn print_bus_factor_factor_3() {
    let bf = make_bus_factor(
        3,
        80.0,
        300,
        vec![
            make_entry("X", 130, 43.3, 43.3, true),
            make_entry("Y", 110, 36.7, 80.0, true),
            make_entry("Z", 60, 20.0, 100.0, true),
        ],
    );
    print_bus_factor_report(&bf);
}

#[test]
fn print_bus_factor_single_contributor() {
    // factor = 1, single person "contributor" (singular)
    let bf = make_bus_factor(
        1,
        80.0,
        500,
        vec![make_entry("Solo", 500, 100.0, 100.0, true)],
    );
    print_bus_factor_report(&bf);
}

// ── print_bus_factor_json ──────────────────────────────────────────────────

#[test]
fn print_bus_factor_json_no_data() {
    let bf = make_bus_factor(0, 80.0, 0, vec![]);
    print_bus_factor_json(&bf).unwrap();
}

#[test]
fn print_bus_factor_json_with_data() {
    let bf = make_bus_factor(
        1,
        80.0,
        1000,
        vec![
            make_entry("Alice", 900, 90.0, 90.0, true),
            make_entry("Bob", 100, 10.0, 100.0, false),
        ],
    );
    print_bus_factor_json(&bf).unwrap();
}
