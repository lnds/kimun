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
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample_files() -> Vec<FileHotspot> {
        vec![
            FileHotspot {
                path: PathBuf::from("src/foo.rs"),
                language: "Rust".to_string(),
                commits: 42,
                complexity: 34,
                score: 42 * 34,
            },
            FileHotspot {
                path: PathBuf::from("src/bar.rs"),
                language: "Rust".to_string(),
                commits: 10,
                complexity: 3,
                score: 10 * 3,
            },
        ]
    }

    #[test]
    fn print_report_does_not_panic_indent() {
        print_report(&sample_files(), "indent");
    }

    #[test]
    fn print_report_does_not_panic_cycom() {
        print_report(&sample_files(), "cycom");
    }

    #[test]
    fn print_report_empty() {
        print_report(&[], "indent");
    }

    #[test]
    fn print_json_does_not_panic() {
        print_json(&sample_files(), "indent").unwrap();
    }

    #[test]
    fn print_json_empty() {
        print_json(&[], "indent").unwrap();
    }

    #[test]
    fn json_structure_is_valid() {
        let files = sample_files();
        let entries: Vec<serde_json::Value> = files
            .iter()
            .map(|f| {
                serde_json::json!({
                    "path": f.path.display().to_string(),
                    "language": f.language,
                    "commits": f.commits,
                    "complexity": f.complexity,
                    "complexity_metric": "indent",
                    "score": f.score,
                })
            })
            .collect();

        let json_str = serde_json::to_string_pretty(&entries).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 2, "should have 2 entries");
        assert_eq!(arr[0]["commits"], 42, "first entry should have 42 commits");
        assert_eq!(
            arr[0]["complexity"], 34,
            "first entry complexity should be 34"
        );
        assert_eq!(
            arr[0]["score"],
            42 * 34,
            "score should be commits * complexity"
        );
        assert_eq!(
            arr[0]["complexity_metric"], "indent",
            "metric should be indent"
        );
        assert!(
            arr[1]["path"].as_str().unwrap().contains("bar.rs"),
            "second entry should be bar.rs"
        );
    }
}
