use super::*;
use std::fs;
use std::path::Path as StdPath;

use git2::Repository;

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
    let err = run(&sub, false, false, 20, "strength", None, 3, None).unwrap_err();
    assert!(
        err.to_string().contains("not a git repository"),
        "should mention not a git repository, got: {err}"
    );
}

#[test]
fn run_min_degree_zero_rejected() {
    let err = run(
        StdPath::new("."),
        false,
        false,
        20,
        "strength",
        None,
        0,
        None,
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("--min-degree must be at least 1"),
        "should reject min_degree=0, got: {err}"
    );
}

#[test]
fn integration_basic() {
    let (dir, repo) = create_test_repo();
    // 3 commits touching a.rs + b.rs together
    for i in 0..3 {
        make_commit(
            &repo,
            &[
                ("a.rs", &format!("fn a() {{ {i} }}")),
                ("b.rs", &format!("fn b() {{ {i} }}")),
            ],
            &format!("commit {i}"),
        );
    }

    // Verify coupling is found by using compute_coupling directly
    let git_repo = GitRepo::open(dir.path()).unwrap();
    let freqs = git_repo.file_frequencies(None).unwrap();
    let freq_map: HashMap<PathBuf, usize> =
        freqs.into_iter().map(|f| (f.path, f.commits)).collect();
    let co = git_repo.co_changing_commits(None).unwrap();
    let results = compute_coupling(&co, &freq_map, 3);

    assert_eq!(results.len(), 1, "should find exactly one coupled pair");
    assert_eq!(results[0].shared_commits, 3);
    assert!((results[0].strength - 1.0).abs() < 0.001);

    // Also verify run() succeeds
    let result = run(dir.path(), false, false, 20, "strength", None, 3, None);
    assert!(result.is_ok(), "basic coupling should succeed");
}

#[test]
fn integration_json() {
    let (dir, repo) = create_test_repo();
    for i in 0..3 {
        make_commit(
            &repo,
            &[
                ("a.rs", &format!("fn a() {{ {i} }}")),
                ("b.rs", &format!("fn b() {{ {i} }}")),
            ],
            &format!("commit {i}"),
        );
    }
    let result = run(dir.path(), true, false, 20, "strength", None, 3, None);
    assert!(result.is_ok(), "JSON output should succeed");
}

#[test]
fn integration_no_coupling() {
    let (dir, repo) = create_test_repo();
    // Each commit touches only one file
    for i in 0..3 {
        make_commit(
            &repo,
            &[("a.rs", &format!("fn a() {{ {i} }}"))],
            &format!("commit a {i}"),
        );
    }
    for i in 0..3 {
        make_commit(
            &repo,
            &[("b.rs", &format!("fn b() {{ {i} }}"))],
            &format!("commit b {i}"),
        );
    }
    let result = run(dir.path(), false, false, 20, "strength", None, 3, None);
    assert!(result.is_ok(), "no coupling should succeed");
}

#[test]
fn integration_min_degree_filter() {
    let (dir, repo) = create_test_repo();
    // Only 2 commits touching a+b → below min_degree=3
    make_commit(
        &repo,
        &[("a.rs", "fn a() { 1 }"), ("b.rs", "fn b() { 1 }")],
        "c1",
    );
    make_commit(
        &repo,
        &[("a.rs", "fn a() { 2 }"), ("b.rs", "fn b() { 2 }")],
        "c2",
    );
    let result = run(dir.path(), false, false, 20, "strength", None, 3, None);
    assert!(result.is_ok(), "min_degree filter should not crash");
}

#[test]
fn integration_test_file_excluded() {
    let (dir, repo) = create_test_repo();
    // 3 commits touching a.rs + a_test.rs
    for i in 0..3 {
        make_commit(
            &repo,
            &[
                ("a.rs", &format!("fn a() {{ {i} }}")),
                ("a_test.rs", &format!("fn test_a() {{ {i} }}")),
            ],
            &format!("commit {i}"),
        );
    }
    // With exclude_tests (default), test file should be filtered
    let git_repo = GitRepo::open(dir.path()).unwrap();
    let freqs = git_repo.file_frequencies(None).unwrap();
    let freq_map: HashMap<PathBuf, usize> = freqs
        .into_iter()
        .filter(|f| f.commits >= 3)
        .filter(|f| !is_test_path(&f.path))
        .map(|f| (f.path, f.commits))
        .collect();
    assert!(
        !freq_map.contains_key(&PathBuf::from("a_test.rs")),
        "test file should be excluded from freq_map"
    );
}

#[test]
fn test_is_test_path() {
    assert!(is_test_path(Path::new("tests/foo.rs")));
    assert!(is_test_path(Path::new("src/tests/bar.rs")));
    assert!(is_test_path(Path::new("counter_test.rs")));
    assert!(!is_test_path(Path::new("src/counter.rs")));
    assert!(!is_test_path(Path::new("src/main.rs")));
}

#[test]
fn run_on_current_repo() {
    // Smoke test on the actual repo
    let result = run(
        StdPath::new("."),
        false,
        false,
        5,
        "strength",
        None,
        3,
        None,
    );
    assert!(result.is_ok(), "tc should succeed on a git repo");
}

#[test]
fn run_on_current_repo_json() {
    let result = run(StdPath::new("."), true, false, 5, "strength", None, 3, None);
    assert!(result.is_ok(), "tc JSON should succeed on a git repo");
}
