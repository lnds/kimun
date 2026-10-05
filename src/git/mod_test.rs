use super::*;
use std::fs;

fn is_git_repo(path: &Path) -> bool {
    Repository::discover(path).is_ok()
}

fn create_test_repo() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    // Configure identity for commits
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config.set_str("user.email", "test@test.com").unwrap();

    (dir, repo)
}

fn make_commit_at(
    repo: &Repository,
    files: &[(&str, &str)],
    message: &str,
    epoch: i64,
) -> git2::Oid {
    let sig = git2::Signature::new("Test", "test@test.com", &git2::Time::new(epoch, 0)).unwrap();
    let mut index = repo.index().unwrap();

    for (path, content) in files {
        let full_path = repo.workdir().unwrap().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full_path, content).unwrap();
        index.add_path(Path::new(path)).unwrap();
    }

    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .unwrap()
}

fn make_commit(repo: &Repository, files: &[(&str, &str)], message: &str) -> git2::Oid {
    make_commit_at(repo, files, message, 1_700_000_000)
}

#[test]
fn test_open_repo() {
    let (dir, _repo) = create_test_repo();
    let git_repo = GitRepo::open(dir.path());
    assert!(git_repo.is_ok());
    assert!(is_git_repo(dir.path()));
}

#[test]
fn test_open_not_repo() {
    let dir = tempfile::tempdir().unwrap();
    // Don't init git — just a plain directory
    let sub = dir.path().join("not_a_repo");
    fs::create_dir_all(&sub).unwrap();
    assert!(GitRepo::open(&sub).is_err());
    assert!(!is_git_repo(&sub));
}

#[test]
fn test_file_frequencies() {
    let (dir, repo) = create_test_repo();

    make_commit(&repo, &[("a.rs", "fn a() {}")], "add a");
    make_commit(&repo, &[("b.rs", "fn b() {}")], "add b");
    make_commit(&repo, &[("a.rs", "fn a() { 1 }")], "modify a");

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let freqs = git_repo.file_frequencies(None).unwrap();

    assert_eq!(freqs.len(), 2);

    let a = freqs.iter().find(|f| f.path == Path::new("a.rs")).unwrap();
    assert_eq!(a.commits, 2);

    let b = freqs.iter().find(|f| f.path == Path::new("b.rs")).unwrap();
    assert_eq!(b.commits, 1);
}

#[test]
fn test_file_frequencies_since() {
    let (dir, repo) = create_test_repo();

    make_commit_at(&repo, &[("a.rs", "v1")], "first", 1_000_000);
    make_commit_at(&repo, &[("b.rs", "v1")], "second", 2_000_000);

    let git_repo = GitRepo::open(dir.path()).unwrap();
    // Filter: only commits at or after 1_500_000 → only the second commit
    let freqs = git_repo.file_frequencies(Some(1_500_000)).unwrap();

    assert_eq!(freqs.len(), 1);
    assert_eq!(freqs[0].path, Path::new("b.rs"));
}

#[test]
fn test_co_changing_commits() {
    let (dir, repo) = create_test_repo();

    // Single-file commit — should NOT appear
    make_commit(&repo, &[("a.rs", "v1")], "one file");

    // Multi-file commit — should appear
    make_commit(&repo, &[("b.rs", "v1"), ("c.rs", "v1")], "two files");

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let co = git_repo.co_changing_commits(None).unwrap();

    assert_eq!(co.len(), 1);
    assert_eq!(co[0].len(), 2);
    assert!(co[0].contains(&PathBuf::from("b.rs")));
    assert!(co[0].contains(&PathBuf::from("c.rs")));
}

#[test]
fn test_blame_single_author() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("a.rs", "line1\nline2\nline3\n")], "add a");

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let blames = git_repo.blame_file(Path::new("a.rs")).unwrap();

    assert_eq!(blames.len(), 1, "single author should produce 1 entry");
    assert_eq!(blames[0].email, "test@test.com");
    assert_eq!(blames[0].lines, 3);
}

