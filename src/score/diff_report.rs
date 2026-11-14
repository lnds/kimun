//! Report formatters for the score diff output.
//!
//! Provides table (ANSI-colored) and JSON output modes for `ScoreDiff`.
//! Green for improvements (+), red for regressions (-), yellow for no change.

use serde::Serialize;

use super::analyzer::{Grade, Graded, grade_text, score_text};
use super::changed::ChangedScope;
use super::diff::ScoreDiff;
use super::report::JsonGraded;
use crate::report_helpers;

// ANSI color codes.
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// Color and signed text for a delta. A delta that rounds to zero at
/// `decimals` places is shown unsigned, not as `-0.0`.
fn delta_parts(delta: f64, decimals: usize) -> (&'static str, String) {
    let half_unit = 0.5 / 10f64.powi(decimals as i32);
    if delta > half_unit {
        (GREEN, format!("+{delta:.decimals$}"))
    } else if delta < -half_unit {
        (RED, format!("{delta:.decimals$}"))
    } else {
        (YELLOW, format!(" {:.decimals$}", 0.0))
    }
}

fn colored_delta(delta: f64, decimals: usize) -> String {
    let (color, text) = delta_parts(delta, decimals);
    format!("{color}{text}{RESET}")
}

/// The colored delta, or `-` when a side has nothing to grade.
fn shown_delta(delta: Option<f64>, decimals: usize) -> String {
    delta.map_or_else(|| "-".to_string(), |d| colored_delta(d, decimals))
}

/// The signed delta without color, or `-` when a side has nothing to grade.
fn plain_delta(delta: Option<f64>) -> String {
    delta.map_or_else(|| "-".to_string(), |d| format!("{d:+.2}"))
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

    println!("{}", overall_line(diff));

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
            " {:<25} {:>5.0}%   {:>5} {:<3}   {:>5} {:<3}   {}",
            d.name,
            d.weight * 100.0,
            score_text(d.before, 1),
            grade_text(d.before),
            score_text(d.after, 1),
            grade_text(d.after),
            shown_delta(d.delta, 1),
        );
    }

    println!("{separator}");

    if let Some(scope) = &diff.changed {
        print_changed_files(scope, diff.before, &separator);
    }
}

/// The overall line: the scores, their delta and the grade, which is shown
/// once when it did not change. `-` stands for nothing to grade.
fn overall_line(diff: &ScoreDiff) -> String {
    let (before, after) = (grade_text(diff.before), grade_text(diff.after));
    let grade_change = if before == after {
        after.to_string()
    } else {
        format!("{before} → {after}")
    };
    format!(
        " Overall Score: {} → {}  ({})  Grade: {grade_change}",
        score_text(diff.before, 2),
        score_text(diff.after, 2),
        shown_delta(diff.delta, 2),
    )
}

fn print_changed_files(scope: &ChangedScope, baseline: Option<Graded>, separator: &str) {
    println!();
    println!(" Changed Files (--gate-scope changed)");
    println!("{separator}");
    println!(" {:>7}  {:>7}  {:>8}  File", "Before", "After", "Delta");
    println!("{separator}");
    for f in &scope.files {
        let (before, (color, delta)) = match (f.before, f.delta) {
            (Some(b), Some(d)) => (format!("{b:.2}"), delta_parts(d, 2)),
            _ => ("-".to_string(), ("", "new".to_string())),
        };
        println!(
            " {before:>7}  {:>7.2}  {color}{delta:>8}{RESET}  {}",
            f.after, f.path
        );
    }
    if scope.files.is_empty() {
        println!(" No scored source files changed.");
    }
    println!("{separator}");
    println!("{}", baseline_line(baseline));
    let (dup_before, dup_after) = (scope.duplicated_lines_before, scope.duplicated_lines_after);
    println!(
        " Duplicated lines: {dup_before} → {dup_after}  ({})",
        colored_int_delta(dup_after as i64 - dup_before as i64)
    );
}

/// What the changed files are held to: the project score at the ref, or,
/// with nothing graded there, only the duplicated lines.
fn baseline_line(baseline: Option<Graded>) -> String {
    match baseline {
        Some(g) => format!(
            " Files ending below the ref score ({:.2}) may not drop more than the tolerance.",
            g.score
        ),
        None => " Nothing was graded at the ref: only duplicated lines are compared.".to_string(),
    }
}

/// Print score diff as a single compact line.
pub fn print_short(diff: &ScoreDiff) {
    println!("{}", short(diff));
}

