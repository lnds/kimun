/// Report formatters for indentation complexity analysis.
///
/// Displays per-file indentation stddev and max depth, classified by
/// Adam Tornhill's complexity thresholds ("Your Code as a Crime Scene").
use std::path::PathBuf;

use serde::Serialize;

use super::analyzer::ComplexityLevel;
use crate::report_helpers;

/// Per-file indentation metrics with classification.
pub struct FileIndentMetrics {
    pub path: PathBuf,
    pub code_lines: usize,
    pub stddev: f64,
    pub max_depth: usize,
    pub total_indent: usize,
    pub complexity: ComplexityLevel,
}

/// Print a table of per-file indentation complexity sorted by stddev.
pub fn print_report(files: &[FileIndentMetrics]) {
    if files.is_empty() {
        println!("No recognized source files found.");
        return;
    }

    let max_path_len = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);
    let header_width = max_path_len + 42; // path + numbers + complexity
    let separator = report_helpers::separator(header_width.max(68));

    println!("{separator}");
    println!(
        " {:<width$} {:>8} {:>6} {:>5}  Complexity",
        "File",
        "Lines",
        "StdDev",
        "Max",
        width = max_path_len
    );
    println!("{separator}");

    for f in files {
        println!(
            " {:<width$} {:>8} {:>6.2} {:>5}  {}",
            f.path.display(),
            f.code_lines,
            f.stddev,
            f.max_depth,
            f.complexity.as_str(),
            width = max_path_len
        );
    }

    println!("{separator}");
    println!();
    println!(" Complexity based on indentation stddev (Adam Tornhill,");
    println!(" \"Your Code as a Crime Scene\", Ch.6). Thresholds are heuristic.");
}

/// JSON-serializable representation of a file's indentation metrics.
#[derive(Serialize)]
struct JsonFileEntry {
    path: String,
    code_lines: usize,
    indent_stddev: f64,
    indent_max: usize,
    complexity: ComplexityLevel,
}

/// Serialize indentation metrics as pretty-printed JSON to stdout.
pub fn print_json(files: &[FileIndentMetrics]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<JsonFileEntry> = files
        .iter()
        .map(|f| JsonFileEntry {
            path: f.path.display().to_string(),
            code_lines: f.code_lines,
            indent_stddev: f.stddev,
            indent_max: f.max_depth,
            complexity: f.complexity,
        })
        .collect();

    report_helpers::print_json_stdout(&entries)
}

/// Print indentation metrics as a single compact line.
pub fn print_short(files: &[FileIndentMetrics]) {
    let count = files.len();
    let avg_sd = if count > 0 {
        files.iter().map(|f| f.stddev).sum::<f64>() / count as f64
    } else {
        0.0
    };
    let max_sd = files.iter().map(|f| f.stddev).fold(0.0_f64, f64::max);
    let max_depth = files.iter().map(|f| f.max_depth).max().unwrap_or(0);
    println!("indent files:{count} avg_sd:{avg_sd:.2} max_sd:{max_sd:.2} max_depth:{max_depth}");
}

/// Print only the average standard deviation.
pub fn print_terse(files: &[FileIndentMetrics]) {
    let count = files.len();
    let avg_sd = if count > 0 {
        files.iter().map(|f| f.stddev).sum::<f64>() / count as f64
    } else {
        0.0
    };
    println!("{avg_sd:.2}");
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
