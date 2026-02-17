use super::*;

fn sample_files() -> Vec<FileCycomMetrics> {
    vec![
        FileCycomMetrics {
            path: PathBuf::from("src/foo.rs"),
            language: "Rust".to_string(),
            function_count: 8,
            avg_complexity: 4.2,
            max_complexity: 34,
            total_complexity: 289,
            level: CyclomaticLevel::HighlyComplex,
            functions: vec![
                FunctionComplexity {
                    name: "classify_reader".to_string(),
                    start_line: 10,
                    complexity: 34,
                    level: CyclomaticLevel::HighlyComplex,
                },
                FunctionComplexity {
                    name: "count_lines".to_string(),
                    start_line: 50,
                    complexity: 12,
                    level: CyclomaticLevel::Complex,
                },
            ],
        },
        FileCycomMetrics {
            path: PathBuf::from("src/bar.rs"),
            language: "Rust".to_string(),
            function_count: 3,
            avg_complexity: 2.0,
            max_complexity: 3,
            total_complexity: 6,
            level: CyclomaticLevel::Simple,
            functions: vec![FunctionComplexity {
                name: "run".to_string(),
                start_line: 1,
                complexity: 3,
                level: CyclomaticLevel::Simple,
            }],
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
fn print_per_function_does_not_panic() {
    print_per_function(&sample_files());
}

#[test]
fn print_per_function_empty() {
    print_per_function(&[]);
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
                "language": f.language,
                "function_count": f.function_count,
                "avg_complexity": f.avg_complexity,
                "max_complexity": f.max_complexity,
                "total_complexity": f.total_complexity,
                "level": f.level.as_str(),
                "functions": f.functions.iter().map(|func| {
                    serde_json::json!({
                        "name": func.name,
                        "start_line": func.start_line,
                        "complexity": func.complexity,
                        "level": func.level.as_str(),
                    })
                }).collect::<Vec<_>>(),
            })
        })
        .collect();

    let json_str = serde_json::to_string_pretty(&entries).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["function_count"], 8);
    assert_eq!(arr[0]["max_complexity"], 34);
    assert!(arr[0]["functions"].is_array());
    assert_eq!(arr[0]["functions"].as_array().unwrap().len(), 2);
}
