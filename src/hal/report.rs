use std::path::PathBuf;

use serde::Serialize;

use super::analyzer::HalsteadMetrics;
use crate::report_helpers;

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
mod tests {
    use super::*;

    fn sample_files() -> Vec<FileHalsteadMetrics> {
        vec![
            FileHalsteadMetrics {
                path: PathBuf::from("src/foo.rs"),
                language: "Rust".to_string(),
                metrics: HalsteadMetrics {
                    distinct_operators: 25,
                    distinct_operands: 42,
                    total_operators: 150,
                    total_operands: 230,
                    vocabulary: 67,
                    length: 380,
                    volume: 2298.5,
                    difficulty: 68.45,
                    effort: 157331.0,
                    bugs: 0.77,
                    time: 8740.6,
                },
            },
            FileHalsteadMetrics {
                path: PathBuf::from("src/bar.rs"),
                language: "Rust".to_string(),
                metrics: HalsteadMetrics {
                    distinct_operators: 10,
                    distinct_operands: 15,
                    total_operators: 30,
                    total_operands: 45,
                    vocabulary: 25,
                    length: 75,
                    volume: 348.4,
                    difficulty: 15.0,
                    effort: 5226.0,
                    bugs: 0.12,
                    time: 290.3,
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
        let mut buf = Vec::new();
        let entries: Vec<serde_json::Value> = files
            .iter()
            .map(|f| {
                serde_json::json!({
                    "path": f.path.display().to_string(),
                    "volume": f.metrics.volume,
                    "effort": f.metrics.effort,
                    "bugs": f.metrics.bugs,
                })
            })
            .collect();

        serde_json::to_writer_pretty(&mut buf, &entries).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&buf).unwrap();
        assert_eq!(parsed.len(), 2);
        assert!(parsed[0]["volume"].as_f64().unwrap() > 0.0);
    }
}
