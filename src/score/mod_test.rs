use super::*;
use crate::util::find_test_block_start;
use std::fs;

#[test]
fn weights_sum_to_one() {
    let total = W_MI + W_CYCOM + W_DUP + W_INDENT + W_HAL + W_SIZE;
    assert!(
        (total - 1.0).abs() < 1e-10,
        "dimension weights must sum to 1.0, got {total}"
    );
}

#[test]
fn file_weights_match_constants() {
    // Ensure FILE_WEIGHTS stays in sync with the individual constants
    let file_sum: f64 = FILE_WEIGHTS.iter().map(|(w, _)| w).sum();
    let expected = W_MI + W_CYCOM + W_INDENT + W_HAL + W_SIZE;
    assert!(
        (file_sum - expected).abs() < 1e-10,
        "FILE_WEIGHTS sum should match non-dup constants"
    );
}

#[test]
fn weighted_mean_all_none() {
    let files = vec![FileMetrics {
        path: "a.rs".into(),
        code_lines: 100,

        mi_score: None,
        max_complexity: None,
        indent_stddev: None,
        halstead_effort: None,
    }];
    let result = weighted_mean(&files, 100, |_| None);
    assert!((result - 0.0).abs() < 0.01, "all None → 0, got {result}");
}

#[test]
fn weighted_mean_total_loc_zero() {
    let files: Vec<FileMetrics> = vec![];
    let result = weighted_mean(&files, 0, |_| Some(80.0));
    assert!((result - 0.0).abs() < 0.01, "total_loc=0 → 0, got {result}");
}

#[test]
fn weighted_mean_single_file() {
    let files = vec![FileMetrics {
        path: "a.rs".into(),
        code_lines: 100,

        mi_score: Some(85.0),
        max_complexity: Some(5),
        indent_stddev: Some(1.0),
        halstead_effort: Some(1000.0),
    }];
    let result = weighted_mean(&files, 100, |f| f.mi_score);
    assert!(
        (result - 85.0).abs() < 0.01,
        "single file → same value, got {result}"
    );
}

#[test]
fn weighted_mean_loc_weighted() {
    let files = vec![
        FileMetrics {
            path: "small.rs".into(),
            code_lines: 10,

            mi_score: Some(100.0),
            max_complexity: None,
            indent_stddev: None,
            halstead_effort: None,
        },
        FileMetrics {
            path: "big.rs".into(),
            code_lines: 90,

            mi_score: Some(50.0),
            max_complexity: None,
            indent_stddev: None,
            halstead_effort: None,
        },
    ];
    let result = weighted_mean(&files, 100, |f| f.mi_score);
    // (100*10 + 50*90) / 100 = (1000 + 4500) / 100 = 55
    assert!(
        (result - 55.0).abs() < 0.01,
        "LOC-weighted → 55, got {result}"
    );
}

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    run(dir.path(), false, false, 10, 6).unwrap();
}

#[test]
fn run_on_rust_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "// a module\nfn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
    )
    .unwrap();
    let score = compute_score(dir.path(), false, 10, 6).unwrap();
    assert!(
        score.score > 50.0,
        "simple code should score well, got {}",
        score.score
    );
    assert_eq!(score.files_analyzed, 1);
    assert!(score.total_loc > 0);
    assert_eq!(score.dimensions.len(), 6);
}

#[test]
fn run_json_output() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    run(dir.path(), true, false, 10, 6).unwrap();
}

#[test]
fn run_includes_tests_with_flag() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("tests")).unwrap();
    fs::write(
        dir.path().join("tests/integration.rs"),
        "fn test() {\n    assert!(true);\n}\n",
    )
    .unwrap();
    let score = compute_score(dir.path(), true, 10, 6).unwrap();
    assert_eq!(score.files_analyzed, 1);
}

#[test]
fn run_excludes_tests_by_default() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("tests")).unwrap();
    fs::write(
        dir.path().join("tests/integration.rs"),
        "fn test() {\n    assert!(true);\n}\n",
    )
    .unwrap();
    let score = compute_score(dir.path(), false, 10, 6).unwrap();
    assert_eq!(score.files_analyzed, 0);
}

#[test]
fn run_on_current_repo() {
    // Smoke test on the actual repo
    run(Path::new("."), false, false, 5, 6).unwrap();
}

#[test]
fn find_test_block_start_finds_cfg_test() {
    let lines = vec![
        "fn foo() {}".to_string(),
        "#[cfg(test)]".to_string(),
        "mod tests {}".to_string(),
    ];
    assert_eq!(find_test_block_start(&lines), 1);
}

#[test]
fn find_test_block_start_with_leading_spaces() {
    let lines = vec![
        "fn foo() {}".to_string(),
        "  #[cfg(test)]  ".to_string(),
        "mod tests {}".to_string(),
    ];
    assert_eq!(find_test_block_start(&lines), 1);
}

#[test]
fn find_test_block_start_no_match() {
    let lines = vec!["fn foo() {}".to_string(), "fn bar() {}".to_string()];
    assert_eq!(find_test_block_start(&lines), 2);
}

#[test]
fn find_test_block_start_empty() {
    let lines: Vec<String> = vec![];
    assert_eq!(find_test_block_start(&lines), 0);
}

#[test]
fn excludes_markdown_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("README.md"), "# Hello\n\nWorld\n").unwrap();
    let score = compute_score(dir.path(), false, 10, 6).unwrap();
    assert_eq!(score.files_analyzed, 0, "Markdown should be excluded");
}

#[test]
fn excludes_toml_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\n",
    )
    .unwrap();
    let score = compute_score(dir.path(), false, 10, 6).unwrap();
    assert_eq!(score.files_analyzed, 0, "TOML should be excluded");
}

#[test]
fn excludes_json_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("data.json"), "{\"key\": \"value\"}\n").unwrap();
    let score = compute_score(dir.path(), false, 10, 6).unwrap();
    assert_eq!(score.files_analyzed, 0, "JSON should be excluded");
}

#[test]
fn run_on_single_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("lib.rs");
    fs::write(
        &file,
        "/// Docs\nfn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )
    .unwrap();
    let score = compute_score(&file, false, 10, 6).unwrap();
    assert_eq!(score.files_analyzed, 1);
    assert!(score.total_loc > 0);
}

#[test]
fn dimensions_sum_to_100_percent() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    let score = compute_score(dir.path(), false, 10, 6).unwrap();
    let total_weight: f64 = score.dimensions.iter().map(|d| d.weight).sum();
    assert!(
        (total_weight - 1.0).abs() < 0.001,
        "weights should sum to 1.0, got {total_weight}"
    );
}
