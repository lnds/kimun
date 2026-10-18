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
/// The totals row shows total LOC and the MI of the files weighted by their
/// code lines (`report_helpers::weighted_mi`).
pub fn print_report(files: &[FileMIMetrics], shown: usize) {
    if files.is_empty() {
        println!("No recognized source files found.");
        return;
    }

    let max_path_len = report_helpers::path_column(files.iter().map(|f| f.path.as_path()), shown);
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

    for f in files.iter().take(shown) {
        let m = &f.metrics;
        println!(
            " {:<width$}  {:>9.1} {:>5} {:>5} {:>6.1}  {}",
            crate::report_helpers::shown(&f.path),
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
    let avg_mi =
        report_helpers::weighted_mi(files.iter().map(|f| (f.metrics.mi_score, f.metrics.loc)));
    let total_loc: usize = files.iter().map(|f| f.metrics.loc).sum();
    let total_label = report_helpers::total_label(count, shown);
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
                path: crate::report_helpers::shown(&f.path),
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

/// Print MI as a single compact line.
pub fn print_short(files: &[FileMIMetrics]) {
    let count = files.len();
    let avg_mi =
        report_helpers::weighted_mi(files.iter().map(|f| (f.metrics.mi_score, f.metrics.loc)));
    let green = files
        .iter()
        .filter(|f| f.metrics.level == MILevel::Green)
        .count();
    let yellow = files
        .iter()
        .filter(|f| f.metrics.level == MILevel::Yellow)
        .count();
    let red = files
        .iter()
        .filter(|f| f.metrics.level == MILevel::Red)
        .count();
    println!("mi files:{count} avg:{avg_mi:.1} green:{green} yellow:{yellow} red:{red}");
}

/// Print only the MI of the files, weighted by their code lines.
pub fn print_terse(files: &[FileMIMetrics]) {
    let avg_mi =
        report_helpers::weighted_mi(files.iter().map(|f| (f.metrics.mi_score, f.metrics.loc)));
    println!("{avg_mi:.1}");
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
