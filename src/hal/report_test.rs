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

// ── format_time ──────────────────────────────────────────────────────────

#[test]
fn format_time_seconds() {
    assert_eq!(format_time(45.0), "45s");
    assert_eq!(format_time(0.0), "0s");
    assert_eq!(format_time(59.9), "60s");
}

#[test]
fn format_time_minutes() {
    assert_eq!(format_time(60.0), "1m 0s");
    assert_eq!(format_time(90.0), "1m 30s");
    assert_eq!(format_time(3599.0), "59m 59s");
}

#[test]
fn format_time_hours() {
    assert_eq!(format_time(3600.0), "1h 0m");
    assert_eq!(format_time(7200.0), "2h 0m");
    assert_eq!(format_time(86399.0), "23h 60m");
}

#[test]
fn format_time_days() {
    let s = format_time(86400.0);
    assert!(s.contains('d'), "expected days format, got: {s}");
    assert_eq!(format_time(86400.0), "1d 0h");

    let s = format_time(172800.0); // 2 days
    assert_eq!(s, "2d 0h");

    let s = format_time(90000.0); // 1 day + 1 hour
    assert!(s.starts_with("1d"), "expected 1d format, got: {s}");
}

// ── print_report with large time ─────────────────────────────────────────

fn make_large_time_file() -> Vec<FileHalsteadMetrics> {
    vec![FileHalsteadMetrics {
        path: PathBuf::from("src/huge.rs"),
        language: "Rust".to_string(),
        metrics: HalsteadMetrics {
            distinct_operators: 100,
            distinct_operands: 200,
            total_operators: 1000,
            total_operands: 2000,
            vocabulary: 300,
            length: 3000,
            volume: 50000.0,
            difficulty: 500.0,
            effort: 25_000_000.0,
            bugs: 16.7,
            time: 172800.0, // 2 days — exercises the "days" branch
        },
    }]
}

#[test]
fn print_report_with_large_time_does_not_panic() {
    print_report(&make_large_time_file());
}

#[test]
fn print_json_with_large_time_does_not_panic() {
    print_json(&make_large_time_file()).unwrap();
}
