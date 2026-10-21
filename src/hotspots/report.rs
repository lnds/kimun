/// Report formatters for hotspot analysis.
///
/// Provides table and JSON output showing files ranked by their
/// hotspot score (commits x complexity), identifying prime refactoring targets.
use serde::Serialize;

use super::FileHotspot;
use crate::report_helpers;

/// Map the metric to a human-readable column header.
fn complexity_label(metric: &str) -> &'static str {
    match metric {
        "cycom" => "Cyclomatic",
        "cogcom" => "Cognitive",
        "lines" => "Lines",
        "mixed" => "Complexity",
        _ => "Total Indent",
    }
}

/// Return a description of the scoring formula for the metric.
fn method_description(metric: &str) -> &'static str {
    match metric {
        "cycom" => "Score = Commits × Cyclomatic Complexity.",
        "cogcom" => "Score = Commits × Cognitive Complexity.",
        "lines" => "Score = Commits × Lines of text (documents).",
        "mixed" => "Score = Commits × Complexity; lines of text for documents.",
        _ => "Score = Commits × Total Indentation (Tornhill method).",
    }
}

/// The metric of the files shown: theirs when they share one.
fn shared_metric(files: &[FileHotspot]) -> &'static str {
    let first = files.first().map_or("indent", |f| f.metric);
    match files.iter().all(|f| f.metric == first) {
        true => first,
        false => "mixed",
    }
}

/// Print a table of hotspots sorted by score descending with a
/// footer explaining the scoring method.
pub fn print_report(files: &[FileHotspot]) {
    if files.is_empty() {
        println!("No hotspots found.");
        return;
    }

    let metric = shared_metric(files);
    let label = complexity_label(metric);

    let max_path_len = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);
    // 1 (leading space) + path + 2 + 10 + 1 + 7 + 1 + 12 + 1 + 10 = path + 45
    let header_width = max_path_len + 45;
    let separator = report_helpers::separator(header_width.max(78));

    let title = match metric {
        "lines" | "mixed" => label.to_string(),
        _ => format!("{label} Complexity"),
    };
    println!("Hotspots (Commits × {title})");
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
    match metric {
        "lines" => {
            println!("High-score documents are long and rewritten often — candidates to split.")
        }
        _ => println!("High-score files are change-prone and complex — prime refactoring targets."),
    }
}

/// JSON-serializable representation of a single hotspot entry.
#[derive(Serialize)]
struct JsonEntry {
    path: String,
    language: String,
    commits: usize,
    complexity: usize,
    complexity_metric: String,
    score: usize,
}

/// Serialize hotspot data as pretty-printed JSON to stdout.
pub fn print_json(files: &[FileHotspot]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<JsonEntry> = files
        .iter()
        .map(|f| JsonEntry {
            path: f.path.display().to_string(),
            language: f.language.clone(),
            commits: f.commits,
            complexity: f.complexity,
            complexity_metric: f.metric.to_string(),
            score: f.score,
        })
        .collect();

    report_helpers::print_json_stdout(&entries)
}

/// Print hotspots as a single compact line.
pub fn print_short(files: &[FileHotspot]) {
    if files.is_empty() {
        println!("hotspots top:- score:0 commits:0 cx:0");
        return;
    }
    let top = &files[0];
    println!(
        "hotspots top:{} score:{} commits:{} cx:{}",
        top.path.display(),
        top.score,
        top.commits,
        top.complexity
    );
}

/// Print only the top hotspot file path.
pub fn print_terse(files: &[FileHotspot]) {
    if files.is_empty() {
        println!("-");
    } else {
        println!("{}", files[0].path.display());
    }
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