#[test]
fn test_blame_multiple_authors() {
    let (dir, repo) = create_test_repo();

    // First author commits line1 and line2
    let sig1 = git2::Signature::new(
        "Alice",
        "alice@test.com",
        &git2::Time::new(1_700_000_000, 0),
    )
    .unwrap();
    let mut index = repo.index().unwrap();
    let full_path = repo.workdir().unwrap().join("a.rs");
    fs::write(&full_path, "line1\nline2\n").unwrap();
    index.add_path(Path::new("a.rs")).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig1, &sig1, "alice adds", &tree, &[])
        .unwrap();

    // Second author modifies line1 but keeps line2
    let sig2 =
        git2::Signature::new("Bob", "bob@test.com", &git2::Time::new(1_700_001_000, 0)).unwrap();
    let mut index = repo.index().unwrap();
    fs::write(&full_path, "modified\nline2\n").unwrap();
    index.add_path(Path::new("a.rs")).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(
        Some("HEAD"),
        &sig2,
        &sig2,
        "bob modifies",
        &tree,
        &[&parent],
    )
    .unwrap();

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let blames = git_repo.blame_file(Path::new("a.rs")).unwrap();

    assert_eq!(blames.len(), 2, "two authors should produce 2 entries");
    let total_lines: usize = blames.iter().map(|b| b.lines).sum();
    assert_eq!(total_lines, 2, "total blamed lines should be 2");
}

#[test]
fn test_blame_nonexistent_file() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("a.rs", "content\n")], "add a");

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let result = git_repo.blame_file(Path::new("nonexistent.rs"));
    assert!(result.is_err(), "blame on missing file should fail");
}

#[test]
fn test_recent_authors() {
    let (dir, repo) = create_test_repo();
    make_commit_at(&repo, &[("a.rs", "v1")], "first", 1_000_000);
    make_commit_at(&repo, &[("b.rs", "v1")], "second", 2_000_000);

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let authors = git_repo.recent_authors(Some(1_500_000)).unwrap();
    assert!(
        authors.contains("test@test.com"),
        "should contain author of recent commit"
    );

    let all_authors = git_repo.recent_authors(None).unwrap();
    assert!(!all_authors.is_empty());
}

#[test]
fn test_empty_repo() {
    let (dir, _repo) = create_test_repo();
    let git_repo = GitRepo::open(dir.path()).unwrap();

    // Empty repo has no HEAD, revwalk.push_head() will fail
    let freqs = git_repo.file_frequencies(None);
    assert!(freqs.is_err() || freqs.unwrap().is_empty());

    let co = git_repo.co_changing_commits(None);
    assert!(co.is_err() || co.unwrap().is_empty());
}

fn change_for<'a>(changes: &'a [FileChange], path: &str) -> Option<&'a FileChange> {
    changes.iter().find(|c| c.path == Path::new(path))
}

#[test]
fn changes_since_in_workdir_classifies_changes() {
    let (dir, repo) = create_test_repo();
    let body = "fn body() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n    println!(\"{a} {b} {c}\");\n}\n";
    make_commit(
        &repo,
        &[
            ("kept.rs", "fn kept() {}\n"),
            ("edited.rs", "fn edited() {}\n"),
            ("moved.rs", body),
            ("gone.rs", "fn gone() {}\n"),
        ],
        "base",
    );
    let root = dir.path();
    fs::write(root.join("edited.rs"), "fn edited() { let x = 1; }\n").unwrap();
    fs::rename(root.join("moved.rs"), root.join("renamed.rs")).unwrap();
    fs::remove_file(root.join("gone.rs")).unwrap();
    fs::write(root.join("new.rs"), "fn new() {}\n").unwrap();

    let git_repo = GitRepo::open(root).unwrap();
    let changes = git_repo.changes_since_in_workdir("HEAD").unwrap();

    assert!(change_for(&changes, "kept.rs").is_none());
    assert!(change_for(&changes, "gone.rs").is_none());
    assert_eq!(
        change_for(&changes, "edited.rs")
            .unwrap()
            .old_path
            .as_deref(),
        Some(Path::new("edited.rs"))
    );
    assert_eq!(
        change_for(&changes, "renamed.rs")
            .unwrap()
            .old_path
            .as_deref(),
        Some(Path::new("moved.rs"))
    );
    assert!(change_for(&changes, "new.rs").unwrap().old_path.is_none());
}

