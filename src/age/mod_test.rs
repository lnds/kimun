use super::*;
use crate::cli::OutputMode;
use crate::walk::{ExcludeFilter, WalkConfig};
use git2::Repository;
use std::fs;
use std::path::Path as StdPath;

fn create_test_repo() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    (dir, repo)
}

fn make_commit(repo: &Repository, files: &[(&str, &str)], message: &str) {
    let sig =
        git2::Signature::new("Test", "test@test.com", &git2::Time::new(1_700_000_000, 0)).unwrap();
    let mut index = repo.index().unwrap();
    for (path, content) in files {
        let full_path = repo.workdir().unwrap().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full_path, content).unwrap();
        index.add_path(StdPath::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .unwrap();
}

#[test]
fn run_on_non_git_dir() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("not_a_repo");
    fs::create_dir_all(&sub).unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(&sub, false, &filter);
    let err = run(&cfg, OutputMode::Table, 90, 365, "date", None).unwrap_err();
    assert!(
        err.to_string().contains("not a git repository"),
        "should mention not a git repository, got: {err}"
    );
}

#[test]
fn run_active_days_not_less_than_frozen_days_error() {
    // Need a git repo so that the active_days >= frozen_days check is reached
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("main.rs", "fn main() {}")], "initial");
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), true, &filter);
    // active_days == frozen_days should error
    let err = run(&cfg, OutputMode::Table, 365, 90, "date", None).unwrap_err();
    assert!(
        err.to_string().contains("--active-days"),
        "should mention --active-days, got: {err}"
    );
}

#[test]
fn integration_basic_table() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, OutputMode::Table, 90, 365, "date", None);
    assert!(result.is_ok(), "age analysis should succeed: {:?}", result);
}

#[test]
fn integration_json_output() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, OutputMode::Json, 90, 365, "date", None);
    assert!(result.is_ok(), "age JSON output should succeed");
}

#[test]
fn integration_sort_by_status() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[
            ("main.rs", "fn main() {}"),
            ("lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }"),
        ],
        "add files",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, OutputMode::Table, 90, 365, "status", None);
    assert!(result.is_ok(), "sort by status should succeed");
}

#[test]
fn integration_sort_by_file() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {}"), ("lib.rs", "pub fn foo() {}")],
        "add files",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, OutputMode::Table, 90, 365, "file", None);
    assert!(result.is_ok(), "sort by file should succeed");
}

#[test]
fn integration_status_filter_active() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("main.rs", "fn main() {}")], "initial");

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, OutputMode::Table, 90, 365, "date", Some("active"));
    assert!(result.is_ok(), "filter by active should succeed");
}

#[test]
fn integration_status_filter_stale() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("main.rs", "fn main() {}")], "initial");

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, OutputMode::Table, 90, 365, "date", Some("stale"));
    assert!(result.is_ok(), "filter by stale should succeed");
}

#[test]
fn integration_status_filter_frozen() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("main.rs", "fn main() {}")], "initial");

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, OutputMode::Table, 90, 365, "date", Some("frozen"));
    assert!(result.is_ok(), "filter by frozen should succeed");
}

#[test]
fn integration_empty_repo_no_files() {
    let (dir, repo) = create_test_repo();
    // Commit a non-source file (no recognized language)
    make_commit(&repo, &[("data.xyz", "not code")], "add data");

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should succeed even with no source files found
    let result = run(&cfg, OutputMode::Table, 90, 365, "date", None);
    assert!(
        result.is_ok(),
        "no source files should not crash: {:?}",
        result
    );
}

#[test]
fn run_on_current_repo() {
    // Smoke test on the actual project repo
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(StdPath::new("."), false, &filter);
    let result = run(&cfg, OutputMode::Table, 90, 365, "date", None);
    assert!(result.is_ok(), "age analysis should work on current repo");
}

#[test]
fn run_on_current_repo_json() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(StdPath::new("."), false, &filter);
    let result = run(&cfg, OutputMode::Json, 90, 365, "date", None);
    assert!(result.is_ok(), "age JSON should work on current repo");
}
