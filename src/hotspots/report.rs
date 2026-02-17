use serde::Serialize;

use super::FileHotspot;
use crate::report_helpers;

fn complexity_label(metric: &str) -> &'static str {
    match metric {
        "cycom" => "Cyclomatic",
        _ => "Total Indent",
    }
}

fn method_description(metric: &str) -> &'static str {
    match metric {
        "cycom" => "Score = Commits × Cyclomatic Complexity.",
        _ => "Score = Commits × Total Indentation (Thornhill method).",
    }
}

pub fn print_report(files: &[FileHotspot], metric: &str) {
    if files.is_empty() {
        println!("No hotspots found.");
        return;
    }

    let label = complexity_label(metric);

    let max_path_len = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);
    // 1 (leading space) + path + 2 + 10 + 1 + 7 + 1 + 12 + 1 + 10 = path + 45
    let header_width = max_path_len + 45;
    let separator = report_helpers::separator(header_width.max(78));

    println!("Hotspots (Commits × {label} Complexity)");
    println!("{separator}");
    println!(
        " {:<width$}  {:>10} {:>7} {:>12} {:>10}",
        "File",
        "Language",
        "Commits",
        label,
        "Score",
        width = max_path_len
    );
    println!("{separator}");

    for f in files {
        println!(
            " {:<width$}  {:>10} {:>7} {:>12} {:>10}",
            f.path.display(),
            f.language,
            f.commits,
            f.complexity,
            f.score,
            width = max_path_len
        );
    }

    println!("{separator}");
    println!();
    println!("{}", method_description(metric));
    println!("High-score files are change-prone and complex — prime refactoring targets.");
}

#[derive(Serialize)]
struct JsonEntry {
    path: String,
    language: String,
    commits: usize,
    complexity: usize,
    complexity_metric: String,
    score: usize,
}

pub fn print_json(files: &[FileHotspot], metric: &str) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<JsonEntry> = files
        .iter()
        .map(|f| JsonEntry {
            path: f.path.display().to_string(),
            language: f.language.clone(),
            commits: f.commits,
            complexity: f.complexity,
            complexity_metric: metric.to_string(),
            score: f.score,
        })
        .collect();

    report_helpers::print_json_stdout(&entries)
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
