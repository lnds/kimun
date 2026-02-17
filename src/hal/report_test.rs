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
