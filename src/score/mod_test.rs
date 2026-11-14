use super::*;
use crate::cli::OutputMode;
use crate::walk::{ExcludeFilter, WalkConfig};
use git2::Repository;
use std::fs;
use std::path::Path;

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Table, 10, 6, "cogcom", None).unwrap();
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
    let value = score.graded.unwrap().score;
    assert!(value > 50.0, "simple code should score well, got {value}");
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
    let value = score.graded.unwrap().score;
    assert!(
        value > 50.0,
        "simple code should score well with legacy model, got {value}"
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
    run(&cfg, OutputMode::Json, 10, 6, "cogcom", None).unwrap();
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
    run(&cfg, OutputMode::Table, 5, 6, "cogcom", None).unwrap();
}

#[test]
fn run_on_current_repo_legacy() {
    // Smoke test on the actual repo with legacy model
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(Path::new("."), false, &filter);
    run(&cfg, OutputMode::Table, 5, 6, "legacy", None).unwrap();
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
    assert_eq!(score.graded, None, "nothing to grade, no grade");
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
    assert_eq!(score.graded, None, "nothing to grade, no grade");
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
    assert_eq!(score.graded, None, "nothing to grade, no grade");
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

// ── ScoreGate error message format ──────────────────────────────────────

#[test]
fn score_gate_error_message_shows_scores_delta_and_tolerance() {
    let before = 86.93_f64;
    let after = 86.89_f64;
    let msg = format_gate_error(before, after, after - before, 0.01);
    assert!(
        msg.contains("86.93 → 86.89"),
        "scores should show two decimal places, got: {msg}"
    );
    assert!(
        msg.contains("(-0.0400)"),
        "delta should show four decimal places, got: {msg}"
    );
    assert!(
        msg.contains("0.01 tolerance"),
        "message should name the tolerance, got: {msg}"
    );
}

#[test]
fn score_gate_error_message_separates_drop_just_over_tolerance() {
    let msg = format_gate_error(77.7800, 77.7696, 77.7696 - 77.7800, 0.01);
    assert!(
        msg.contains("(-0.0104)"),
        "a drop just over the tolerance must not print as -0.01, got: {msg}"
    );
}

#[test]
fn gate_score_worsened_ignores_drop_within_tolerance() {
    assert!(!gate_score_worsened(87.814, 87.811, 0.01));
    assert!(!gate_score_worsened(90.0, 90.0, 0.01));
}

#[test]
fn gate_score_worsened_ignores_sub_tolerance_drop_across_rounding_boundary() {
    // -0.0078, but rounds to 77.78 → 77.77.
    assert!(!gate_score_worsened(77.77897, 77.77122, 0.01));
}

#[test]
fn gate_score_worsened_catches_drop_over_tolerance() {
    assert!(gate_score_worsened(87.81, 87.79, 0.01));
    assert!(gate_score_worsened(90.0, 89.5, 0.01));
}

#[test]
fn gate_score_worsened_honors_custom_tolerance() {
    assert!(!gate_score_worsened(90.0, 89.96, 0.05));
    assert!(gate_score_worsened(90.0, 89.9, 0.05));
}

#[test]
fn gate_score_worsened_zero_tolerance_catches_any_drop() {
    assert!(gate_score_worsened(90.0, 89.999, 0.0));
    assert!(!gate_score_worsened(90.0, 90.0, 0.0));
}

#[test]
fn gate_score_worsened_allows_improvement() {
    assert!(!gate_score_worsened(87.80, 87.81, 0.01));
    assert!(!gate_score_worsened(89.5, 90.0, 0.0));
}

#[test]
fn score_gate_fail_if_worse_error_contains_decimal_delta() {
    // Integration: run_diff with fail_if_worse on a repo whose second commit
    // introduces a function with 10 levels of nesting (cognitive complexity ≈ 55,
    // normalizes to ~2/100) against a trivial baseline (~90/100). The ~40-point
    // drop guarantees the gate fires regardless of normalization variance.
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    let sig =
        git2::Signature::new("Test", "test@test.com", &git2::Time::new(1_700_000_000, 0)).unwrap();

    // First commit: minimal clean file → high baseline score (~90).
    let simple = "fn clean() -> i32 { 42 }\n";
    std::fs::write(dir.path().join("lib.rs"), simple).unwrap();
    let oid1 = {
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("lib.rs")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap()
    };

    // Second commit: 10 levels of nesting hits cognitive complexity (~55),
    // indentation stddev, and Halstead simultaneously — guaranteed large drop.
    let complex = r#"fn clean() -> i32 { 42 }
fn deeply_nested(x: i32) -> i32 {
    if x > 0 {
        if x > 1 {
            if x > 2 {
                if x > 3 {
                    if x > 4 {
                        if x > 5 {
                            if x > 6 {
                                if x > 7 {
                                    if x > 8 {
                                        if x > 9 {
                                            return x;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    0
}
"#;
    std::fs::write(dir.path().join("lib.rs"), complex).unwrap();
    {
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("lib.rs")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repo.find_commit(oid1).unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "add deeply nested fn",
            &tree,
            &[&parent],
        )
        .unwrap();
    }

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    let gate = ScoreGate {
        max_drop: Some(0.01),
        fail_below: None,
        ..ScoreGate::default()
    };
    let result = run_diff(&cfg, "HEAD~1", OutputMode::Table, 10, 6, "cogcom", gate);

    let msg = result
        .expect_err(
            "gate should fire: 10-level nesting must produce a score drop vs trivial baseline",
        )
        .to_string();

    let paren_start = msg.rfind('(').expect("message should contain '('");
    let paren_end = msg.rfind(')').expect("message should contain ')'");
    let delta_str = &msg[paren_start + 1..paren_end];
    let dot_idx = delta_str.find('.').unwrap_or_else(|| {
        panic!("delta '{delta_str}' should contain a decimal point, full msg: {msg}")
    });
    let decimals = &delta_str[dot_idx + 1..];
    assert_eq!(
        decimals.len(),
        4,
        "delta should have exactly 4 decimal places, got '{delta_str}' in: {msg}"
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

#[test]
fn scoring_model_from_arg_cogcom_0_35() {
    assert_eq!(
        ScoringModel::from_arg("cogcom-0.35"),
        ScoringModel::Cognitive035
    );
}

#[test]
fn the_weights_up_to_v0_35_give_the_former_score() {
    let files = vec![FileMetrics {
        path: "a.rs".into(),
        code_lines: 100,
        max_cognitive: Some(20),
        mi_score: None,
        mi_vs: None,
        max_complexity: None,
        indent_stddev: Some(1.0),
        halstead_effort: Some(1000.0),
    }];
    let score_with = |model| {
        let dims = build_dimensions(&files, 100, 0.0, &model);
        compute_project_score(&dims).unwrap()
    };
    let dims = build_dimensions(&files, 100, 0.0, &ScoringModel::Cognitive035);
    let d = |i: usize| dims[i].graded.unwrap().score;
    let former = 0.30 * d(0) + 0.20 * d(1) + 0.15 * d(2) + 0.20 * d(3) + 0.15 * d(4);
    assert!((score_with(ScoringModel::Cognitive035) - former).abs() < 1e-9);
    assert!(score_with(ScoringModel::Cognitive) < former);
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
    make_git_repo_with_files(&[("main.rs", content)])
}

fn make_git_repo_with_files(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    let sig =
        git2::Signature::new("Test", "test@test.com", &git2::Time::new(1_700_000_000, 0)).unwrap();
    let mut index = repo.index().unwrap();
    for (name, content) in files {
        fs::write(dir.path().join(name), content).unwrap();
        index.add_path(Path::new(name)).unwrap();
    }
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
        max_drop: None,
        fail_below: Some(analyzer::Grade::FMinusMinus),
        ..ScoreGate::default()
    };
    let result = run_diff(&cfg, "HEAD", OutputMode::Table, 10, 6, "cogcom", gate);
    assert!(
        result.is_ok(),
        "F-- threshold should not trigger on clean code"
    );
}

/// A diff between two scores, `None` for nothing to grade, through
/// `compute_diff` as `run_diff` builds it.
fn diff_of(before: Option<f64>, after: Option<f64>) -> diff::ScoreDiff {
    let project = |score: Option<f64>| analyzer::ProjectScore {
        graded: score.map(analyzer::Graded::of),
        files_analyzed: usize::from(score.is_some()),
        total_loc: if score.is_some() { 10 } else { 0 },
        dimensions: vec![],
        needs_attention: vec![],
    };
    diff::compute_diff("HEAD~1", &project(before), &project(after))
}

fn gate(max_drop: Option<f64>, fail_below: Option<analyzer::Grade>) -> ScoreGate {
    ScoreGate {
        max_drop,
        fail_below,
        ..ScoreGate::default()
    }
}

#[test]
fn score_gate_fail_below_unit() {
    let b_to_c = diff_of(Some(80.0), Some(70.0));
    assert_eq!(
        gate_failure(&b_to_c, gate(None, Some(analyzer::Grade::B))).as_deref(),
        Some("quality gate failed: score C is below minimum threshold B")
    );
    assert_eq!(
        gate_failure(&b_to_c, gate(None, Some(analyzer::Grade::FMinusMinus))),
        None
    );
    // fail-if-worse compares the scores, not the grades.
    assert_eq!(
        gate_failure(&b_to_c, gate(Some(0.01), None)).as_deref(),
        Some(
            "quality gate failed: score dropped 80.00 → 70.00 (-10.0000), more than the 0.01 tolerance"
        )
    );
}

#[test]
fn no_gate_fails_when_there_is_nothing_to_grade() {
    let strict = gate(Some(0.01), Some(analyzer::Grade::APlusPlus));
    assert_eq!(gate_failure(&diff_of(None, None), strict), None);
    // All the code deleted: nothing left to compare or to hold to a grade.
    assert_eq!(gate_failure(&diff_of(Some(90.0), None), strict), None);
}

#[test]
fn the_first_code_is_held_to_fail_below_only() {
    let first_code = diff_of(None, Some(70.0));
    assert_eq!(gate_failure(&first_code, gate(Some(0.01), None)), None);
    assert_eq!(
        gate_failure(&first_code, gate(Some(0.01), Some(analyzer::Grade::B))).as_deref(),
        Some("quality gate failed: score C is below minimum threshold B")
    );
}

#[test]
fn skipped_gates_names_the_target_and_the_flags() {
    let strict = gate(Some(0.01), Some(analyzer::Grade::B));
    assert_eq!(
        skipped_gates(&diff_of(Some(90.0), None), strict, "README.md").as_deref(),
        Some(
            "note: nothing to grade in README.md (no code files measured); --fail-if-worse and --fail-below skipped"
        )
    );
    assert_eq!(
        skipped_gates(&diff_of(None, Some(90.0)), strict, "src").as_deref(),
        Some(
            "note: nothing to grade in src at HEAD~1 (no code files measured); --fail-if-worse skipped"
        )
    );
    assert_eq!(
        skipped_gates(
            &diff_of(None, None),
            gate(None, Some(analyzer::Grade::B)),
            "docs"
        )
        .as_deref(),
        Some("note: nothing to grade in docs (no code files measured); --fail-below skipped")
    );
    assert_eq!(
        skipped_gates(&diff_of(Some(80.0), Some(70.0)), strict, "src"),
        None
    );
    assert_eq!(
        skipped_gates(&diff_of(None, None), gate(None, None), "docs"),
        None
    );
}

#[test]
fn score_gate_fail_if_worse_same_ref_passes() {
    let dir = make_git_repo_with_file("fn main() {}\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Comparing HEAD to itself — score cannot be worse
    let gate = ScoreGate {
        max_drop: Some(0.01),
        fail_below: None,
        ..ScoreGate::default()
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
    run(&cfg, OutputMode::Table, 5, 6, "cogcom", None).unwrap();
}

#[test]
fn run_json_with_target_path() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(Path::new("src"), false, &filter);
    run(&cfg, OutputMode::Json, 5, 6, "cogcom", None).unwrap();
}

#[test]
fn run_short_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() { let x = 1; }\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Short, 10, 6, "cogcom", None).unwrap();
}

#[test]
fn run_terse_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() { let x = 1; }\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Terse, 10, 6, "cogcom", None).unwrap();
}

#[test]
fn run_diff_short_format() {
    let dir = make_git_repo_with_file("fn main() { let x = 1; }\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let gate = ScoreGate {
        max_drop: None,
        fail_below: None,
        ..ScoreGate::default()
    };
    run_diff(&cfg, "HEAD", OutputMode::Short, 10, 6, "cogcom", gate).unwrap();
}

#[test]
fn run_diff_terse_format() {
    let dir = make_git_repo_with_file("fn main() { let x = 1; }\n");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let gate = ScoreGate {
        max_drop: None,
        fail_below: None,
        ..ScoreGate::default()
    };
    run_diff(&cfg, "HEAD", OutputMode::Terse, 10, 6, "cogcom", gate).unwrap();
}

// ── --gate-scope changed ────────────────────────────────────────────────

const SIMPLE_FNS: &str = "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\n\
fn sub(a: i32, b: i32) -> i32 {\n    a - b\n}\n\n\
fn mul(a: i32, b: i32) -> i32 {\n    a * b\n}\n";

const NESTED_FN: &str = "fn tangle(a: i32, b: i32, c: i32) -> i32 {
    let mut r = 0;
    if a > 0 {
        for i in 0..a {
            if i % 2 == 0 {
                while r < b {
                    if c > 3 && b > 2 || a < 1 {
                        match i {
                            0 => { if b > 1 { r += 1; } else { r += 2; } }
                            _ => { r += 3; }
                        }
                    }
                    r += 1;
                }
            }
        }
    }
    r
}
";

fn changed_scope_gate() -> ScoreGate {
    ScoreGate {
        max_drop: Some(GateScope::Changed.default_tolerance()),
        scope: GateScope::Changed,
        ..ScoreGate::default()
    }
}

fn run_gate(dir: &Path, gate: ScoreGate) -> Result<(), Box<dyn std::error::Error>> {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir, false, &filter);
    run_diff(&cfg, "HEAD", OutputMode::Table, 10, 6, "cogcom", gate)
}

#[test]
fn changed_scope_passes_when_healthy_code_is_deleted() {
    let dir = make_git_repo_with_files(&[("good.rs", SIMPLE_FNS), ("bad.rs", NESTED_FN)]);
    fs::write(
        dir.path().join("good.rs"),
        "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )
    .unwrap();

    let project = ScoreGate {
        max_drop: Some(0.01),
        ..ScoreGate::default()
    };
    assert!(
        run_gate(dir.path(), project).is_err(),
        "the project aggregate drops when healthy code leaves the denominator"
    );
    run_gate(dir.path(), changed_scope_gate()).expect("no changed file got worse");
}

#[test]
fn changed_scope_ignores_deleted_files() {
    let dir = make_git_repo_with_files(&[("good.rs", SIMPLE_FNS), ("bad.rs", NESTED_FN)]);
    fs::remove_file(dir.path().join("good.rs")).unwrap();
    run_gate(dir.path(), changed_scope_gate()).expect("deleting a file never fails");
}

#[test]
fn changed_scope_fails_naming_the_file_that_got_worse() {
    let dir = make_git_repo_with_files(&[("good.rs", SIMPLE_FNS), ("other.rs", SIMPLE_FNS)]);
    fs::write(
        dir.path().join("good.rs"),
        format!("{SIMPLE_FNS}\n{NESTED_FN}"),
    )
    .unwrap();

    let msg = run_gate(dir.path(), changed_scope_gate())
        .expect_err("good.rs gained a deeply nested function")
        .to_string();
    assert!(msg.contains("good.rs dropped"), "{msg}");
    assert!(!msg.contains("other.rs"), "{msg}");
}

#[test]
fn changed_scope_does_not_fail_on_new_files() {
    let dir = make_git_repo_with_files(&[("good.rs", SIMPLE_FNS)]);
    fs::write(dir.path().join("bad.rs"), NESTED_FN).unwrap();
    run_gate(dir.path(), changed_scope_gate()).expect("new files have no score to fall from");
}

#[test]
fn changed_scope_fails_when_duplicated_lines_grow() {
    let dir = make_git_repo_with_files(&[("a.rs", NESTED_FN)]);
    fs::write(
        dir.path().join("b.rs"),
        NESTED_FN.replace("tangle", "tangle_copy"),
    )
    .unwrap();

    let msg = run_gate(dir.path(), changed_scope_gate())
        .expect_err("b.rs duplicates a.rs")
        .to_string();
    assert!(msg.contains("duplicated lines grew"), "{msg}");
}

#[test]
fn changed_scope_json_includes_changed_files() {
    let dir = make_git_repo_with_files(&[("good.rs", SIMPLE_FNS)]);
    fs::write(dir.path().join("new.rs"), "fn new() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run_diff(
        &cfg,
        "HEAD",
        OutputMode::Json,
        10,
        6,
        "cogcom",
        changed_scope_gate(),
    )
    .unwrap();
}

#[test]
fn changed_scope_maps_paths_when_analyzing_a_subdirectory() {
    let dir = make_git_repo_with_files(&[("root.rs", SIMPLE_FNS)]);
    let lib = dir.path().join("lib");
    fs::create_dir(&lib).unwrap();
    fs::write(lib.join("good.rs"), SIMPLE_FNS).unwrap();
    let repo = Repository::open(dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("lib/good.rs")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let sig = git2::Signature::now("Test", "test@test.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "lib", &tree, &[&head])
        .unwrap();

    fs::write(lib.join("good.rs"), format!("{SIMPLE_FNS}\n{NESTED_FN}")).unwrap();
    fs::write(dir.path().join("root.rs"), NESTED_FN).unwrap();

    let msg = run_gate(&lib, changed_scope_gate())
        .expect_err("lib/good.rs got worse")
        .to_string();
    assert!(msg.contains("lib/good.rs dropped"), "{msg}");
    assert!(!msg.contains("root.rs"), "{msg}");
}

#[test]
fn a_file_is_scored_without_its_inline_tests_when_they_are_excluded() {
    let (_dir, path) = crate::test_blocks::tests::source_with_test_module();
    let spec = crate::loc::language::detect(&path).unwrap();
    for model in [ScoringModel::Cognitive, ScoringModel::Legacy] {
        let of = |exclude_tests| {
            collector::analyze_single_file(&path, spec, exclude_tests, &model)
                .unwrap()
                .unwrap()
                .metrics
        };
        assert_eq!(of(true).code_lines, 3);
        assert_eq!(of(false).code_lines, 14);
        assert!(of(true).halstead_effort < of(false).halstead_effort);
    }
}

// ── nothing to grade ────────────────────────────────────────────────────

#[test]
fn a_target_with_nothing_to_grade_has_no_grade() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("README.md"), "# Hello\n\nWorld\n").unwrap();
    fs::write(dir.path().join("ci.yml"), "on: [push]\njobs: {}\n").unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    fs::write(dir.path().join("data.json"), "{\"key\": 1}\n").unwrap();
    fs::write(dir.path().join("empty.rs"), "").unwrap();
    fs::write(dir.path().join("comment.rs"), "// only a comment\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    for (model, dimensions) in [(ScoringModel::Cognitive, 5), (ScoringModel::Legacy, 6)] {
        let score = compute_score(&cfg, 10, 6, &model).unwrap();
        assert_eq!(score.graded, None);
        assert_eq!(score.files_analyzed, 0);
        assert_eq!(score.total_loc, 0);
        assert_eq!(score.dimensions.len(), dimensions);
        assert!(score.dimensions.iter().all(|d| d.graded.is_none()));
        let weights: f64 = score.dimensions.iter().map(|d| d.weight).sum();
        assert!((weights - 1.0).abs() < 1e-9, "weights stay: {weights}");
    }
}

#[test]
fn an_empty_source_file_is_not_graded() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("empty.rs");
    fs::write(&file, "").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(&file, false, &filter);
    let score = compute_score(&cfg, 10, 6, &ScoringModel::Cognitive).unwrap();
    assert_eq!(score.graded, None);
}

#[test]
fn fail_below_passes_when_the_target_has_nothing_to_grade() {
    let dir = make_git_repo_with_files(&[("README.md", "# Hi\n"), ("ci.yml", "on: [push]\n")]);
    fs::write(dir.path().join("README.md"), "# Hi\n\nMore docs.\n").unwrap();
    let strict = ScoreGate {
        max_drop: Some(0.01),
        fail_below: Some(analyzer::Grade::APlusPlus),
        ..ScoreGate::default()
    };
    run_gate(dir.path(), strict).expect("a directory with no code has no grade to fail");
    let filter = ExcludeFilter::default();
    let readme = dir.path().join("README.md");
    let cfg = WalkConfig::new(&readme, false, &filter);
    run_diff(&cfg, "HEAD", OutputMode::Json, 10, 6, "cogcom", strict)
        .expect("a README has no grade to fail");
}

#[test]
fn deleting_all_the_code_does_not_fail_the_gates() {
    let dir = make_git_repo_with_files(&[("lib.rs", SIMPLE_FNS), ("README.md", "# Hi\n")]);
    fs::remove_file(dir.path().join("lib.rs")).unwrap();
    let project = ScoreGate {
        max_drop: Some(0.01),
        fail_below: Some(analyzer::Grade::APlusPlus),
        ..ScoreGate::default()
    };
    run_gate(dir.path(), project).expect("no code left to compare or to grade");
    run_gate(dir.path(), changed_scope_gate()).expect("no code left to compare");
}

#[test]
fn the_first_code_in_a_target_is_held_to_fail_below() {
    let dir = make_git_repo_with_files(&[("README.md", "# Hi\n")]);
    fs::write(dir.path().join("lib.rs"), NESTED_FN).unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let grade = compute_score(&cfg, 10, 6, &ScoringModel::Cognitive)
        .unwrap()
        .graded
        .unwrap()
        .grade;
    assert!(grade.numeric_rank() < analyzer::Grade::APlusPlus.numeric_rank());

    let drop_only = ScoreGate {
        max_drop: Some(0.01),
        ..ScoreGate::default()
    };
    run_gate(dir.path(), drop_only).expect("no score at the ref to drop from");
    let strict = ScoreGate {
        fail_below: Some(analyzer::Grade::APlusPlus),
        ..drop_only
    };
    let err = run_gate(dir.path(), strict).expect_err("the new code is below A++");
    assert_eq!(
        err.to_string(),
        format!("quality gate failed: score {grade} is below minimum threshold A++")
    );
}

#[test]
fn duplication_added_where_the_ref_had_no_code_fails_the_changed_gate() {
    let dir = make_git_repo_with_files(&[("README.md", "# Hi\n")]);
    fs::write(dir.path().join("a.rs"), NESTED_FN).unwrap();
    fs::write(dir.path().join("b.rs"), NESTED_FN).unwrap();
    let err = run_gate(dir.path(), changed_scope_gate())
        .expect_err("the copies are duplicated lines the ref did not have");
    assert!(
        err.to_string()
            .starts_with("quality gate failed:\n  duplicated lines grew 0 → "),
        "{err}"
    );
}

#[test]
fn a_skipped_changed_gate_is_noted_only_when_there_is_no_code_now() {
    let changed = |before, after| diff::ScoreDiff {
        changed: Some(changed::ChangedScope {
            files: vec![],
            duplicated_lines_before: 0,
            duplicated_lines_after: 0,
        }),
        ..diff_of(before, after)
    };
    let gate = changed_scope_gate();
    assert_eq!(skipped_gates(&changed(None, Some(90.0)), gate, "src"), None);
    assert_eq!(
        skipped_gates(&changed(Some(90.0), None), gate, "src").as_deref(),
        Some("note: nothing to grade in src (no code files measured); --fail-if-worse skipped")
    );
}

#[cfg(unix)]
#[test]
fn code_that_cannot_be_read_is_an_error_not_nothing_to_grade() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("lib.rs");
    fs::write(&file, SIMPLE_FNS).unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&file).is_ok() {
        return; // running as root: the file stays readable
    }
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let score = compute_score(&cfg, 10, 6, &ScoringModel::Cognitive);
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    let err = score.expect_err("a gate must not pass on a file it could not read");
    assert_eq!(
        err.to_string(),
        "cannot grade: the code could not be read (unreadable files: 1)"
    );
}

#[cfg(unix)]
#[test]
fn an_unreadable_file_does_not_stop_the_rest_from_being_graded() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("lib.rs"), SIMPLE_FNS).unwrap();
    let locked = dir.path().join("locked.rs");
    fs::write(&locked, NESTED_FN).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&locked).is_ok() {
        return; // running as root: the file stays readable
    }
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let score = compute_score(&cfg, 10, 6, &ScoringModel::Cognitive);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();
    let score = score.expect("the file that can be read is graded");
    assert_eq!(score.files_analyzed, 1);
    assert!(score.graded.is_some());
}
