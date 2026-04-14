use super::*;
use crate::cli::OutputMode;
use crate::walk::{ExcludeFilter, WalkConfig};
use git2::Repository;
use std::fs;
use std::path::Path as StdPath;

#[test]
fn run_on_temp_dir_with_rust_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    // hello\n    println!(\"hi\");\n}\n",
    )
    .unwrap();

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should succeed without error
    run(&cfg, false, OutputMode::Table).unwrap();
}

#[test]
fn run_on_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should succeed and print "No recognized source files found."
    run(&cfg, false, OutputMode::Table).unwrap();
}

#[test]
fn run_skips_binary_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should succeed — binary file silently skipped
    run(&cfg, false, OutputMode::Table).unwrap();
}

#[test]
fn run_deduplicates_identical_files() {
    let dir = tempfile::tempdir().unwrap();
    let content = "int x = 1;\n";
    fs::write(dir.path().join("a.c"), content).unwrap();
    fs::write(dir.path().join("b.c"), content).unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should succeed — one of the duplicates skipped
    run(&cfg, false, OutputMode::Table).unwrap();
}

#[test]
fn run_with_shebang_detection() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("script"),
        "#!/usr/bin/env python3\nprint('hello')\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, OutputMode::Table).unwrap();
}

#[test]
fn run_verbose_on_temp_dir() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(dir.path().join("lib.rs"), "pub fn x() {}\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should succeed with verbose stats printed
    run(&cfg, true, OutputMode::Table).unwrap();
}

#[test]
fn run_verbose_with_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let content = "int x = 1;\n";
    fs::write(dir.path().join("a.c"), content).unwrap();
    fs::write(dir.path().join("b.c"), content).unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // Should show skipped_files=1 (duplicate)
    run(&cfg, true, OutputMode::Table).unwrap();
}

#[test]
fn run_verbose_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, true, OutputMode::Table).unwrap();
}

#[test]
fn run_json_output() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    println!(\"hi\");\n}\n",
    )
    .unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, OutputMode::Json).unwrap();
}

#[test]
fn run_json_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, false, OutputMode::Json).unwrap();
}

#[test]
fn hash_file_works() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    fs::write(&path, "hello world").unwrap();

    let h1 = hash_file(&path).unwrap();
    let h2 = hash_file(&path).unwrap();
    assert_eq!(h1, h2, "same content should produce same hash");
}

#[test]
fn hash_file_nonexistent() {
    assert!(hash_file(StdPath::new("/nonexistent/file")).is_none());
}

// ── run_by_author tests ─────────────────────────────────────────────────

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
fn run_by_author_basic() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "initial",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run_by_author(&cfg, OutputMode::Table);
    assert!(result.is_ok(), "run_by_author should succeed: {:?}", result);
}

#[test]
fn run_by_author_json() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "initial",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run_by_author(&cfg, OutputMode::Json);
    assert!(result.is_ok(), "run_by_author JSON should succeed");
}

#[test]
fn run_by_author_multiple_files() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[
            ("main.rs", "fn main() {\n    let x = 1;\n}\n"),
            (
                "lib.rs",
                "// lib\npub fn add(a: i32, b: i32) -> i32 { a + b }\n",
            ),
        ],
        "add files",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run_by_author(&cfg, OutputMode::Table);
    assert!(result.is_ok(), "multi-file run_by_author should succeed");
}

#[test]
fn run_by_author_empty_repo() {
    // No source files — exercises the empty by_author branch
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("data.xyz", "not code")], "add data");

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run_by_author(&cfg, OutputMode::Table);
    assert!(result.is_ok(), "empty repo should not crash: {:?}", result);
}

#[test]
fn run_by_author_empty_json() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("data.xyz", "not code")], "add data");

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run_by_author(&cfg, OutputMode::Json);
    assert!(result.is_ok(), "empty repo JSON should not crash");
}

#[test]
fn run_by_author_on_current_repo() {
    // Smoke test on the actual repo
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(StdPath::new("."), false, &filter);
    let result = run_by_author(&cfg, OutputMode::Table);
    assert!(
        result.is_ok(),
        "run_by_author on current repo should succeed"
    );
}

#[test]
fn run_by_author_json_on_current_repo() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(StdPath::new("."), false, &filter);
    let result = run_by_author(&cfg, OutputMode::Json);
    assert!(
        result.is_ok(),
        "run_by_author JSON on current repo should succeed"
    );
}

#[test]
fn run_short_format() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(StdPath::new("."), false, &filter);
    run(&cfg, false, OutputMode::Short).unwrap();
}

#[test]
fn run_terse_format() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(StdPath::new("."), false, &filter);
    run(&cfg, false, OutputMode::Terse).unwrap();
}

#[test]
fn run_codeclimate_returns_error() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(StdPath::new("."), false, &filter);
    let result = run(&cfg, false, OutputMode::Codeclimate);
    assert!(
        result.is_err(),
        "codeclimate format should not be supported by loc"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("codeclimate"),
        "error message should mention codeclimate"
    );
}
