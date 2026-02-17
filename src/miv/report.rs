use std::path::PathBuf;

use serde::Serialize;

use super::analyzer::{MILevel, MIMetrics};
use crate::report_helpers;

pub struct FileMIMetrics {
    pub path: PathBuf,
    pub language: String,
    pub metrics: MIMetrics,
}

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
mod tests {
    use super::*;

    fn sample_files() -> Vec<FileMIMetrics> {
        vec![
            FileMIMetrics {
                path: PathBuf::from("src/foo.rs"),
                language: "Rust".to_string(),
                metrics: MIMetrics {
                    halstead_volume: 2298.5,
                    cyclomatic_complexity: 15,
                    loc: 120,
                    comment_lines: 20,
                    comment_percent: 14.3,
                    mi_woc: 68.2,
                    mi_cw: 8.5,
                    mi_score: 76.7,
                    level: MILevel::Moderate,
                },
            },
            FileMIMetrics {
                path: PathBuf::from("src/bar.rs"),
                language: "Rust".to_string(),
                metrics: MIMetrics {
                    halstead_volume: 348.4,
                    cyclomatic_complexity: 3,
                    loc: 25,
                    comment_lines: 5,
                    comment_percent: 16.7,
                    mi_woc: 100.3,
                    mi_cw: 9.1,
                    mi_score: 109.4,
                    level: MILevel::Good,
                },
            },
        ]
    }

    #[test]
    fn print_report_does_not_panic() {
        print_report(&sample_files());
    }

    #[test]
    fn print_report_empty() {
        print_report(&[]);
    }

    #[test]
    fn print_json_does_not_panic() {
        print_json(&sample_files()).unwrap();
    }

    #[test]
    fn print_json_empty() {
        print_json(&[]).unwrap();
    }

    #[test]
    fn json_structure_is_valid() {
        let files = sample_files();
        let entries: Vec<serde_json::Value> = files
            .iter()
            .map(|f| {
                serde_json::json!({
                    "path": f.path.display().to_string(),
                    "mi_score": f.metrics.mi_score,
                    "level": f.metrics.level.as_str(),
                })
            })
            .collect();

        let json_str = serde_json::to_string_pretty(&entries).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert!(arr[0]["mi_score"].as_f64().unwrap() > 0.0);
    }
}
