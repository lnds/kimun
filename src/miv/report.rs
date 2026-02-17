/// Report formatters for the verifysoft Maintainability Index.
///
/// Provides table and JSON output showing per-file MI scores with
/// Halstead volume, cyclomatic complexity, LOC, comment percentage,
/// and the comment-weight contribution (MIwoc vs MI).
use std::path::PathBuf;

use serde::Serialize;

use super::analyzer::{MILevel, MIMetrics};
use crate::report_helpers;

/// Per-file MI metrics bundled with filesystem path and language name.
pub struct FileMIMetrics {
    pub path: PathBuf,
    pub language: String,
    pub metrics: MIMetrics,
}

/// Print a table of per-file MI metrics with a totals row showing
/// average MI and total LOC across all analyzed files.
pub fn print_report(files: &[FileMIMetrics]) {
    if files.is_empty() {
        println!("No recognized source files found.");
        return;
    }

    let max_path_len = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);
    // Width derived from the header format string below:
    // " {path}  {Volume:>9} {Cyclo:>5} {LOC:>5} {Cmt%:>5} {MIwoc:>7} {MI:>7}  Level"
    let header_width = 1 + max_path_len + 2 + 9 + 1 + 5 + 1 + 5 + 1 + 5 + 1 + 7 + 1 + 7 + 2 + 5;
    let separator = report_helpers::separator(header_width.max(80));

    println!("Maintainability Index");
    println!("{separator}");
    println!(
        " {:<width$}  {:>9} {:>5} {:>5} {:>5} {:>7} {:>7}  Level",
        "File",
        "Volume",
        "Cyclo",
        "LOC",
        "Cmt%",
        "MIwoc",
        "MI",
        width = max_path_len
    );
    println!("{separator}");

    for f in files {
        let m = &f.metrics;
        println!(
            " {:<width$}  {:>9.1} {:>5} {:>5} {:>5.1} {:>7.1} {:>7.1}  {}",
            f.path.display(),
            m.halstead_volume,
            m.cyclomatic_complexity,
            m.loc,
            m.comment_percent,
            m.mi_woc,
            m.mi_score,
            m.level.as_str(),
            width = max_path_len
        );
    }

    println!("{separator}");

    let count = files.len();
    let avg_mi: f64 = files.iter().map(|f| f.metrics.mi_score).sum::<f64>() / count as f64;
    let total_loc: usize = files.iter().map(|f| f.metrics.loc).sum();
    let total_label = format!(" Total ({count} files)");
    println!(
        "{:<width$}  {:>9} {:>5} {:>5} {:>5} {:>7} {:>7.1}",
        total_label,
        "",
        "",
        total_loc,
        "",
        "",
        avg_mi,
        width = max_path_len + 1,
    );
}

/// JSON-serializable representation of a file's MI breakdown.
#[derive(Serialize)]
struct JsonEntry {
    path: String,
    language: String,
    halstead_volume: f64,
    cyclomatic_complexity: usize,
    loc: usize,
    comment_lines: usize,
    comment_percent: f64,
    mi_woc: f64,
    mi_cw: f64,
    mi_score: f64,
    level: MILevel,
}

/// Serialize per-file MI data as pretty-printed JSON to stdout.
pub fn print_json(files: &[FileMIMetrics]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<JsonEntry> = files
        .iter()
        .map(|f| {
            let m = &f.metrics;
            JsonEntry {
                path: f.path.display().to_string(),
                language: f.language.clone(),
                halstead_volume: m.halstead_volume,
                cyclomatic_complexity: m.cyclomatic_complexity,
                loc: m.loc,
                comment_lines: m.comment_lines,
                comment_percent: m.comment_percent,
                mi_woc: m.mi_woc,
                mi_cw: m.mi_cw,
                mi_score: m.mi_score,
                level: m.level,
            }
        })
        .collect();

    report_helpers::print_json_stdout(&entries)
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
