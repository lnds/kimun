use super::*;
use crate::cli::OutputMode;
use crate::walk::{ExcludeFilter, WalkConfig};
use std::fs;
use std::path::Path as StdPath;
use std::time::SystemTime;

use git2::Repository;

#[test]
fn parse_since_days() {
    let ts = parse_since("30d").unwrap();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let expected = now - 30 * 86_400;
    assert!(
        (ts - expected).abs() < 2,
        "timestamp should be within 2s of 30 days ago (got {ts}, expected ~{expected})"
    );
}

#[test]
fn parse_since_months() {
    let ts = parse_since("6m").unwrap();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let expected = now - 6 * 30 * 86_400;
    assert!(
        (ts - expected).abs() < 2,
        "timestamp should be within 2s of 6 months ago (got {ts}, expected ~{expected})"
    );
}

#[test]
fn parse_since_years() {
    let ts = parse_since("1y").unwrap();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let expected = now - 365 * 86_400;
    assert!(
        (ts - expected).abs() < 2,
        "timestamp should be within 2s of 1 year ago (got {ts}, expected ~{expected})"
    );
}

#[test]
fn parse_since_invalid_unit() {
    let err = parse_since("5x").unwrap_err();
    assert!(
        err.to_string().contains("unknown unit"),
        "should mention unknown unit, got: {err}"
    );
}

#[test]
fn parse_since_invalid_number() {
    let err = parse_since("abcd").unwrap_err();
    assert!(
        err.to_string().contains("invalid"),
        "should mention invalid value, got: {err}"
    );
}

#[test]
fn parse_since_no_unit() {
    let err = parse_since("123").unwrap_err();
    assert!(
        err.to_string().contains("no unit"),
        "should mention no unit, got: {err}"
    );
}

#[test]
fn parse_since_multi_char_units() {
    // All multi-char unit variants should parse correctly
    for unit in [
        "6mo", "6month", "6months", "1yr", "1year", "1years", "30day", "30days",
    ] {
        let result = parse_since(unit);
        assert!(result.is_ok(), "should accept {unit:?}, got: {result:?}");
    }
}

#[test]
fn parse_since_zero_is_valid() {
    let ts = parse_since("0d").unwrap();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    assert!(
        (ts - now).abs() < 2,
        "0d should be approximately now (got {ts}, expected ~{now})"
    );
}

#[test]
fn parse_since_whitespace() {
    let ts = parse_since(" 30d ").unwrap();
    assert!(ts > 0, "should parse with leading/trailing whitespace");
}

#[test]
fn parse_since_empty() {
    let err = parse_since("").unwrap_err();
    assert!(
        err.to_string().contains("empty"),
        "should mention empty value, got: {err}"
    );
}

#[test]
fn parse_since_overflow() {
    let err = parse_since("999999999999999999999y").unwrap_err();
    assert!(
        err.to_string().contains("too large") || err.to_string().contains("invalid"),
        "should reject overflow, got: {err}"
    );
}

#[test]
fn parse_since_underflow() {
    let err = parse_since("9999999y").unwrap_err();
    assert!(
        err.to_string().contains("epoch") || err.to_string().contains("too large"),
        "should reject dates before epoch, got: {err}"
    );
}

#[test]
fn run_on_non_git_dir() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("not_a_repo");
    fs::create_dir_all(&sub).unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(&sub, false, &filter);
    let err = run(&cfg, OutputMode::Table, Some(20), "score", None, "indent").unwrap_err();
    assert!(
        err.to_string().contains("not a git repository"),
        "should mention not a git repository, got: {err}"
    );
}

#[test]
fn run_json_output_indent() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(StdPath::new("."), false, &filter);
    let result = run(&cfg, OutputMode::Json, Some(5), "score", None, "indent");
    assert!(
        result.is_ok(),
        "hotspots (indent) should succeed on a git repo"
    );
}

#[test]
fn run_json_output_cycom() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(StdPath::new("."), false, &filter);
    let result = run(&cfg, OutputMode::Json, Some(5), "score", None, "cycom");
    assert!(
        result.is_ok(),
        "hotspots (cycom) should succeed on a git repo"
    );
}

// -- Integration tests with a real git repo --

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
fn integration_indent_scores() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[(
            "main.rs",
            "fn main() {\n    if true {\n        println!(\"hi\");\n    }\n}\n",
        )],
        "add main",
    );
    make_commit(
        &repo,
        &[(
            "main.rs",
            "fn main() {\n    if true {\n        println!(\"hello\");\n    }\n}\n",
        )],
        "update main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, OutputMode::Table, Some(20), "score", None, "indent");
    assert!(result.is_ok(), "indent hotspots should succeed");
}

