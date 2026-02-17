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
