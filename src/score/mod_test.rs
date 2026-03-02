use super::*;
use crate::util::find_test_block_start;
use crate::walk::{ExcludeFilter, WalkConfig};
use std::fs;
use std::path::Path;

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, 10, 6, "cogcom").unwrap();
}

#[test]
fn run_on_rust_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "// a module\nfn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let model = ScoringModel::Cognitive;
    let score = compute_score(&cfg, 10, 6, &model).unwrap();
    assert!(
        score.score > 50.0,
        "simple code should score well, got {}",
        score.score
    );
    assert_eq!(score.files_analyzed, 1);
    assert!(score.total_loc > 0);
    assert_eq!(score.dimensions.len(), 5);
}

#[test]
fn run_on_rust_file_legacy_model() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "// a module\nfn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let model = ScoringModel::Legacy;
    let score = compute_score(&cfg, 10, 6, &model).unwrap();
    assert!(
        score.score > 50.0,
        "simple code should score well with legacy model, got {}",
        score.score
    );
    assert_eq!(score.files_analyzed, 1);
    assert!(score.total_loc > 0);
    assert_eq!(score.dimensions.len(), 6);
    assert_eq!(score.dimensions[0].name, "Maintainability Index");
    assert_eq!(score.dimensions[1].name, "Cyclomatic Complexity");
}

#[test]
fn run_json_output() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, true, 10, 6, "cogcom").unwrap();
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
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    let model = ScoringModel::Cognitive;
    let score = compute_score(&cfg, 10, 6, &model).unwrap();
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
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let model = ScoringModel::Cognitive;
    let score = compute_score(&cfg, 10, 6, &model).unwrap();
    assert_eq!(score.files_analyzed, 0);
}

#[test]
fn run_on_current_repo() {
    // Smoke test on the actual repo
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(Path::new("."), false, &filter);
    run(&cfg, false, 5, 6, "cogcom").unwrap();
}

#[test]
fn run_on_current_repo_legacy() {
    // Smoke test on the actual repo with legacy model
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(Path::new("."), false, &filter);
    run(&cfg, false, 5, 6, "legacy").unwrap();
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
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let model = ScoringModel::Cognitive;
    let score = compute_score(&cfg, 10, 6, &model).unwrap();
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
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let model = ScoringModel::Cognitive;
    let score = compute_score(&cfg, 10, 6, &model).unwrap();
    assert_eq!(score.files_analyzed, 0, "TOML should be excluded");
}

#[test]
fn excludes_json_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("data.json"), "{\"key\": \"value\"}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let model = ScoringModel::Cognitive;
    let score = compute_score(&cfg, 10, 6, &model).unwrap();
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
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(&file, false, &filter);
    let model = ScoringModel::Cognitive;
    let score = compute_score(&cfg, 10, 6, &model).unwrap();
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
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let model = ScoringModel::Cognitive;
    let score = compute_score(&cfg, 10, 6, &model).unwrap();
    let total_weight: f64 = score.dimensions.iter().map(|d| d.weight).sum();
    assert!(
        (total_weight - 1.0).abs() < 0.001,
        "weights should sum to 1.0, got {total_weight}"
    );
}

#[test]
fn legacy_dimensions_sum_to_100_percent() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let model = ScoringModel::Legacy;
    let score = compute_score(&cfg, 10, 6, &model).unwrap();
    let total_weight: f64 = score.dimensions.iter().map(|d| d.weight).sum();
    assert!(
        (total_weight - 1.0).abs() < 0.001,
        "legacy weights should sum to 1.0, got {total_weight}"
    );
}
