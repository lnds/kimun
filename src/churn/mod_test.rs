use crate::cli::OutputMode;
use crate::walk::{ExcludeFilter, WalkConfig};
use git2::Repository;
use std::fs;
use std::path::Path as StdPath;

#[test]
fn run_on_current_repo() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(std::path::Path::new("."), false, &filter);
    super::run(&cfg, OutputMode::Table, 20, "commits", None).unwrap();
}

#[test]
fn run_json_on_current_repo() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(std::path::Path::new("."), false, &filter);
    super::run(&cfg, OutputMode::Json, 20, "commits", None).unwrap();
}

#[test]
fn run_sort_by_rate() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(std::path::Path::new("."), false, &filter);
    super::run(&cfg, OutputMode::Table, 20, "rate", None).unwrap();
}

#[test]
fn run_sort_by_file() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(std::path::Path::new("."), false, &filter);
    super::run(&cfg, OutputMode::Table, 20, "file", None).unwrap();
}

#[test]
fn run_with_since() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(std::path::Path::new("."), false, &filter);
    super::run(&cfg, OutputMode::Table, 20, "commits", Some("1y")).unwrap();
}

#[test]
fn run_on_non_git_dir() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("not_a_repo");
    fs::create_dir_all(&sub).unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(&sub, false, &filter);
    let err = super::run(&cfg, OutputMode::Table, 20, "commits", None).unwrap_err();
    assert!(
        err.to_string().contains("not a git repository"),
        "should mention not a git repository, got: {err}"
    );
}

#[test]
fn run_on_empty_repo_with_since() {
    // Same as above but with a since filter — exercises the "with since" empty message
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config.set_str("user.email", "test@test.com").unwrap();

    // Commit only one file with an old timestamp so "1d" filter excludes it
    let sig =
        git2::Signature::new("Test", "test@test.com", &git2::Time::new(1_700_000_000, 0)).unwrap();
    let file_path = dir.path().join("main.rs");
    fs::write(&file_path, "fn main() {}").unwrap();
    {
        let mut index = repo.index().unwrap();
        index.add_path(StdPath::new("main.rs")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
    }

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // 1d filter: commits from epoch 1_700_000_000 are old, so freqs will be empty
    let result = super::run(&cfg, OutputMode::Table, 20, "commits", Some("1d"));
    assert!(
        result.is_ok(),
        "since-filtered empty repo should not crash: {:?}",
        result
    );
}

#[test]
fn run_short_format() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(std::path::Path::new("."), false, &filter);
    super::run(&cfg, OutputMode::Short, 20, "commits", None).unwrap();
}

#[test]
fn run_terse_format() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(std::path::Path::new("."), false, &filter);
    super::run(&cfg, OutputMode::Terse, 20, "commits", None).unwrap();
}
