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
