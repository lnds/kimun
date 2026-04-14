use super::*;
use crate::cli::OutputMode;
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
    run(&cfg, OutputMode::Table, 10, 6, "cogcom").unwrap();
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
    run(&cfg, OutputMode::Json, 10, 6, "cogcom").unwrap();
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
    run(&cfg, OutputMode::Table, 5, 6, "cogcom").unwrap();
}

#[test]
fn run_on_current_repo_legacy() {
    // Smoke test on the actual repo with legacy model
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(Path::new("."), false, &filter);
    run(&cfg, OutputMode::Table, 5, 6, "legacy").unwrap();
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

fn make_git_repo_with_file(content: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    let sig =
        git2::Signature::new("Test", "test@test.com", &git2::Time::new(1_700_000_000, 0)).unwrap();
    fs::write(dir.path().join("main.rs"), content).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("main.rs")).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
        .unwrap();
    drop(tree);
    dir
}

#[test]
fn run_diff_on_git_repo() {
    let (dir, _repo) = create_test_repo_with_rust_file();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    let result = run_diff(
        &cfg,
        "HEAD",
        OutputMode::Table,
        5,
        6,
        "cogcom",
        ScoreGate::default(),
    );
    assert!(result.is_ok(), "run_diff should succeed: {:?}", result);
}

#[test]
fn run_diff_json_on_git_repo() {
    let (dir, _repo) = create_test_repo_with_rust_file();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    let result = run_diff(
        &cfg,
        "HEAD",
        OutputMode::Json,
        5,
        6,
        "cogcom",
        ScoreGate::default(),
    );
    assert!(result.is_ok(), "run_diff JSON should succeed: {:?}", result);
}

#[test]
fn run_diff_legacy_model_on_git_repo() {
    let (dir, _repo) = create_test_repo_with_rust_file();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    let result = run_diff(
        &cfg,
        "HEAD",
        OutputMode::Table,
        5,
        6,
        "legacy",
        ScoreGate::default(),
    );
    assert!(
        result.is_ok(),
        "run_diff legacy should succeed: {:?}",
        result
    );
}

#[test]
fn score_gate_no_flags_succeeds() {
    let dir = make_git_repo_with_file("fn main() {}\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let gate = ScoreGate::default();
    let result = run_diff(&cfg, "HEAD", OutputMode::Table, 10, 6, "cogcom", gate);
    assert!(result.is_ok(), "no gates should always succeed: {result:?}");
}

#[test]
fn score_gate_fail_below_passes_when_above_threshold() {
    let dir = make_git_repo_with_file("fn main() {}\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // A small clean file should score high — F-- threshold should never trigger
    let gate = ScoreGate {
        fail_if_worse: false,
        fail_below: Some(analyzer::Grade::FMinusMinus),
    };
    let result = run_diff(&cfg, "HEAD", OutputMode::Table, 10, 6, "cogcom", gate);
    assert!(
        result.is_ok(),
        "F-- threshold should not trigger on clean code"
    );
}

#[test]
fn score_gate_fail_below_unit() {
    // Unit test: verify gate logic directly using ScoreDiff without a full git run.
    let before = analyzer::score_to_grade(80.0); // B
    let after = analyzer::score_to_grade(70.0); // C
    let diff = diff::ScoreDiff {
        git_ref: "HEAD".to_string(),
        overall: diff::ScoreDelta {
            before: 80.0,
            after: 70.0,
            delta: -10.0,
        },
        before_grade: before,
        after_grade: after,
        files_before: 1,
        files_after: 1,
        loc_before: 10,
        loc_after: 10,
        dimensions: vec![],
    };
    // Gate: fail if below B — current score is C, so should fail
    let threshold = analyzer::Grade::B;
    assert!(
        diff.after_grade.numeric_rank() < threshold.numeric_rank(),
        "C should be below B"
    );
    // Gate: fail if below F-- — current score is C, should pass
    let low_threshold = analyzer::Grade::FMinusMinus;
    assert!(
        diff.after_grade.numeric_rank() >= low_threshold.numeric_rank(),
        "C should not be below F--"
    );
    // Gate: fail-if-worse compares numeric score, not grade
    assert!(diff.overall.delta < 0.0, "80→70 is a negative delta");
}

#[test]
fn score_gate_fail_if_worse_same_ref_passes() {
    let dir = make_git_repo_with_file("fn main() {}\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Comparing HEAD to itself — score cannot be worse
    let gate = ScoreGate {
        fail_if_worse: true,
        fail_below: None,
    };
    let result = run_diff(&cfg, "HEAD", OutputMode::Table, 10, 6, "cogcom", gate);
    assert!(
        result.is_ok(),
        "same ref comparison should not trigger fail-if-worse"
    );
}

#[test]
fn run_on_current_repo_with_target() {
    // Test with an explicit non-"." path to exercise the target display branch
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(Path::new("src"), false, &filter);
    run(&cfg, OutputMode::Table, 5, 6, "cogcom").unwrap();
}

#[test]
fn run_json_with_target_path() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(Path::new("src"), false, &filter);
    run(&cfg, OutputMode::Json, 5, 6, "cogcom").unwrap();
}

#[test]
fn run_short_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() { let x = 1; }\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Short, 10, 6, "cogcom").unwrap();
}

#[test]
fn run_terse_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() { let x = 1; }\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Terse, 10, 6, "cogcom").unwrap();
}

#[test]
fn run_diff_short_format() {
    let dir = make_git_repo_with_file("fn main() { let x = 1; }\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let gate = ScoreGate {
        fail_if_worse: false,
        fail_below: None,
    };
    run_diff(&cfg, "HEAD", OutputMode::Short, 10, 6, "cogcom", gate).unwrap();
}

#[test]
fn run_diff_terse_format() {
    let dir = make_git_repo_with_file("fn main() { let x = 1; }\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let gate = ScoreGate {
        fail_if_worse: false,
        fail_below: None,
    };
    run_diff(&cfg, "HEAD", OutputMode::Terse, 10, 6, "cogcom", gate).unwrap();
}
