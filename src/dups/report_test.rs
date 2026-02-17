use super::*;
use crate::dups::detector::DuplicateLocation;
use std::path::PathBuf;

fn sample_metrics() -> DuplicationMetrics {
    DuplicationMetrics {
        total_code_lines: 1000,
        duplicated_lines: 48,
        duplicate_groups: 2,
        files_with_duplicates: 3,
        largest_block: 12,
    }
}

fn sample_groups() -> Vec<DuplicateGroup> {
    vec![
        DuplicateGroup {
            locations: vec![
                DuplicateLocation {
                    file_path: PathBuf::from("src/a.rs"),
                    start_line: 1,
                    end_line: 6,
                },
                DuplicateLocation {
                    file_path: PathBuf::from("src/b.rs"),
                    start_line: 5,
                    end_line: 10,
                },
                DuplicateLocation {
                    file_path: PathBuf::from("src/c.rs"),
                    start_line: 20,
                    end_line: 25,
                },
            ],
            line_count: 6,
            sample: vec!["use std::io;".to_string(), "use std::fs;".to_string()],
            severity: DuplicationSeverity::Critical,
        },
        DuplicateGroup {
            locations: vec![
                DuplicateLocation {
                    file_path: PathBuf::from("src/foo.rs"),
                    start_line: 10,
                    end_line: 21,
                },
                DuplicateLocation {
                    file_path: PathBuf::from("src/bar.rs"),
                    start_line: 30,
                    end_line: 41,
                },
            ],
            line_count: 12,
            sample: vec![
                "fn process() {".to_string(),
                "let x = read();".to_string(),
                "transform(x);".to_string(),
            ],
            severity: DuplicationSeverity::Tolerable,
        },
    ]
}

#[test]
fn percentage_zero_lines() {
    let m = DuplicationMetrics {
        total_code_lines: 0,
        duplicated_lines: 0,
        duplicate_groups: 0,
        files_with_duplicates: 0,
        largest_block: 0,
    };
    assert_eq!(m.percentage(), 0.0);
}

#[test]
fn percentage_calculation() {
    let m = sample_metrics();
    assert!((m.percentage() - 4.8).abs() < 0.01);
}

#[test]
fn assessment_labels() {
    assert_eq!(assessment(0.0), "Excellent");
    assert_eq!(assessment(2.9), "Excellent");
    assert_eq!(assessment(3.0), "Good");
    assert_eq!(assessment(4.9), "Good");
    assert_eq!(assessment(5.0), "Moderate");
    assert_eq!(assessment(9.9), "Moderate");
    assert_eq!(assessment(10.0), "High");
    assert_eq!(assessment(19.9), "High");
    assert_eq!(assessment(20.0), "Very High");
    assert_eq!(assessment(50.0), "Very High");
}

#[test]
fn print_summary_does_not_panic() {
    print_summary(&sample_metrics(), &sample_groups());
}

#[test]
fn print_summary_zero_metrics() {
    let m = DuplicationMetrics {
        total_code_lines: 0,
        duplicated_lines: 0,
        duplicate_groups: 0,
        files_with_duplicates: 0,
        largest_block: 0,
    };
    print_summary(&m, &[]);
}

#[test]
fn print_detailed_does_not_panic() {
    let groups = sample_groups();
    let limit = display_limit(groups.len(), false);
    print_detailed(&sample_metrics(), &groups[..limit], groups.len());
}

#[test]
fn print_detailed_show_all() {
    let groups = sample_groups();
    let limit = display_limit(groups.len(), true);
    print_detailed(&sample_metrics(), &groups[..limit], groups.len());
}

#[test]
fn print_detailed_empty_groups() {
    let m = DuplicationMetrics {
        total_code_lines: 100,
        duplicated_lines: 0,
        duplicate_groups: 0,
        files_with_duplicates: 0,
        largest_block: 0,
    };
    print_detailed(&m, &[], 0);
}

#[test]
fn print_json_with_groups() {
    print_json(&sample_metrics(), &sample_groups()).unwrap();
}

#[test]
fn format_json_validates_structure() {
    let json_str = format_json(&sample_metrics(), &sample_groups()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    let metrics = &parsed["metrics"];
    assert_eq!(metrics["total_code_lines"], 1000);
    assert_eq!(metrics["duplicated_lines"], 48);
    assert_eq!(metrics["duplicate_groups"], 2);
    assert_eq!(metrics["files_with_duplicates"], 3);
    assert_eq!(metrics["largest_block"], 12);
    assert_eq!(metrics["assessment"], "Good");
    assert!((metrics["duplication_percentage"].as_f64().unwrap() - 4.8).abs() < 0.01);

    let groups = parsed["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0]["line_count"], 6);
    assert_eq!(groups[0]["locations"].as_array().unwrap().len(), 3);
    assert_eq!(groups[0]["severity"], "Critical");
    assert_eq!(groups[1]["severity"], "Tolerable");
    assert!(groups[0]["sample"].as_array().unwrap().len() > 0);
}

#[test]
fn format_json_empty_groups() {
    let m = DuplicationMetrics {
        total_code_lines: 100,
        duplicated_lines: 0,
        duplicate_groups: 0,
        files_with_duplicates: 0,
        largest_block: 0,
    };
    let json_str = format_json(&m, &[]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(parsed["metrics"]["total_code_lines"], 100);
    assert_eq!(parsed["metrics"]["duplicated_lines"], 0);
    assert_eq!(parsed["metrics"]["assessment"], "Excellent");
    assert_eq!(parsed["groups"].as_array().unwrap().len(), 0);
}

#[test]
fn format_json_respects_group_slice() {
    let groups = sample_groups();
    // Pass only first group
    let json_str = format_json(&sample_metrics(), &groups[..1]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["groups"].as_array().unwrap().len(), 1);
}
