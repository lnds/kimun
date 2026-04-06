use super::*;
use crate::walk::{ExcludeFilter, WalkConfig};
use git2::Repository;
use std::fs;
use std::path::Path as StdPath;

fn create_test_repo() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Alice").unwrap();
    config.set_str("user.email", "alice@example.com").unwrap();
    (dir, repo)
}

fn make_commit(repo: &Repository, files: &[(&str, &str)], message: &str) {
    let sig = git2::Signature::new(
        "Alice",
        "alice@example.com",
        &git2::Time::new(1_700_000_000, 0),
    )
    .unwrap();
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
    let err = run(&cfg, false, None).unwrap_err();
    assert!(
        err.to_string().contains("not a git repository"),
        "should mention not a git repository, got: {err}"
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
    let result = run(&cfg, false, None);
    assert!(
        result.is_ok(),
        "author analysis should succeed: {:?}",
        result
    );
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
    let result = run(&cfg, true, None);
    assert!(result.is_ok(), "author JSON output should succeed");
}

#[test]
fn integration_multiple_files() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[
            ("main.rs", "fn main() {\n    println!(\"hi\");\n}\n"),
            ("lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n"),
        ],
        "add files",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, false, None);
    assert!(result.is_ok(), "multi-file author analysis should succeed");
}

#[test]
fn integration_empty_repo() {
    // Repo with only non-source files
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("data.xyz", "not code")], "add data");

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should return Ok with "No authors found." or empty JSON
    let result = run(&cfg, false, None);
    assert!(result.is_ok(), "empty repo should not crash: {:?}", result);
}

#[test]
fn integration_empty_json() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("data.xyz", "not code")], "add data");

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, true, None);
    assert!(result.is_ok(), "empty repo JSON should not crash");
}

#[test]
fn integration_with_since_filter() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("main.rs", "fn main() {}")], "initial");

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Use "1y" — all commits should pass (happened recently)
    let result = run(&cfg, false, Some("1y"));
    assert!(
        result.is_ok(),
        "since filter should not crash: {:?}",
        result
    );
}

#[test]
fn integration_with_since_filter_excludes_all() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("main.rs", "fn main() {}")], "initial");

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Use "1d" — commits at epoch 1_700_000_000 are old, so lines should be filtered out
    let result = run(&cfg, false, Some("1d"));
    // Should succeed (returns empty or "No authors found.")
    assert!(
        result.is_ok(),
        "since filter should not crash: {:?}",
        result
    );
}

#[test]
fn run_on_current_repo() {
    // Smoke test on the actual project repo
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(StdPath::new("."), false, &filter);
    let result = run(&cfg, false, None);
    assert!(
        result.is_ok(),
        "author analysis should work on current repo"
    );
}

#[test]
fn run_on_current_repo_json() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(StdPath::new("."), false, &filter);
    let result = run(&cfg, true, None);
    assert!(result.is_ok(), "author JSON should work on current repo");
}