#[test]
fn changes_since_in_workdir_rejects_unknown_ref() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("a.rs", "fn a() {}\n")], "base");
    let git_repo = GitRepo::open(dir.path()).unwrap();
    assert!(git_repo.changes_since_in_workdir("no-such-ref").is_err());
}

#[test]
fn diff_stats_classify_each_kind_of_change() {
    let (dir, repo) = create_test_repo();
    make_commit(
        &repo,
        &[
            ("modified.rs", "one\ntwo\n"),
            ("deleted.rs", "one\ntwo\nthree\n"),
            ("old_name.rs", "a\nb\nc\nd\ne\n"),
        ],
        "base",
    );
    fs::write(dir.path().join("modified.rs"), "one\nTWO\nthree\n").unwrap();
    fs::remove_file(dir.path().join("deleted.rs")).unwrap();
    fs::rename(
        dir.path().join("old_name.rs"),
        dir.path().join("new_name.rs"),
    )
    .unwrap();
    fs::write(dir.path().join("untracked.rs"), "x\ny\n").unwrap();
    fs::write(dir.path().join("image.bin"), [0u8, 159, 146, 150, 0, 1]).unwrap();

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let mut stats = git_repo.diff_stats_since("HEAD").unwrap();
    stats.sort_by(|a, b| a.path.cmp(&b.path));
    let summary: Vec<(&str, ChangeKind, Option<&str>, usize, usize)> = stats
        .iter()
        .map(|s| {
            (
                s.path.to_str().unwrap(),
                s.kind,
                s.old_path.as_deref().and_then(Path::to_str),
                s.added,
                s.deleted,
            )
        })
        .collect();

    assert_eq!(
        summary,
        [
            ("deleted.rs", ChangeKind::Deleted, Some("deleted.rs"), 0, 3),
            ("image.bin", ChangeKind::Added, None, 0, 0),
            (
                "modified.rs",
                ChangeKind::Modified,
                Some("modified.rs"),
                2,
                1
            ),
            (
                "new_name.rs",
                ChangeKind::Renamed,
                Some("old_name.rs"),
                0,
                0
            ),
            ("untracked.rs", ChangeKind::Added, None, 2, 0),
        ]
    );
}

#[test]
fn co_change_history_counts_only_the_targets() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("a.rs", "1"), ("b.rs", "1")], "both");
    make_commit(&repo, &[("a.rs", "2"), ("b.rs", "2"), ("c.rs", "2")], "all");
    make_commit(&repo, &[("a.rs", "3")], "alone");
    make_commit(&repo, &[("b.rs", "4"), ("c.rs", "4")], "others");

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let targets: HashSet<PathBuf> = [PathBuf::from("a.rs")].into();
    let history = git_repo
        .co_change_history((None, "HEAD"), None, &targets, 30)
        .unwrap();

    // The commit that touches a.rs alone counts towards its total.
    assert_eq!(history.commits, HashMap::from([(PathBuf::from("a.rs"), 3)]));
    assert_eq!(
        history.shared[Path::new("a.rs")],
        HashMap::from([(PathBuf::from("b.rs"), 2), (PathBuf::from("c.rs"), 1)])
    );
}

#[test]
fn co_change_history_stops_at_the_ref() {
    let (dir, repo) = create_test_repo();
    let base = make_commit(&repo, &[("a.rs", "1"), ("b.rs", "1")], "base");
    make_commit(&repo, &[("a.rs", "2"), ("b.rs", "2")], "after");

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let targets: HashSet<PathBuf> = [PathBuf::from("a.rs")].into();
    let history = git_repo
        .co_change_history((Some(&base.to_string()), "HEAD"), None, &targets, 30)
        .unwrap();

    assert_eq!(history.commits[Path::new("a.rs")], 1);
    assert_eq!(history.shared[Path::new("a.rs")][Path::new("b.rs")], 1);
}

