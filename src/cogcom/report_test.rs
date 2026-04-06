use super::*;

fn make_metrics(path: &str, max: usize, total: usize) -> FileCogcomMetrics {
    FileCogcomMetrics {
        path: path.into(),
        language: "Rust".to_string(),
        function_count: 1,
        avg_complexity: total as f64,
        max_complexity: max,
        total_complexity: total,
        level: CognitiveLevel::from_complexity(max),
        functions: vec![FunctionCognitive {
            name: "foo".to_string(),
            start_line: 1,
            complexity: total,
            level: CognitiveLevel::from_complexity(total),
        }],
    }
}

#[test]
fn print_report_empty() {
    print_report(&[]);
}

#[test]
fn print_report_with_files() {
    let files = vec![make_metrics("src/main.rs", 5, 10)];
    print_report(&files);
}

#[test]
fn print_per_function_empty() {
    print_per_function(&[]);
}

#[test]
fn print_per_function_with_files() {
    let files = vec![make_metrics("src/main.rs", 5, 10)];
    print_per_function(&files);
}

#[test]
fn print_json_with_files() {
    let files = vec![make_metrics("src/main.rs", 5, 10)];
    print_json(&files).unwrap();
}

// ── print_github ─────────────────────────────────────────────────────────────

#[test]
fn print_github_empty_files() {
    print_github(&[], 5);
}

#[test]
fn print_github_no_functions_above_threshold() {
    // complexity=2, threshold=5 → no annotations
    let files = vec![make_metrics("src/main.rs", 2, 2)];
    print_github(&files, 5);
}

#[test]
fn print_github_all_above_threshold() {
    // complexity=10, threshold=5 → annotation emitted
    let files = vec![make_metrics("src/main.rs", 10, 10)];
    print_github(&files, 5);
}

#[test]
fn print_github_threshold_boundary_exactly_equal() {
    // complexity == threshold → annotation emitted (>=)
    let files = vec![make_metrics("src/main.rs", 5, 5)];
    print_github(&files, 5);
}

#[test]
fn print_github_multiple_files() {
    let files = vec![
        make_metrics("src/a.rs", 8, 8),
        make_metrics("src/b.rs", 2, 2),
        make_metrics("src/c.rs", 12, 12),
    ];
    print_github(&files, 5);
}