/// The compact line: `-` for a side with nothing to grade.
fn short(diff: &ScoreDiff) -> String {
    format!(
        "score-diff ref:{} before:{} after:{} delta:{} grade:{}",
        diff.git_ref,
        score_text(diff.before, 2),
        score_text(diff.after, 2),
        plain_delta(diff.delta),
        grade_text(diff.after),
    )
}

/// Print only the score delta, `-` when a side has nothing to grade.
pub fn print_terse(diff: &ScoreDiff) {
    println!("{}", terse(diff));
}

/// The delta alone, `-` when a side has nothing to grade.
fn terse(diff: &ScoreDiff) -> String {
    plain_delta(diff.delta)
}

// --- JSON output ---

#[derive(Serialize)]
struct JsonScoreSnapshot {
    #[serde(flatten)]
    graded: JsonGraded,
    files: usize,
    loc: usize,
}

/// Keeps its `before_score`/`before_grade` keys: serde cannot prefix the
/// keys of a flattened `JsonGraded`.
#[derive(Serialize)]
struct JsonDimensionDelta {
    name: String,
    weight: f64,
    before_score: Option<f64>,
    before_grade: Option<Grade>,
    after_score: Option<f64>,
    after_grade: Option<Grade>,
    delta: Option<f64>,
}

#[derive(Serialize)]
struct JsonScoreDiff<'a> {
    git_ref: String,
    before: JsonScoreSnapshot,
    after: JsonScoreSnapshot,
    delta: Option<f64>,
    dimensions: Vec<JsonDimensionDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    changed: Option<&'a ChangedScope>,
}

/// Serialize the score diff as pretty-printed JSON to stdout.
pub fn print_json(diff: &ScoreDiff) -> Result<(), Box<dyn std::error::Error>> {
    report_helpers::print_json_stdout(&to_json(diff))
}