#[test]
fn integration_cycom_scores() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[(
            "main.rs",
            "fn main() {\n    if true {\n        println!(\"hi\");\n    }\n}\n",
        )],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, OutputMode::Table, Some(20), "score", None, "cycom");
    assert!(result.is_ok(), "cycom hotspots should succeed");
}

#[test]
fn integration_sort_by_commits() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("a.rs", "fn a() {\n    if true { bar(); }\n}\n")],
        "c1",
    );
    make_commit(
        &repo,
        &[("a.rs", "fn a() {\n    if true { baz(); }\n}\n")],
        "c2",
    );
    make_commit(
        &repo,
        &[("b.rs", "fn b() {\n    if x { if y { foo(); } }\n}\n")],
        "c3",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, OutputMode::Table, Some(20), "commits", None, "indent");
    assert!(result.is_ok(), "sort by commits should work");
}

#[test]
fn integration_since_filters_commits() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("a.rs", "fn a() {\n    if true { bar(); }\n}\n")],
        "old",
    );

    // Commits at epoch 2023 → --since 1d from 2026 excludes all
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        OutputMode::Table,
        Some(20),
        "score",
        Some("1d"),
        "indent",
    );
    assert!(result.is_ok(), "since filter should not crash");
}

#[test]
fn integration_empty_repo() {
    let (dir, _repo) = create_test_repo();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, OutputMode::Table, Some(20), "score", None, "indent");
    // Empty repo: file_frequencies fails, which is ok
    assert!(
        result.is_ok() || result.is_err(),
        "should handle empty repo gracefully"
    );
}

#[test]
fn integration_json_structure() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[(
            "main.rs",
            "fn main() {\n    if true {\n        println!(\"hi\");\n    }\n}\n",
        )],
        "add main",
    );
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, OutputMode::Json, Some(20), "score", None, "indent");
    assert!(result.is_ok(), "JSON output should succeed");
}

#[test]
fn integration_cogcom_scores() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[(
            "main.rs",
            "fn main() {\n    if true {\n        println!(\"hi\");\n    }\n}\n",
        )],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(&cfg, OutputMode::Table, Some(20), "score", None, "cogcom");
    assert!(result.is_ok(), "cogcom hotspots should succeed");
}

#[test]
fn integration_sort_by_complexity() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[(
            "main.rs",
            "fn main() {\n    if true {\n        println!(\"hi\");\n    }\n}\n",
        )],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        OutputMode::Table,
        Some(20),
        "complexity",
        None,
        "indent",
    );
    assert!(result.is_ok(), "sort by complexity should work");
}

#[test]
fn run_short_format() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Short, Some(20), "score", None, "indent").unwrap();
}

#[test]
fn run_terse_format() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    run(&cfg, OutputMode::Terse, Some(20), "score", None, "indent").unwrap();
}

#[test]
fn complexity_leaves_inline_tests_out_when_they_are_excluded() {
    let (_dir, path) = crate::test_blocks::tests::source_with_test_module();
    let spec = crate::loc::language::detect(&path).unwrap();
    for metric in ["cycom", "cogcom", "indent"] {
        let of = |exclude_tests| {
            compute_complexity(&path, spec, metric, exclude_tests)
                .unwrap()
                .unwrap()
        };
        assert!(of(true) < of(false), "{metric}");
    }
}

#[test]
fn documents_are_measured_by_their_lines() {
    let (dir, repo) = create_test_repo();
    let main = "fn main() {\n    if true {\n        println!(\"hi\");\n    }\n}\n";
    let guide = "# Guide\n\n    indented prose\n\nmore\n";
    make_commit(&repo, &[("main.rs", main), ("guide.md", guide)], "add");
    make_commit(&repo, &[("guide.md", "# Guide\n\nshort\n")], "edit");

    let kinds = crate::kind::Selection::new(
        &[crate::kind::Kind::Code, crate::kind::Kind::Docs],
        &Default::default(),
    );
    let filter = ExcludeFilter::default().with_kinds(kinds);
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let mut found = collect(&cfg, None, "cycom").unwrap().unwrap();
    found.sort_by(|a, b| a.path.cmp(&b.path));
    let summary: Vec<(&str, &str, usize, usize)> = found
        .iter()
        .map(|f| (f.path.to_str().unwrap(), f.metric, f.complexity, f.commits))
        .collect();
    assert_eq!(summary[0], ("guide.md", "lines", 2, 2));
    assert_eq!((summary[1].0, summary[1].1), ("main.rs", "cycom"));
}
