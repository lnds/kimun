//! Report formatters for the score diff output.
//!
//! Provides table (ANSI-colored) and JSON output modes for `ScoreDiff`.
//! Green for improvements (+), red for regressions (-), yellow for no change.

use serde::Serialize;

use super::analyzer::Grade;
use super::diff::ScoreDiff;
use crate::report_helpers;

// ANSI color codes.
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// Format a signed delta with color and sign prefix.
fn colored_delta(delta: f64) -> String {
    if delta > 0.05 {
        format!("{GREEN}+{delta:.1}{RESET}")
    } else if delta < -0.05 {
        format!("{RED}{delta:.1}{RESET}")
    } else {
        format!("{YELLOW} 0.0{RESET}")
    }
}

/// Format a signed integer delta with color and sign prefix.
fn colored_int_delta(delta: i64) -> String {
    if delta > 0 {
        format!("{GREEN}+{delta}{RESET}")
    } else if delta < 0 {
        format!("{RED}{delta}{RESET}")
    } else {
        format!("{YELLOW} 0{RESET}")
    }
}

/// Print the score diff as a formatted table with ANSI colors.
pub fn print_report(diff: &ScoreDiff) {
    let separator = report_helpers::separator(72);

    println!("Score Diff vs {BOLD}{}{RESET}", diff.git_ref);
    println!("{separator}");

    let grade_change = if diff.before_grade == diff.after_grade {
        diff.after_grade.as_str().to_string()
    } else {
        format!(
            "{} → {}",
            diff.before_grade.as_str(),
            diff.after_grade.as_str()
        )
    };

    println!(
        " Overall Score: {:.1} → {:.1}  ({})  Grade: {grade_change}",
        diff.overall.before,
        diff.overall.after,
        colored_delta(diff.overall.delta),
    );

    let files_delta = diff.files_after as i64 - diff.files_before as i64;
    let loc_delta = diff.loc_after as i64 - diff.loc_before as i64;
    println!(
        " Files: {} → {}  ({})    LOC: {} → {}  ({})",
        diff.files_before,
        diff.files_after,
        colored_int_delta(files_delta),
        diff.loc_before,
        diff.loc_after,
        colored_int_delta(loc_delta),
    );

    println!("{separator}");
    println!(
        " {:<25} {:>6}   {:>10}   {:>10}   {:>7}",
        "Dimension", "Weight", "Before", "After", "Delta"
    );
    println!("{separator}");

    for d in &diff.dimensions {
        println!(
            " {:<25} {:>5.0}%   {:>5.1} {:<3}   {:>5.1} {:<3}   {}",
            d.name,
            d.weight * 100.0,
            d.before_score,
            d.before_grade.as_str(),
            d.after_score,
            d.after_grade.as_str(),
            colored_delta(d.delta),
        );
    }

    println!("{separator}");
}

// --- JSON output ---

#[derive(Serialize)]
struct JsonScoreSnapshot {
    score: f64,
    grade: Grade,
    files: usize,
    loc: usize,
}

#[derive(Serialize)]
struct JsonDimensionDelta {
    name: String,
    weight: f64,
    before_score: f64,
    before_grade: Grade,
    after_score: f64,
    after_grade: Grade,
    delta: f64,
}

#[derive(Serialize)]
struct JsonScoreDiff {
    git_ref: String,
    before: JsonScoreSnapshot,
    after: JsonScoreSnapshot,
    delta: f64,
    dimensions: Vec<JsonDimensionDelta>,
}

