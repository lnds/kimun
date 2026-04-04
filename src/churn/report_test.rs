use super::*;
use crate::churn::analyzer::{ChurnLevel, FileChurn, classify};
use std::path::PathBuf;

const NOW: i64 = 1_700_000_000;
const MONTH: i64 = 30 * 86_400;

fn sample() -> Vec<FileChurn> {
    vec![
        classify(
            PathBuf::from("src/hot.rs"),
            "Rust",
            20,
            NOW - 2 * MONTH,
            NOW,
        ),
        classify(
            PathBuf::from("src/active.rs"),
            "Rust",
            3,
            NOW - 3 * MONTH,
            NOW,
        ),
        classify(
            PathBuf::from("src/stable.rs"),
            "Rust",
            1,
            NOW - 12 * MONTH,
            NOW,
        ),
    ]
}

#[test]
fn print_report_does_not_panic() {
    print_report(&sample());
}

#[test]
fn print_report_empty() {
    print_report(&[]);
}

#[test]
fn print_json_does_not_panic() {
    print_json(&sample());
}

#[test]
fn print_json_empty() {
    print_json(&[]);
}

#[test]
fn separator_matches_row_width() {
    let files = sample();
    let col_path = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);
    let sep_width = col_path + FIXED_WIDTH;

    let row = format!(
        " {:<col_path$}  {:>COL_LANG$} {:>COL_COMMITS$} {:>COL_RATE$} {:>COL_DATE$} {:>COL_LEVEL$}",
        "File", "Language", "Commits", "Rate/Month", "Last Commit", "Level",
    );
    assert_eq!(row.len(), sep_width);
}

#[test]
fn sample_levels_are_correct() {
    let files = sample();
    assert_eq!(files[0].level, ChurnLevel::High); // 10/month
    assert_eq!(files[1].level, ChurnLevel::Medium); // 1/month
    assert_eq!(files[2].level, ChurnLevel::Low); // ~0.08/month
}