#[test]
fn diff_stats_fail_on_an_unknown_ref() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("a.rs", "1")], "base");
    let git_repo = GitRepo::open(dir.path()).unwrap();
    let err = git_repo.diff_stats_since("nope").err().unwrap();
    assert!(err.to_string().contains("cannot resolve ref 'nope'"));
}

#[test]
fn diff_stats_count_no_lines_for_a_modified_binary_file() {
    let (dir, repo) = create_test_repo();
    let path = dir.path().join("image.bin");
    fs::write(&path, [0u8, 1, 2, 3, 0, 255]).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("image.bin")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig =
        git2::Signature::new("Test", "test@test.com", &git2::Time::new(1_700_000_000, 0)).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "binary", &tree, &[])
        .unwrap();
    fs::write(&path, [0u8, 9, 9, 9, 0, 255, 7]).unwrap();

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let stats = git_repo.diff_stats_since("HEAD").unwrap();

    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].kind, ChangeKind::Modified);
    assert_eq!((stats[0].added, stats[0].deleted), (0, 0));
}

#[test]
fn renames_are_detected_whatever_the_git_configuration() {
    let (dir, repo) = create_test_repo();
    repo.config()
        .unwrap()
        .set_bool("diff.renames", false)
        .unwrap();
    make_commit(&repo, &[("old_name.rs", "a\nb\nc\nd\ne\n")], "base");
    fs::rename(
        dir.path().join("old_name.rs"),
        dir.path().join("new_name.rs"),
    )
    .unwrap();

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let stats = git_repo.diff_stats_since("HEAD").unwrap();

    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].kind, ChangeKind::Renamed);
    assert_eq!(stats[0].old_path.as_deref(), Some(Path::new("old_name.rs")));
}

const PATCH_OF_EVERY_KIND: &str = "\
diff --git a/modified.rs b/modified.rs
index 1111111..2222222 100644
--- a/modified.rs
+++ b/modified.rs
@@ -1,2 +1,3 @@
 one
-two
+TWO
+three
diff --git a/added.rs b/added.rs
new file mode 100644
index 0000000..3333333
--- /dev/null
+++ b/added.rs
@@ -0,0 +1,2 @@
+x
+y
diff --git a/deleted.rs b/deleted.rs
deleted file mode 100644
index 4444444..0000000
--- a/deleted.rs
+++ /dev/null
@@ -1,3 +0,0 @@
-a
-b
-c
diff --git a/old_name.rs b/new_name.rs
similarity index 100%
rename from old_name.rs
rename to new_name.rs
diff --git a/logo.png b/logo.png
index 5555555..6666666 100644
Binary files a/logo.png and b/logo.png differ
";

#[test]
fn patch_stats_classify_each_kind_of_change() {
    let mut stats = patch_stats(PATCH_OF_EVERY_KIND.as_bytes()).unwrap();
    stats.sort_by(|a, b| a.path.cmp(&b.path));
    let summary: Vec<(&str, ChangeKind, Option<&str>, usize, usize)> = stats
        .iter()
        .map(|s| {
            (
                s.path.to_str().unwrap(),
                s.kind,
                s.old_path.as_deref().and_then(Path::to_str),
                s.added,
                s.deleted,
            )
        })
        .collect();

    assert_eq!(
        summary,
        [
            ("added.rs", ChangeKind::Added, None, 2, 0),
            ("deleted.rs", ChangeKind::Deleted, Some("deleted.rs"), 0, 3),
            ("logo.png", ChangeKind::Modified, Some("logo.png"), 0, 0),
            (
                "modified.rs",
                ChangeKind::Modified,
                Some("modified.rs"),
                2,
                1
            ),
            (
                "new_name.rs",
                ChangeKind::Renamed,
                Some("old_name.rs"),
                0,
                0
            ),
        ]
    );
}

#[test]
fn patch_stats_reject_what_is_not_a_patch() {
    let err = patch_stats(b"hello\n").err().unwrap().to_string();
    assert!(err.contains("cannot read the patch"), "{err}");
    assert!(patch_stats(b"").unwrap().is_empty());
}

