use super::*;
use crate::knowledge::analyzer::RiskLevel;
use std::path::PathBuf;

fn sample_files() -> Vec<FileOwnership> {
    vec![
        FileOwnership {
            path: PathBuf::from("src/foo.rs"),
            language: "Rust".to_string(),
            total_lines: 731,
            primary_owner: "Alice".to_string(),
            ownership_pct: 94.0,
            contributors: 2,
            risk: RiskLevel::Critical,
            knowledge_loss: true,
        },
        FileOwnership {
            path: PathBuf::from("src/bar.rs"),
            language: "Rust".to_string(),
            total_lines: 241,
            primary_owner: "Bob".to_string(),
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
