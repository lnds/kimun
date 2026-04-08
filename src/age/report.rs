/// Report formatters for code age analysis.
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::analyzer::{AgeStatus, AgeThresholds, FileAge};
use crate::report_helpers;

const COL_LANG: usize = 12;
const COL_DATE: usize = 13; // "Last Modified"
const COL_DAYS: usize = 4; // "Days"
const COL_STATUS: usize = 6; // "FROZEN"

fn format_date(ts: i64) -> String {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Print a table of files sorted by last-modified date (oldest first),
/// followed by a status summary.
pub fn print_report(files: &[FileAge], thresholds: &AgeThresholds) {
    if files.is_empty() {
        println!("No source files found.");
        return;
    }

    let col_path = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);

    let sep_width = 1 + col_path + 1 + COL_LANG + 1 + COL_DATE + 1 + COL_DAYS + 1 + COL_STATUS;
    let separator = report_helpers::separator(sep_width);

    println!("{separator}");
    println!(
        " {:<col_path$} {:<COL_LANG$} {:>COL_DATE$} {:>COL_DAYS$} {:>COL_STATUS$}",
        "File", "Language", "Last Modified", "Days", "Status"
    );
    println!("{separator}");

    for f in files {
        println!(
            " {:<col_path$} {:<COL_LANG$} {:>COL_DATE$} {:>COL_DAYS$} {:>COL_STATUS$}",
            f.path.display(),
            f.language,
            format_date(f.last_modified),
            f.age_days,
            f.status.label(),
        );
    }

    println!("{separator}");
    print_summary(files, thresholds);
}

fn print_summary(files: &[FileAge], thresholds: &AgeThresholds) {
    let active = files
        .iter()
        .filter(|f| f.status == AgeStatus::Active)
        .count();
    let stale = files
        .iter()
        .filter(|f| f.status == AgeStatus::Stale)
        .count();
    let frozen = files
        .iter()
        .filter(|f| f.status == AgeStatus::Frozen)
        .count();
    let a = thresholds.active_days;
    let f = thresholds.frozen_days;
    println!();
    println!("  ACTIVE  {active:>5}  (modified < {a} days)");
    println!("  STALE   {stale:>5}  ({a} days – {f} days)");
    println!("  FROZEN  {frozen:>5}  (not modified > {f} days)");
}

/// JSON-serializable representation of a single file's age data.
#[derive(Serialize)]
struct JsonEntry {
    path: String,
    language: String,
    last_modified: String,
    age_days: u64,
    status: String,
}

/// Serialize file age data as pretty-printed JSON to stdout.
pub fn print_json(files: &[FileAge]) {
    let entries: Vec<JsonEntry> = files
        .iter()
        .map(|f| JsonEntry {
            path: f.path.display().to_string(),
            language: f.language.clone(),
            last_modified: format_date(f.last_modified),
            age_days: f.age_days,
            status: f.status.label().to_string(),
        })
        .collect();

    report_helpers::print_json_stdout(&entries).unwrap();
}

/// Print age as a single compact line.
pub fn print_short(files: &[FileAge]) {
    let count = files.len();
    let active = files
        .iter()
        .filter(|f| f.status == AgeStatus::Active)
        .count();
    let stale = files
        .iter()
        .filter(|f| f.status == AgeStatus::Stale)
        .count();
    let frozen = files
        .iter()
        .filter(|f| f.status == AgeStatus::Frozen)
        .count();
    println!("age files:{count} active:{active} stale:{stale} frozen:{frozen}");
}

/// Print only the file count.
pub fn print_terse(files: &[FileAge]) {
    println!("{}", files.len());
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
