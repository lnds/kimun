use super::*;
use crate::tc::analyzer::CouplingLevel;
use std::path::PathBuf;

fn sample_pairs() -> Vec<FileCoupling> {
    vec![
        FileCoupling {
            file_a: PathBuf::from("src/auth/jwt.rs"),
            file_b: PathBuf::from("src/auth/middleware.rs"),
            shared_commits: 12,
            commits_a: 14,
            commits_b: 14,
            strength: 0.86,
            level: CouplingLevel::Strong,
        },
        FileCoupling {
            file_a: PathBuf::from("lib/parser.rs"),
            file_b: PathBuf::from("lib/validator.rs"),
            shared_commits: 8,
            commits_a: 15,
            commits_b: 10,
            strength: 0.53,
            level: CouplingLevel::Strong,
        },
    ]
}

#[test]
fn print_report_does_not_panic() {
    print_report(&sample_pairs(), 10);
}

#[test]
fn print_report_empty() {
    print_report(&[], 0);
}

#[test]
fn print_json_does_not_panic() {
    print_json(&sample_pairs()).unwrap();
}

#[test]
fn print_json_empty() {
    print_json(&[]).unwrap();
}
