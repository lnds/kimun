use super::*;
use crate::cli::OutputMode;
use crate::walk::{ExcludeFilter, WalkConfig};
use std::fs;
use std::path::Path as StdPath;

use git2::Repository;

fn opts<'a>(
    output: OutputMode,
    top: usize,
    sort_by: &'a str,
    since: Option<&'a str>,
    risk_only: bool,
    summary: bool,
) -> KnowledgeOptions<'a> {
    KnowledgeOptions {
        output,
        top,
        sort_by,
        since,
        risk_only,
        summary,
        bus_factor: false,
        author: None,
    }
}

#[test]
fn test_is_generated() {
    assert!(is_generated(StdPath::new("Cargo.lock")));
    assert!(is_generated(StdPath::new("package-lock.json")));
    assert!(is_generated(StdPath::new("app.min.js")));
    assert!(is_generated(StdPath::new("main.bundle.js")));
    assert!(is_generated(StdPath::new("proto.pb.go")));
    assert!(is_generated(StdPath::new("msg_pb2.py")));
    assert!(is_generated(StdPath::new("foo.generated.ts")));
    assert!(!is_generated(StdPath::new("main.rs")));
    assert!(!is_generated(StdPath::new("lib.js")));
}

#[test]
fn run_on_non_git_dir() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("not_a_repo");
    fs::create_dir_all(&sub).unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(&sub, false, &filter);
    let err = run(
        &cfg,
        &opts(OutputMode::Table, 20, "concentration", None, false, false),
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("not a git repository"),
        "should mention not a git repo, got: {err}"
    );
}

fn create_test_repo() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Fresia").unwrap();
    config.set_str("user.email", "fresia@ruca.mapu").unwrap();
    (dir, repo)
}

fn make_commit(repo: &Repository, files: &[(&str, &str)], message: &str) {
    let sig = git2::Signature::new(
        "Fresia",
        "fresia@ruca.mapu",
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
fn integration_basic() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        &opts(OutputMode::Table, 20, "concentration", None, false, false),
    );
    assert!(result.is_ok(), "knowledge map should succeed on a git repo");
}

#[test]
fn integration_json() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        &opts(OutputMode::Json, 20, "concentration", None, false, false),
    );
    assert!(result.is_ok(), "JSON output should succeed");
}

#[test]
fn integration_risk_only() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        &opts(OutputMode::Table, 20, "risk", None, true, false),
    );
    assert!(result.is_ok(), "risk-only filter should work");
}

#[test]
fn integration_sort_by_risk() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        &opts(OutputMode::Table, 20, "risk", None, false, false),
    );
    assert!(result.is_ok(), "sort by risk should work");
}

#[test]
fn integration_sort_by_diffusion() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        &opts(OutputMode::Table, 20, "diffusion", None, false, false),
    );
    assert!(result.is_ok(), "sort by diffusion should work");
}

#[test]
fn run_on_current_repo() {
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(StdPath::new("."), false, &filter);
    let result = run(
        &cfg,
        &opts(OutputMode::Json, 5, "concentration", None, false, false),
    );
    assert!(result.is_ok(), "knowledge map should work on current repo");
}

#[test]
fn integration_summary() {
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
    let result = run(
        &cfg,
        &opts(OutputMode::Table, 20, "concentration", None, false, true),
    );
    assert!(result.is_ok(), "summary mode should work");
}

#[test]
fn integration_summary_json() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        &opts(OutputMode::Json, 20, "concentration", None, false, true),
    );
    assert!(result.is_ok(), "summary JSON mode should work");
}

#[test]
fn integration_with_since_filter() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        &opts(
            OutputMode::Table,
            20,
            "concentration",
            Some("1y"),
            false,
            false,
        ),
    );
    assert!(result.is_ok(), "since filter should work: {:?}", result);
}

#[test]
fn integration_with_since_filter_json() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        &opts(
            OutputMode::Json,
            20,
            "concentration",
            Some("1d"),
            false,
            false,
        ),
    );
    assert!(
        result.is_ok(),
        "since filter JSON should work: {:?}",
        result
    );
}

#[test]
fn integration_with_generated_file_skipped() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[
            ("main.rs", "fn main() {}"),
            ("Cargo.lock", "[dependencies]\n"),
        ],
        "add files",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        &opts(OutputMode::Table, 20, "concentration", None, false, false),
    );
    assert!(result.is_ok(), "generated files should be skipped");
}

#[test]
fn integration_author_filter_match() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[
            ("main.rs", "fn main() {}\n"),
            ("lib.rs", "pub fn foo() {}\n"),
        ],
        "add files",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // "Fresia" matches the committer name — should return files
    let result = run(
        &cfg,
        &KnowledgeOptions {
            output: OutputMode::Table,
            top: 20,
            sort_by: "concentration",
            since: None,
            risk_only: false,
            summary: false,
            bus_factor: false,
            author: Some("Fresia"),
        },
    );
    assert!(result.is_ok(), "author filter should succeed");
}

#[test]
fn integration_author_filter_no_match() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("main.rs", "fn main() {}\n")], "add main");

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    // "nonexistent" won't match — should produce an empty result without error
    let result = run(
        &cfg,
        &KnowledgeOptions {
            output: OutputMode::Table,
            top: 20,
            sort_by: "concentration",
            since: None,
            risk_only: false,
            summary: false,
            bus_factor: false,
            author: Some("nonexistent"),
        },
    );
    assert!(
        result.is_ok(),
        "author filter with no match should not error"
    );
}

#[test]
fn integration_bus_factor_table() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        &KnowledgeOptions {
            output: OutputMode::Table,
            top: 20,
            sort_by: "concentration",
            since: None,
            risk_only: false,
            summary: false,
            bus_factor: true,
            author: None,
        },
    );
    assert!(
        result.is_ok(),
        "bus factor table should succeed: {result:?}"
    );
}

#[test]
fn integration_bus_factor_json() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        &KnowledgeOptions {
            output: OutputMode::Json,
            top: 20,
            sort_by: "concentration",
            since: None,
            risk_only: false,
            summary: false,
            bus_factor: true,
            author: None,
        },
    );
    assert!(result.is_ok(), "bus factor JSON should succeed: {result:?}");
}

#[test]
fn integration_summary_sort_by_diffusion() {
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
    let result = run(
        &cfg,
        &KnowledgeOptions {
            output: OutputMode::Table,
            top: 20,
            sort_by: "diffusion",
            since: None,
            risk_only: false,
            summary: true,
            bus_factor: false,
            author: None,
        },
    );
    assert!(
        result.is_ok(),
        "summary diffusion sort should work: {result:?}"
    );
}

#[test]
fn integration_summary_sort_by_risk() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let result = run(
        &cfg,
        &KnowledgeOptions {
            output: OutputMode::Table,
            top: 20,
            sort_by: "risk",
            since: None,
            risk_only: false,
            summary: true,
            bus_factor: false,
            author: None,
        },
    );
    assert!(result.is_ok(), "summary risk sort should work: {result:?}");
}
