use super::*;
use crate::knowledge::analyzer::{AuthorSummary, RiskLevel};
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
