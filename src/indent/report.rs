use std::path::PathBuf;

use serde::Serialize;

use super::analyzer::ComplexityLevel;
use crate::report_helpers;

pub struct FileIndentMetrics {
    pub path: PathBuf,
    pub code_lines: usize,
    pub stddev: f64,
    pub max_depth: usize,
    pub total_indent: usize,
    pub complexity: ComplexityLevel,
}

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

#[derive(Serialize)]
struct JsonFileEntry {
    path: String,
    code_lines: usize,
    indent_stddev: f64,
    indent_max: usize,
    complexity: ComplexityLevel,
}

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

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
