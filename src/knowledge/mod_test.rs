use super::*;
use std::fs;
use std::path::Path as StdPath;

use git2::Repository;

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
    let err = run(&sub, false, false, 20, "concentration", None, false).unwrap_err();
    assert!(
        err.to_string().contains("not a git repository"),
        "should mention not a git repo, got: {err}"
    );
}

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
fn integration_basic() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[("main.rs", "fn main() {\n    println!(\"hi\");\n}\n")],
        "add main",
    );

    let result = run(dir.path(), false, false, 20, "concentration", None, false);
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

    let result = run(dir.path(), true, false, 20, "concentration", None, false);
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

    let result = run(dir.path(), false, false, 20, "risk", None, true);
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

    let result = run(dir.path(), false, false, 20, "risk", None, false);
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

    let result = run(dir.path(), false, false, 20, "diffusion", None, false);
    assert!(result.is_ok(), "sort by diffusion should work");
}

#[test]
fn run_on_current_repo() {
    let result = run(
        StdPath::new("."),
        true,
        false,
        5,
        "concentration",
        None,
        false,
    );
    assert!(result.is_ok(), "knowledge map should work on current repo");
}
