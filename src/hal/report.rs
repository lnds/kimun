/// Report formatters for Halstead complexity metrics.
///
/// Displays per-file operator/operand counts, volume, effort, estimated
/// bugs, and development time in both table and JSON formats.
use std::path::PathBuf;

use serde::Serialize;

use super::analyzer::HalsteadMetrics;
use crate::report_helpers;

/// Per-file Halstead metrics with path and detected language.
pub struct FileHalsteadMetrics {
    pub path: PathBuf,
    pub language: String,
    pub metrics: HalsteadMetrics,
}

/// Format seconds as a human-readable duration (e.g. "45s", "3m 20s", "2h 15m", "1d 4h").
pub(crate) fn format_time(seconds: f64) -> String {
    if seconds < 60.0 {
        format!("{seconds:.0}s")
    } else if seconds < 3600.0 {
        let m = (seconds / 60.0).floor();
        let s = (seconds % 60.0).round();
        format!("{m:.0}m {s:.0}s")
    } else if seconds < 86400.0 {
        let h = (seconds / 3600.0).floor();
        let m = ((seconds % 3600.0) / 60.0).round();
        format!("{h:.0}h {m:.0}m")
    } else {
        let d = (seconds / 86400.0).floor();
        let h = ((seconds % 86400.0) / 3600.0).round();
        format!("{d:.0}d {h:.0}h")
    }
}

/// Print a table of per-file Halstead metrics with totals row.
pub fn print_report(files: &[FileHalsteadMetrics]) {
    if files.is_empty() {
        println!("No recognized source files found.");
        return;
    }

    let max_path_len = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);
    let header_width = max_path_len + 72;
    let separator = report_helpers::separator(header_width.max(88));

    println!("Halstead Complexity Metrics");
    println!("{separator}");
    println!(
        " {:<width$}  {:>4} {:>4} {:>5} {:>5} {:>9} {:>10} {:>6} {:>8}",
        "File",
        "\u{03b7}\u{2081}",
        "\u{03b7}\u{2082}",
        "N\u{2081}",
        "N\u{2082}",
        "Volume",
        "Effort",
        "Bugs",
        "Time",
        width = max_path_len
    );
    println!("{separator}");

    for f in files {
        let m = &f.metrics;
        println!(
            " {:<width$}  {:>4} {:>4} {:>5} {:>5} {:>9.1} {:>10.0} {:>6.2} {:>8}",
            f.path.display(),
            m.distinct_operators,
            m.distinct_operands,
            m.total_operators,
            m.total_operands,
            m.volume,
            m.effort,
            m.bugs,
            format_time(m.time),
            width = max_path_len
        );
    }

    println!("{separator}");

    let total_n1: usize = files.iter().map(|f| f.metrics.total_operators).sum();
    let total_n2: usize = files.iter().map(|f| f.metrics.total_operands).sum();
    let total_volume: f64 = files.iter().map(|f| f.metrics.volume).sum();
    let total_effort: f64 = files.iter().map(|f| f.metrics.effort).sum();
    let total_bugs: f64 = files.iter().map(|f| f.metrics.bugs).sum();
    let total_time: f64 = files.iter().map(|f| f.metrics.time).sum();

    let total_label = format!(" Total ({} files)", files.len());
    println!(
        "{:<width$}  {:>4} {:>4} {:>5} {:>5} {:>9.1} {:>10.0} {:>6.2} {:>8}",
        total_label,
        "",
        "",
        total_n1,
        total_n2,
        total_volume,
        total_effort,
        total_bugs,
        format_time(total_time),
        width = max_path_len + 1,
    );
}

/// JSON-serializable representation of a file's Halstead metrics.
#[derive(Serialize)]
struct JsonEntry {
    path: String,
    language: String,
    distinct_operators: usize,
    distinct_operands: usize,
    total_operators: usize,
    total_operands: usize,
    vocabulary: usize,
    length: usize,
    volume: f64,
    difficulty: f64,
    effort: f64,
    bugs: f64,
    time: f64,
}

/// Serialize Halstead metrics as pretty-printed JSON to stdout.
pub fn print_json(files: &[FileHalsteadMetrics]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<JsonEntry> = files
        .iter()
        .map(|f| {
            let m = &f.metrics;
            JsonEntry {
                path: f.path.display().to_string(),
                language: f.language.clone(),
                distinct_operators: m.distinct_operators,
                distinct_operands: m.distinct_operands,
                total_operators: m.total_operators,
                total_operands: m.total_operands,
                vocabulary: m.vocabulary,
                length: m.length,
                volume: m.volume,
                difficulty: m.difficulty,
                effort: m.effort,
                bugs: m.bugs,
                time: m.time,
            }
        })
        .collect();

    report_helpers::print_json_stdout(&entries)
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
