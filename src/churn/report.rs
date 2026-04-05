/// Report formatters for churn analysis.
///
/// Provides table and JSON output showing per-file commit frequency,
/// churn rate (commits/month), and activity level classification.
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::analyzer::FileChurn;
use crate::report_helpers;

const COL_LANG: usize = 10;
const COL_COMMITS: usize = 7; // "Commits"
const COL_RATE: usize = 10; // "Rate/Month"
const COL_DATE: usize = 10; // "YYYY-MM-DD"
const COL_LEVEL: usize = 6; // "MEDIUM"
// 1 (lead) + 2 (after path) + 1 + 1 + 1 + 1 + 1 (between remaining cols)
const COL_SPACING: usize = 8;
const FIXED_WIDTH: usize = COL_SPACING + COL_LANG + COL_COMMITS + COL_RATE + COL_DATE + COL_LEVEL;

fn format_date(ts: i64) -> String {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Print a table of files sorted by the chosen metric with a summary footer.
pub fn print_report(files: &[FileChurn]) {
    if files.is_empty() {
        println!("No source files found in git history.");
        return;
    }

    let col_path = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);
    let sep_width = col_path + FIXED_WIDTH;
    let separator = report_helpers::separator(sep_width);

    println!("Code Churn — Change Frequency by File");
    println!("{separator}");
    println!(
        " {:<col_path$}  {:>COL_LANG$} {:>COL_COMMITS$} {:>COL_RATE$} {:>COL_DATE$} {:>COL_LEVEL$}",
        "File", "Language", "Commits", "Rate/Month", "Last Commit", "Level",
    );
    println!("{separator}");

    for f in files {
        println!(
            " {:<col_path$}  {:>COL_LANG$} {:>COL_COMMITS$} {:>COL_RATE$.2} {:>COL_DATE$} {:>COL_LEVEL$}",
            f.path.display(),
            f.language,
            f.commits,
            f.rate,
            format_date(f.last_commit),
            f.level.label(),
        );
    }

    println!("{separator}");
    print_summary(files);
}

fn print_summary(files: &[FileChurn]) {
    use super::analyzer::ChurnLevel;
    let high = files.iter().filter(|f| f.level == ChurnLevel::High).count();
    let medium = files
        .iter()
        .filter(|f| f.level == ChurnLevel::Medium)
        .count();
    let low = files.iter().filter(|f| f.level == ChurnLevel::Low).count();
    println!();
    println!("  HIGH    {high:>5}  (> 4 commits/month — moving targets)");
    println!("  MEDIUM  {medium:>5}  (1–4 commits/month — active development)");
    println!("  LOW     {low:>5}  (< 1 commit/month — stable)");
}

/// JSON-serializable representation of a single file's churn data.
#[derive(Serialize)]
struct JsonEntry {
    path: String,
    language: String,
    commits: usize,
    rate_per_month: f64,
    first_commit: String,
    last_commit: String,
    level: String,
}

/// Serialize churn data as pretty-printed JSON to stdout.
pub fn print_json(files: &[FileChurn]) {
    let entries: Vec<JsonEntry> = files
        .iter()
        .map(|f| JsonEntry {
            path: f.path.display().to_string(),
            language: f.language.clone(),
            commits: f.commits,
            rate_per_month: (f.rate * 100.0).round() / 100.0,
            first_commit: format_date(f.first_commit),
            last_commit: format_date(f.last_commit),
            level: f.level.label().to_string(),
        })
        .collect();

    report_helpers::print_json_stdout(&entries).unwrap();
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