#[test]
fn diff_stats_between_two_commits() {
    let (dir, repo) = create_test_repo();
    let base = make_commit(
        &repo,
        &[("kept.rs", "one\ntwo\n"), ("moved.rs", "a\nb\nc\nd\ne\n")],
        "base",
    );
    // Move a file and edit another in the next commit.
    fs::rename(dir.path().join("moved.rs"), dir.path().join("renamed.rs")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("moved.rs")).unwrap();
    index.write().unwrap();
    let tip = make_commit(
        &repo,
        &[("kept.rs", "one\nTWO\n"), ("renamed.rs", "a\nb\nc\nd\ne\n")],
        "tip",
    );
    // What is in the working tree afterwards does not count.
    fs::write(dir.path().join("kept.rs"), "scratch\n").unwrap();

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let mut stats = git_repo
        .diff_stats_between(&base.to_string(), &tip.to_string())
        .unwrap();
    stats.sort_by(|a, b| a.path.cmp(&b.path));

    assert_eq!(stats.len(), 2);
    assert_eq!(stats[0].path, PathBuf::from("kept.rs"));
    assert_eq!((stats[0].added, stats[0].deleted), (1, 1));
    assert_eq!(stats[1].kind, ChangeKind::Renamed);
    assert_eq!(stats[1].old_path.as_deref(), Some(Path::new("moved.rs")));

    // Nothing separates a commit from itself.
    let same = git_repo
        .diff_stats_between(&tip.to_string(), &tip.to_string())
        .unwrap();
    assert!(same.is_empty());
}

#[test]
fn commits_and_their_ancestry() {
    let (dir, repo) = create_test_repo();
    let first = make_commit(&repo, &[("a.rs", "1")], "first").to_string();
    let second = make_commit(&repo, &[("a.rs", "2")], "second").to_string();
    let git_repo = GitRepo::open(dir.path()).unwrap();

    assert!(git_repo.has_commit(&second));
    assert!(git_repo.has_commit("HEAD"));
    assert!(!git_repo.has_commit("0123456789012345678901234567890123456789"));
    assert!(!git_repo.has_commit("no-such-ref"));

    assert!(git_repo.is_ancestor(&first, &second));
    assert!(!git_repo.is_ancestor(&second, &first));
    assert!(!git_repo.is_ancestor(&first, &first));
    assert!(!git_repo.is_ancestor("no-such-ref", &second));
    assert!(!git_repo.is_ancestor(&first, "no-such-ref"));
}

#[test]
fn files_of_a_tree_are_read_without_checking_it_out() {
    let (dir, repo) = create_test_repo();
    let commit = make_commit(
        &repo,
        &[
            ("Cargo.toml", "root manifest"),
            ("src/main.rs", "fn main() {}"),
            ("crates/a/Cargo.toml", "a manifest"),
            ("target/pkg/Cargo.toml", "built"),
            ("crates/a/target/Cargo.toml", "built too"),
        ],
        "tree",
    );
    // The working tree moves on; the tree of the commit does not.
    fs::write(dir.path().join("Cargo.toml"), "edited").unwrap();
    fs::remove_file(dir.path().join("crates/a/Cargo.toml")).unwrap();

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let is_target = |p: &Path| p.file_name().is_some_and(|n| n == "target");
    let is_manifest = |p: &Path| p.file_name().is_some_and(|n| n == "Cargo.toml");
    let mut files = git_repo
        .files_at(&commit.to_string(), is_target, is_manifest)
        .unwrap();
    files.sort();

    assert_eq!(
        files,
        [
            (PathBuf::from("Cargo.toml"), "root manifest".to_string()),
            (
                PathBuf::from("crates/a/Cargo.toml"),
                "a manifest".to_string()
            ),
        ]
    );

    let at = |path: &str| git_repo.has_file_at(&commit.to_string(), Path::new(path));
    assert!(at("crates/a/Cargo.toml"));
    assert!(at("src/main.rs"));
    assert!(!at("src"));
    assert!(!at("missing.rs"));
    assert!(!git_repo.has_file_at("no-such-ref", Path::new("Cargo.toml")));
    assert!(
        git_repo
            .files_at("no-such-ref", is_target, is_manifest)
            .is_err()
    );
}

