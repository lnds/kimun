use std::path::PathBuf;

use super::*;
use crate::score::analyzer::{DimensionScore, FileScore, Graded};

fn sample_score() -> ProjectScore {
    ProjectScore {
        graded: Some(Graded {
            score: 84.3,
            grade: Grade::BPlus,
        }),
        files_analyzed: 42,
        total_loc: 8432,
        dimensions: vec![
            DimensionScore {
                name: "Cognitive Complexity",
                weight: 0.30,
                graded: Some(Graded {
                    score: 88.2,
                    grade: Grade::AMinus,
                }),
            },
            DimensionScore {
                name: "Duplication",
                weight: 0.20,
                graded: Some(Graded {
                    score: 82.4,
                    grade: Grade::BPlus,
                }),
            },
        ],
        needs_attention: vec![FileScore {
            path: PathBuf::from("src/legacy/parser.rs"),
            score: 54.2,
            grade: Grade::F,
            loc: 500,
            issues: vec!["Cognitive: 87".to_string()],
        }],
    }
}

fn empty_score() -> ProjectScore {
    ProjectScore {
        graded: None,
        files_analyzed: 0,
        total_loc: 0,
        dimensions: vec![],
        needs_attention: vec![],
    }
}

/// A target with nothing to grade, as `snapshot_of` builds it: the
/// dimensions keep their names and weights, and have no score.
fn nothing_to_grade() -> ProjectScore {
    let mut score = sample_score();
    score.graded = None;
    score.files_analyzed = 0;
    score.total_loc = 0;
    score.needs_attention = vec![];
    for d in &mut score.dimensions {
        d.graded = None;
    }
    score
}

#[test]
fn nothing_to_grade_is_shown_as_a_dash() {
    let score = nothing_to_grade();
    assert_eq!(
        project_line(&score),
        " Project Score:  - (nothing to grade)"
    );
    assert_eq!(
        dimension_row(&score.dimensions[0]),
        " Cognitive Complexity         30%       -   -    "
    );
    assert_eq!(
        short(&score),
        "score s:- g:- files:0 loc:0 cognitive:- duplication:-"
    );
}

#[test]
fn a_graded_score_keeps_its_lines() {
    let score = sample_score();
    assert_eq!(project_line(&score), " Project Score:  B+ (84.3)");
    assert_eq!(
        dimension_row(&score.dimensions[0]),
        " Cognitive Complexity         30%    88.2   A-   "
    );
    assert_eq!(
        short(&score),
        "score s:84.3 g:B+ files:42 loc:8432 cognitive:88.2 duplication:82.4"
    );
}

#[test]
fn nothing_to_grade_is_null_in_json_with_every_key() {
    let json = serde_json::to_value(to_json(&nothing_to_grade(), Some("README.md"), None)).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "target": "README.md",
            "score": null,
            "grade": null,
            "files_analyzed": 0,
            "total_loc": 0,
            "dimensions": [
                {"name": "Cognitive Complexity", "weight": 0.3, "score": null, "grade": null},
                {"name": "Duplication", "weight": 0.2, "score": null, "grade": null}
            ],
            "needs_attention": []
        })
    );
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

#[test]
fn print_report_truncates_long_path() {
    // Path longer than 40 chars triggers the "..." truncation branch
    let mut score = sample_score();
    score.needs_attention = vec![FileScore {
        path: PathBuf::from("src/very/deeply/nested/directory/structure/with/a/long/path/file.rs"),
        score: 40.0,
        grade: Grade::FMinusMinus,
        loc: 1000,
        issues: vec!["Cognitive: 100".to_string()],
    }];
    // Should not panic even with long paths
    print_report(&score, 10, None);
}
