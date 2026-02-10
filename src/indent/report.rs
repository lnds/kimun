use std::path::PathBuf;

use serde::Serialize;

use super::analyzer::ComplexityLevel;

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

    let max_path_len = files
        .iter()
        .map(|f| f.path.display().to_string().len())
        .max()
        .unwrap_or(4)
        .max(4);

    let header_width = max_path_len + 42; // path + numbers + complexity
    let separator = "─".repeat(header_width.max(68));

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

    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_files() -> Vec<FileIndentMetrics> {
        vec![
            FileIndentMetrics {
                path: PathBuf::from("src/foo.rs"),
                code_lines: 100,
                stddev: 1.8,
                max_depth: 6,
                total_indent: 180,
                complexity: ComplexityLevel::High,
            },
            FileIndentMetrics {
                path: PathBuf::from("src/bar.rs"),
                code_lines: 50,
                stddev: 1.2,
                max_depth: 3,
                total_indent: 60,
                complexity: ComplexityLevel::Moderate,
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
        let json_str = serde_json::to_string_pretty(&entries).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["code_lines"], 100);
        assert_eq!(arr[0]["indent_stddev"], 1.8);
        assert_eq!(arr[0]["indent_max"], 6);
        assert_eq!(arr[0]["complexity"], "high");
        assert_eq!(arr[1]["complexity"], "moderate");
    }
}