/// Serialize the score diff as pretty-printed JSON to stdout.
pub fn print_json(diff: &ScoreDiff) -> Result<(), Box<dyn std::error::Error>> {
    let json = JsonScoreDiff {
        git_ref: diff.git_ref.clone(),
        before: JsonScoreSnapshot {
            score: diff.overall.before,
            grade: diff.before_grade,
            files: diff.files_before,
            loc: diff.loc_before,
        },
        after: JsonScoreSnapshot {
            score: diff.overall.after,
            grade: diff.after_grade,
            files: diff.files_after,
            loc: diff.loc_after,
        },
        delta: diff.overall.delta,
        dimensions: diff
            .dimensions
            .iter()
            .map(|d| JsonDimensionDelta {
                name: d.name.clone(),
                weight: d.weight,
                before_score: d.before_score,
                before_grade: d.before_grade,
                after_score: d.after_score,
                after_grade: d.after_grade,
                delta: d.delta,
            })
            .collect(),
    };
    report_helpers::print_json_stdout(&json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::analyzer::{DimensionScore, Grade, ProjectScore, score_to_grade};
    use crate::score::diff::compute_diff;

    fn make_project_score(score: f64, files: usize, loc: usize, dim_score: f64) -> ProjectScore {
        let grade = score_to_grade(score);
        let dim_grade = score_to_grade(dim_score);
        ProjectScore {
            score,
            grade,
            files_analyzed: files,
            total_loc: loc,
            dimensions: vec![
                DimensionScore {
                    name: "Cognitive Complexity",
                    weight: 0.50,
                    score: dim_score,
                    grade: dim_grade,
                },
                DimensionScore {
                    name: "Duplication",
                    weight: 0.50,
                    score: dim_score,
                    grade: dim_grade,
                },
            ],
            needs_attention: vec![],
        }
    }

    #[test]
    fn print_report_improvement_does_not_panic() {
        let before = make_project_score(70.0, 10, 1000, 70.0);
        let after = make_project_score(80.0, 12, 1200, 80.0);
        let diff = compute_diff("HEAD~1", &before, &after);
        print_report(&diff);
    }

    #[test]
    fn print_report_regression_does_not_panic() {
        let before = make_project_score(80.0, 10, 1000, 80.0);
        let after = make_project_score(70.0, 8, 900, 70.0);
        let diff = compute_diff("main", &before, &after);
        print_report(&diff);
    }

    #[test]
    fn print_report_no_change_does_not_panic() {
        let before = make_project_score(85.0, 10, 1000, 85.0);
        let after = make_project_score(85.0, 10, 1000, 85.0);
        let diff = compute_diff("v1.0", &before, &after);
        print_report(&diff);
    }

    #[test]
    fn print_json_does_not_panic() {
        let before = make_project_score(75.0, 5, 500, 75.0);
        let after = make_project_score(85.0, 7, 700, 85.0);
        let diff = compute_diff("abc123", &before, &after);
        print_json(&diff).unwrap();
    }

    #[test]
    fn print_json_regression_does_not_panic() {
        let before = make_project_score(90.0, 10, 1000, 90.0);
        let after = make_project_score(60.0, 10, 1000, 60.0);
        let diff = compute_diff("v2.0", &before, &after);
        print_json(&diff).unwrap();
    }

    #[test]
    fn colored_delta_positive() {
        let s = colored_delta(5.0);
        assert!(s.contains('+'), "positive delta should have + prefix: {s}");
    }

    #[test]
    fn colored_delta_negative() {
        let s = colored_delta(-5.0);
        assert!(s.contains('-'), "negative delta should have - prefix: {s}");
    }

    #[test]
    fn colored_delta_near_zero() {
        let s = colored_delta(0.0);
        assert!(s.contains("0.0"), "zero delta should show 0.0: {s}");
    }

    #[test]
    fn colored_int_delta_positive() {
        let s = colored_int_delta(3);
        assert!(
            s.contains('+'),
            "positive int delta should have + prefix: {s}"
        );
    }

    #[test]
    fn colored_int_delta_negative() {
        let s = colored_int_delta(-3);
        assert!(
            s.contains('-'),
            "negative int delta should have - prefix: {s}"
        );
    }

    #[test]
    fn colored_int_delta_zero() {
        let s = colored_int_delta(0);
        assert!(s.contains('0'), "zero int delta should show 0: {s}");
    }

    #[test]
    fn same_grade_shows_single_grade() {
        // When before and after grade are the same, should just show the grade once
        let before = make_project_score(80.0, 10, 1000, 80.0);
        let after = make_project_score(81.0, 10, 1000, 81.0);
        let diff = compute_diff("HEAD", &before, &after);
        assert_eq!(diff.before_grade, Grade::B);
        assert_eq!(diff.after_grade, Grade::B);
        // print_report should not panic with same grades
        print_report(&diff);
    }

    #[test]
    fn different_grades_shows_transition() {
        let before = make_project_score(70.0, 10, 1000, 70.0); // C
        let after = make_project_score(80.0, 10, 1000, 80.0); // B
        let diff = compute_diff("HEAD", &before, &after);
        assert_eq!(diff.before_grade, Grade::C);
        assert_eq!(diff.after_grade, Grade::B);
        print_report(&diff);
    }
}
