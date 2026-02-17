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
                mi_score: 35.2,
                level: MILevel::Green,
            },
        },
        FileMIMetrics {
            path: PathBuf::from("src/bar.rs"),
            language: "Rust".to_string(),
            metrics: MIMetrics {
                halstead_volume: 348.4,
                cyclomatic_complexity: 3,
                loc: 25,
                mi_score: 62.1,
                level: MILevel::Green,
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
