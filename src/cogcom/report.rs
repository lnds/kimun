//! Report formatters for cognitive complexity analysis.
//!
//! Provides three output modes: per-file table, per-function breakdown,
//! and JSON. Based on the SonarSource cognitive complexity specification.
use std::path::PathBuf;

use serde::Serialize;

use super::analyzer::{CognitiveLevel, FunctionCognitive};
use crate::report_helpers::{self, PerFunctionFile, PerFunctionRow};

/// Per-file cognitive complexity metrics, including per-function breakdown.
pub struct FileCogcomMetrics {
    pub path: PathBuf,
    pub language: String,
    pub function_count: usize,
    pub avg_complexity: f64,
    pub max_complexity: usize,
    pub total_complexity: usize,
    pub level: CognitiveLevel,
    pub functions: Vec<FunctionCognitive>,
}

/// Print a table of per-file cognitive complexity with a totals row.
pub fn print_report(files: &[FileCogcomMetrics]) {
    if files.is_empty() {
        println!("No recognized source files found.");
        return;
    }

    let max_path_len = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);
    let header_width = max_path_len + 55;
    let separator = report_helpers::separator(header_width.max(78));

    println!("Cognitive Complexity");
    println!("{separator}");
    println!(
        " {:<width$}  {:>9} {:>5} {:>5} {:>7}  Level",
        "File",
        "Functions",
        "Avg",
        "Max",
        "Total",
        width = max_path_len
    );
    println!("{separator}");

    for f in files {
        println!(
            " {:<width$}  {:>9} {:>5.1} {:>5} {:>7}  {}",
            f.path.display(),
            f.function_count,
            f.avg_complexity,
            f.max_complexity,
            f.total_complexity,
            f.level.as_str(),
            width = max_path_len
        );
    }

    println!("{separator}");

    let total_functions: usize = files.iter().map(|f| f.function_count).sum();
    let total_complexity: usize = files.iter().map(|f| f.total_complexity).sum();
    let max_complexity = files.iter().map(|f| f.max_complexity).max().unwrap_or(0);
    let avg = if total_functions > 0 {
        total_complexity as f64 / total_functions as f64
    } else {
        0.0
    };

    let total_label = format!(" Total ({} files)", files.len());
    println!(
        "{:<width$}  {:>9} {:>5.1} {:>5} {:>7}",
        total_label,
        total_functions,
        avg,
        max_complexity,
        total_complexity,
        width = max_path_len + 1,
    );
}

impl PerFunctionRow for FunctionCognitive {
    fn name(&self) -> &str {
        &self.name
    }
    fn start_line(&self) -> usize {
        self.start_line
    }
    fn complexity(&self) -> usize {
        self.complexity
    }
    fn level_str(&self) -> &str {
        self.level.as_str()
    }
}

impl PerFunctionFile for FileCogcomMetrics {
    type Row = FunctionCognitive;
    fn path_str(&self) -> String {
        self.path.display().to_string()
    }
    fn rows(&self) -> &[FunctionCognitive] {
        &self.functions
    }
}

/// Print per-function cognitive complexity breakdown grouped by file.
pub fn print_per_function(files: &[FileCogcomMetrics]) {
    report_helpers::print_per_function_breakdown("Cognitive Complexity (per function)", files);
}

/// JSON-serializable representation of a single function's complexity.
#[derive(Serialize)]
struct JsonFunctionEntry {
    name: String,
    start_line: usize,
    complexity: usize,
    level: CognitiveLevel,
}

/// JSON-serializable representation of a file's complexity with function details.
#[derive(Serialize)]
struct JsonFileEntry {
    path: String,
    language: String,
    function_count: usize,
    avg_complexity: f64,
    max_complexity: usize,
    total_complexity: usize,
    level: CognitiveLevel,
    functions: Vec<JsonFunctionEntry>,
}

/// Serialize per-file metrics as pretty-printed JSON to stdout.
pub fn print_json(files: &[FileCogcomMetrics]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<JsonFileEntry> = files
        .iter()
        .map(|f| JsonFileEntry {
            path: f.path.display().to_string(),
            language: f.language.clone(),
            function_count: f.function_count,
            avg_complexity: f.avg_complexity,
            max_complexity: f.max_complexity,
            total_complexity: f.total_complexity,
            level: f.level,
            functions: f
                .functions
                .iter()
                .map(|func| JsonFunctionEntry {
                    name: func.name.clone(),
                    start_line: func.start_line,
                    complexity: func.complexity,
                    level: func.level,
                })
                .collect(),
        })
        .collect();

    report_helpers::print_json_stdout(&entries)
}

/// Emit a CodeClimate JSON array (GitLab Code Quality format) for functions
/// that exceed the complexity threshold.
pub fn print_codeclimate(
    files: &[FileCogcomMetrics],
    min_complexity: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    report_helpers::print_codeclimate_complexity(
        files,
        min_complexity,
        "Cognitive Complexity",
        "cognitive",
    )
}

/// Emit one GitHub Actions warning annotation per function that exceeds
/// the complexity threshold. Uses `start_line` for precise line linking.
pub fn print_github(files: &[FileCogcomMetrics], min_complexity: usize) {
    for f in files {
        let path = f.path.display().to_string();
        for func in &f.functions {
            if func.complexity >= min_complexity {
                let message = format!(
                    "function '{}' has cognitive complexity {} (threshold: {})",
                    func.name, func.complexity, min_complexity
                );
                report_helpers::github_annotation(
                    "warning",
                    &path,
                    func.start_line,
                    "Cognitive Complexity",
                    &message,
                );
            }
        }
    }
}

/// Average complexity across all functions in the result set.
fn avg_complexity(files: &[FileCogcomMetrics]) -> f64 {
    let total_fns: usize = files.iter().map(|f| f.functions.len()).sum();
    let total: usize = files.iter().map(|f| f.total_complexity).sum();
    if total_fns > 0 {
        total as f64 / total_fns as f64
    } else {
        0.0
    }
}

/// Print cognitive complexity as a single compact line.
pub fn print_short(files: &[FileCogcomMetrics]) {
    let count = files.len();
    let total_fns: usize = files.iter().map(|f| f.functions.len()).sum();
    let total: usize = files.iter().map(|f| f.total_complexity).sum();
    let max: usize = files.iter().map(|f| f.max_complexity).max().unwrap_or(0);
    let avg = avg_complexity(files);
    println!("cogcom files:{count} fns:{total_fns} avg:{avg:.1} max:{max} total:{total}");
}

/// Print only the average cognitive complexity.
pub fn print_terse(files: &[FileCogcomMetrics]) {
    println!("{:.1}", avg_complexity(files));
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