#[test]
fn files_of_a_tree_leave_out_what_is_not_text() {
    let (dir, repo) = create_test_repo();
    let path = dir.path().join("data.bin");
    fs::write(&path, [0u8, 159, 146, 150]).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("data.bin")).unwrap();
    index.write().unwrap();
    let commit = make_commit(&repo, &[("notes.txt", "text")], "mixed");

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let files = git_repo
        .files_at(&commit.to_string(), |_| false, |_| true)
        .unwrap();
    assert_eq!(files, [(PathBuf::from("notes.txt"), "text".to_string())]);
}

#[test]
fn diff_stats_tell_which_lines_a_change_touches() {
    let (dir, repo) = create_test_repo();
    let before: String = (1..=10).map(|n| format!("line {n}\n")).collect();
    make_commit(&repo, &[("a.txt", &before)], "base");
    // Edit line 2, delete lines 5 and 6, append two lines.
    let after = "line 1\nLINE 2\nline 3\nline 4\nline 7\nline 8\nline 9\nline 10\nnew 11\nnew 12\n";
    fs::write(dir.path().join("a.txt"), after).unwrap();
    fs::write(dir.path().join("b.txt"), "x\ny\n").unwrap();

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let stats = git_repo.diff_stats_since("HEAD").unwrap();
    let lines = |path: &str| {
        &stats
            .iter()
            .find(|s| s.path == Path::new(path))
            .unwrap()
            .lines
    };

    // Line 5 is the one that now stands where two were deleted.
    assert_eq!(lines("a.txt"), &[2, 5, 9, 10]);
    assert_eq!(lines("b.txt"), &[1, 2]);
}

#[test]
fn patch_stats_tell_which_lines_a_patch_touches() {
    let stats = patch_stats(PATCH_OF_EVERY_KIND.as_bytes()).unwrap();
    let lines = |path: &str| {
        &stats
            .iter()
            .find(|s| s.path == Path::new(path))
            .unwrap()
            .lines
    };
    assert_eq!(lines("modified.rs"), &[2, 3]);
    assert_eq!(lines("added.rs"), &[1, 2]);
    assert_eq!(lines("deleted.rs"), &[1]);
    assert!(lines("new_name.rs").is_empty());
    assert!(lines("logo.png").is_empty());
}

#[test]
fn the_lines_of_a_change_follow_the_text_they_are_looked_up_in() {
    let (dir, repo) = create_test_repo();
    let before = "one\ntwo\nthree\nfour\nfive\n";
    make_commit(&repo, &[("a.txt", before)], "base");
    // Insert two lines after the first one.
    let after = "one\nnew a\nnew b\ntwo\nthree\nfour\nfive\n";
    fs::write(dir.path().join("a.txt"), after).unwrap();

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let stat = &git_repo.diff_stats_since("HEAD").unwrap()[0];

    assert_eq!(stat.lines, [2, 3]);
    assert_eq!(stat.old_lines, [2]);
    assert_eq!(
        stat.probe,
        [(2, "new a".to_string()), (3, "new b".to_string())]
    );
    // The text after the change holds the added lines where the change says.
    assert_eq!(stat.lines_in(after), [2, 3]);
    // The text before it does not: the change touches it where it inserts.
    assert_eq!(stat.lines_in(before), [2]);
    assert_eq!(stat.lines_in(""), [2]);
}

#[test]
fn a_change_that_only_deletes_has_nothing_to_probe() {
    let (dir, repo) = create_test_repo();
    make_commit(&repo, &[("a.txt", "one\ntwo\nthree\n")], "base");
    fs::write(dir.path().join("a.txt"), "one\nthree\n").unwrap();

    let git_repo = GitRepo::open(dir.path()).unwrap();
    let stat = &git_repo.diff_stats_since("HEAD").unwrap()[0];

    assert!(stat.probe.is_empty());
    assert_eq!(stat.old_lines, [2]);
    // With nothing to tell the texts apart, the lines after the change are used.
    assert_eq!(stat.lines_in("one\ntwo\nthree\n"), [2]);
}