/// The JSON of the diff. Every key is present; what a side with nothing
/// to grade lacks is null.
fn to_json(diff: &ScoreDiff) -> JsonScoreDiff<'_> {
    JsonScoreDiff {
        git_ref: diff.git_ref.clone(),
        before: JsonScoreSnapshot {
            graded: diff.before.into(),
            files: diff.files_before,
            loc: diff.loc_before,
        },
        after: JsonScoreSnapshot {
            graded: diff.after.into(),
            files: diff.files_after,
            loc: diff.loc_after,
        },
        delta: diff.delta,
        dimensions: diff
            .dimensions
            .iter()
            .map(|d| JsonDimensionDelta {
                name: d.name.clone(),
                weight: d.weight,
                before_score: d.before.map(|g| g.score),
                before_grade: d.before.map(|g| g.grade),
                after_score: d.after.map(|g| g.score),
                after_grade: d.after.map(|g| g.grade),
                delta: d.delta,
            })
            .collect(),
        changed: diff.changed.as_ref(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::analyzer::{DimensionScore, Grade, ProjectScore};
    use crate::score::diff::compute_diff;

    /// A two-dimension score; `None` for nothing to grade.
    fn graded_or_not(score: Option<f64>, files: usize, loc: usize) -> ProjectScore {
        let graded = score.map(Graded::of);
        let dimension = |name| DimensionScore {
            name,
            weight: 0.50,
            graded,
        };
        ProjectScore {
            graded,
            files_analyzed: files,
            total_loc: loc,
            dimensions: vec![dimension("Cognitive Complexity"), dimension("Duplication")],
            needs_attention: vec![],
        }
    }

    #[test]
    fn print_report_improvement_does_not_panic() {
        let before = graded_or_not(Some(70.0), 10, 1000);
        let after = graded_or_not(Some(80.0), 12, 1200);
        let diff = compute_diff("HEAD~1", &before, &after);
        print_report(&diff);
    }

    #[test]
    fn print_report_regression_does_not_panic() {
        let before = graded_or_not(Some(80.0), 10, 1000);
        let after = graded_or_not(Some(70.0), 8, 900);
        let diff = compute_diff("main", &before, &after);
        print_report(&diff);
    }

    #[test]
    fn print_report_no_change_does_not_panic() {
        let before = graded_or_not(Some(85.0), 10, 1000);
        let after = graded_or_not(Some(85.0), 10, 1000);
        let diff = compute_diff("v1.0", &before, &after);
        print_report(&diff);
    }

    #[test]
    fn print_json_does_not_panic() {
        let before = graded_or_not(Some(75.0), 5, 500);
        let after = graded_or_not(Some(85.0), 7, 700);
        let diff = compute_diff("abc123", &before, &after);
        print_json(&diff).unwrap();
    }

    #[test]
    fn print_json_regression_does_not_panic() {
        let before = graded_or_not(Some(90.0), 10, 1000);
        let after = graded_or_not(Some(60.0), 10, 1000);
        let diff = compute_diff("v2.0", &before, &after);
        print_json(&diff).unwrap();
    }

    #[test]
    fn colored_delta_positive() {
        let s = colored_delta(5.0, 1);
        assert!(s.contains('+'), "positive delta should have + prefix: {s}");
    }

    #[test]
    fn colored_delta_negative() {
        let s = colored_delta(-5.0, 1);
        assert!(s.contains('-'), "negative delta should have - prefix: {s}");
    }

    #[test]
    fn colored_delta_near_zero() {
        let s = colored_delta(0.0, 1);
        assert!(s.contains("0.0"), "zero delta should show 0.0: {s}");
    }

    #[test]
    fn colored_delta_two_decimals_shows_sub_tenth_drop() {
        let s = colored_delta(-0.0123, 2);
        assert!(s.contains("-0.01"), "a -0.0123 drop should show -0.01: {s}");
    }

    #[test]
    fn colored_delta_rounding_to_zero_is_unsigned() {
        let s = colored_delta(-0.004, 2);
        assert!(
            s.contains(" 0.00") && !s.contains('-'),
            "a drop that rounds to zero should show 0.00 without a sign: {s}"
        );
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
        let before = graded_or_not(Some(80.0), 10, 1000);
        let after = graded_or_not(Some(81.0), 10, 1000);
        let diff = compute_diff("HEAD", &before, &after);
        assert_eq!(diff.before.map(|g| g.grade), Some(Grade::B));
        assert_eq!(diff.after.map(|g| g.grade), Some(Grade::B));
        assert!(overall_line(&diff).ends_with("Grade: B"));
    }

    #[test]
    fn different_grades_shows_transition() {
        let before = graded_or_not(Some(70.0), 10, 1000); // C
        let after = graded_or_not(Some(80.0), 10, 1000); // B
        let diff = compute_diff("HEAD", &before, &after);
        assert_eq!(diff.before.map(|g| g.grade), Some(Grade::C));
        assert_eq!(diff.after.map(|g| g.grade), Some(Grade::B));
        assert!(overall_line(&diff).ends_with("Grade: C → B"));
    }

    #[test]
    fn a_side_with_nothing_to_grade_prints_dashes() {
        let before = graded_or_not(Some(70.0), 4, 300);
        let after = graded_or_not(None, 0, 0);
        let diff = compute_diff("HEAD", &before, &after);

        assert_eq!(
            short(&diff),
            "score-diff ref:HEAD before:70.00 after:- delta:- grade:-"
        );
        assert_eq!(terse(&diff), "-");
        assert_eq!(
            overall_line(&diff),
            " Overall Score: 70.00 → -  (-)  Grade: C → -"
        );
        let json = serde_json::to_value(to_json(&diff)).unwrap();
        assert_eq!(
            json["after"],
            serde_json::json!({"score": null, "grade": null, "files": 0, "loc": 0})
        );
        assert_eq!(json["delta"], serde_json::Value::Null);
        assert_eq!(
            json["dimensions"][0],
            serde_json::json!({
                "name": "Cognitive Complexity",
                "weight": 0.5,
                "before_score": 70.0,
                "before_grade": "C",
                "after_score": null,
                "after_grade": null,
                "delta": null
            })
        );
    }

    #[test]
    fn changed_files_with_nothing_graded_at_the_ref_are_held_to_duplication() {
        assert_eq!(
            baseline_line(Some(Graded::of(91.523))),
            " Files ending below the ref score (91.52) may not drop more than the tolerance."
        );
        assert_eq!(
            baseline_line(None),
            " Nothing was graded at the ref: only duplicated lines are compared."
        );
    }

    #[test]
    fn nothing_to_grade_on_both_sides_shows_one_dash_for_the_grade() {
        let none = graded_or_not(None, 0, 0);
        let diff = compute_diff("HEAD", &none, &none);
        assert_eq!(overall_line(&diff), " Overall Score: - → -  (-)  Grade: -");
    }
}
