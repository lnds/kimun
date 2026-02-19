/// Report formatters for the Visual Studio Maintainability Index.
///
/// Provides table and JSON output showing per-file MI scores with
/// Halstead volume, cyclomatic complexity, LOC, and traffic-light level
/// (green/yellow/red). The table includes a totals row with average MI.
use std::path::PathBuf;

use serde::Serialize;

use super::analyzer::{MILevel, MIMetrics};
use crate::report_helpers;

/// Per-file MI analysis result bundled with filesystem path and language.
pub struct FileMIMetrics {
    pub path: PathBuf,
    pub language: String,
    pub metrics: MIMetrics,
}

/// Print a table of per-file MI metrics with a totals row.
///
/// Columns: File, Volume, Cyclomatic, LOC, MI score, Level.
/// The totals row shows total LOC and average MI across all files.
pub fn print_report(files: &[FileMIMetrics]) {
    if files.is_empty() {
        println!("No recognized source files found.");
        return;
    }

    let max_path_len = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);
    // Width derived from the header format string below:
    // " {path}  {Volume:>9} {Cyclo:>5} {LOC:>5} {MI:>6}  Level"
    let header_width = 1 + max_path_len + 2 + 9 + 1 + 5 + 1 + 5 + 1 + 6 + 2 + 5;
    let separator = report_helpers::separator(header_width.max(70));

    println!("Maintainability Index (Visual Studio)");
    println!("{separator}");
    println!(
        " {:<width$}  {:>9} {:>5} {:>5} {:>6}  Level",
        "File",
        "Volume",
        "Cyclo",
        "LOC",
        "MI",
        width = max_path_len
    );
    println!("{separator}");

    for f in files {
        let m = &f.metrics;
        println!(
            " {:<width$}  {:>9.1} {:>5} {:>5} {:>6.1}  {}",
            f.path.display(),
            m.halstead_volume,
            m.cyclomatic_complexity,
            m.loc,
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
        "{:<width$}  {:>9} {:>5} {:>5} {:>6.1}",
        total_label,
        "",
        "",
        total_loc,
        avg_mi,
        width = max_path_len + 1,
    );
}

/// JSON-serializable representation of a file's MI metrics.
#[derive(Serialize)]
struct JsonEntry {
    path: String,
    language: String,
    halstead_volume: f64,
    cyclomatic_complexity: usize,
    loc: usize,
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
