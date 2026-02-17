use std::path::PathBuf;

use super::*;
use crate::score::analyzer::{DimensionScore, FileScore};

fn sample_score() -> ProjectScore {
    ProjectScore {
        score: 84.3,
        grade: Grade::BPlus,
        files_analyzed: 42,
        total_loc: 8432,
        dimensions: vec![
            DimensionScore {
                name: "Maintainability Index",
                weight: 0.25,
                score: 88.2,
                grade: Grade::AMinus,
            },
            DimensionScore {
                name: "Cyclomatic Complexity",
                weight: 0.20,
                score: 82.4,
                grade: Grade::BPlus,
            },
        ],
        needs_attention: vec![FileScore {
            path: PathBuf::from("src/legacy/parser.rs"),
            score: 54.2,
            grade: Grade::F,
            loc: 500,
            issues: vec!["Complexity: 87".to_string(), "MI: 12".to_string()],
        }],
    }
}

fn empty_score() -> ProjectScore {
    ProjectScore {
        score: 0.0,
        grade: Grade::FMinusMinus,
        files_analyzed: 0,
        total_loc: 0,
        dimensions: vec![],
        needs_attention: vec![],
    }
}

#[test]
fn print_report_does_not_panic() {
    print_report(&sample_score(), 10, None);
}

#[test]
fn print_report_empty() {
    print_report(&empty_score(), 10, None);
}

#[test]
fn print_report_with_target_dir() {
    print_report(&sample_score(), 10, Some("src/"));
}

#[test]
fn print_report_with_target_file() {
    let mut score = sample_score();
    score.files_analyzed = 1;
    print_report(&score, 10, Some("src/main.rs"));
}

#[test]
fn print_json_does_not_panic() {
    print_json(&sample_score(), None).unwrap();
}

#[test]
fn print_json_empty() {
    print_json(&empty_score(), None).unwrap();
}

#[test]
fn print_json_with_target() {
    print_json(&sample_score(), Some("src/")).unwrap();
}

#[test]
fn format_thousands_works() {
    assert_eq!(format_thousands(0), "0");
    assert_eq!(format_thousands(999), "999");
    assert_eq!(format_thousands(1000), "1,000");
    assert_eq!(format_thousands(1234567), "1,234,567");
}
