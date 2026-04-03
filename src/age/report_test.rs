use super::*;
use crate::age::analyzer::{AgeStatus, AgeThresholds, FileAge, classify};
use std::path::PathBuf;

const NOW: i64 = 1_700_000_000;
const DAY: i64 = 86_400;

fn sample() -> Vec<FileAge> {
    vec![
        classify(
            PathBuf::from("src/foo.rs"),
            "Rust",
            NOW - 30 * DAY,
            NOW,
            &AgeThresholds::default(),
        ),
        classify(
            PathBuf::from("src/bar.rs"),
            "Rust",
            NOW - 180 * DAY,
            NOW,
            &AgeThresholds::default(),
        ),
        classify(
            PathBuf::from("src/old.rs"),
            "Rust",
            NOW - 500 * DAY,
            NOW,
            &AgeThresholds::default(),
        ),
    ]
}

#[test]
fn print_report_does_not_panic() {
    print_report(&sample(), &AgeThresholds::default());
}

#[test]
fn print_report_empty() {
    print_report(&[], &AgeThresholds::default());
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
    let sep_width = 1 + col_path + 1 + COL_LANG + 1 + COL_DATE + 1 + COL_DAYS + 1 + COL_STATUS;

    let row = format!(
        " {:<col_path$} {:<COL_LANG$} {:>COL_DATE$} {:>COL_DAYS$} {:>COL_STATUS$}",
        "File", "Language", "Last Modified", "Days", "Status"
    );
    assert_eq!(row.len(), sep_width);
}
