use super::*;
use crate::util::find_test_block_start;
use crate::walk::{ExcludeFilter, WalkConfig};
use git2::Repository;
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

// ── ScoringModel::from_arg ──────────────────────────────────────────────

#[test]
fn scoring_model_from_arg_cogcom() {
    assert_eq!(ScoringModel::from_arg("cogcom"), ScoringModel::Cognitive);
}

#[test]
fn scoring_model_from_arg_legacy() {
    assert_eq!(ScoringModel::from_arg("legacy"), ScoringModel::Legacy);
}

// ── run helpers ─────────────────────────────────────────────────────────

fn create_test_repo_with_rust_file() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config.set_str("user.email", "test@test.com").unwrap();

    // Write a Rust file and commit it.
    let rs_path = dir.path().join("main.rs");
    fs::write(&rs_path, "fn main() {\n    let x = 1;\n}\n").unwrap();

    let sig =
        git2::Signature::new("Test", "test@test.com", &git2::Time::new(1_700_000_000, 0)).unwrap();
    {
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("main.rs")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
    }

    (dir, repo)
}

#[test]
fn run_diff_on_git_repo() {
    let (dir, _repo) = create_test_repo_with_rust_file();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    // "HEAD" is a valid ref that points to the current commit
    let result = run_diff(&cfg, "HEAD", false, 5, 6, "cogcom");
    assert!(result.is_ok(), "run_diff should succeed: {:?}", result);
}

#[test]
fn run_diff_json_on_git_repo() {
    let (dir, _repo) = create_test_repo_with_rust_file();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    let result = run_diff(&cfg, "HEAD", true, 5, 6, "cogcom");
    assert!(result.is_ok(), "run_diff JSON should succeed: {:?}", result);
}

#[test]
fn run_diff_legacy_model_on_git_repo() {
    let (dir, _repo) = create_test_repo_with_rust_file();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    let result = run_diff(&cfg, "HEAD", false, 5, 6, "legacy");
    assert!(
        result.is_ok(),
        "run_diff legacy should succeed: {:?}",
        result
    );
}

#[test]
fn run_on_current_repo_with_target() {
    // Test with an explicit non-"." path to exercise the target display branch
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(Path::new("src"), false, &filter);
    run(&cfg, false, 5, 6, "cogcom").unwrap();
}

#[test]
fn run_json_with_target_path() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(Path::new("src"), false, &filter);
    run(&cfg, true, 5, 6, "cogcom").unwrap();
}
