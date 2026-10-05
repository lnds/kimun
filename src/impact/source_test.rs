use super::*;
use std::fs;

use git2::Repository;

fn create_test_repo() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    (dir, repo)
}

/// Commit `files` as written and `removed` as deletions; returns the commit id.
fn commit(repo: &Repository, files: &[(&str, &str)], removed: &[&str]) -> String {
    let sig =
        git2::Signature::new("Test", "test@test.com", &git2::Time::new(1_700_000_000, 0)).unwrap();
    let workdir = repo.workdir().unwrap();
    let mut index = repo.index().unwrap();
    for (path, content) in files {
        let full = workdir.join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, content).unwrap();
        index.add_path(Path::new(path)).unwrap();
    }
    for path in removed {
        fs::remove_file(workdir.join(path)).unwrap();
        index.remove_path(Path::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &parents)
        .unwrap()
        .to_string()
}

fn refs(since: &str, until: Option<&str>) -> DiffSource {
    DiffSource::Refs {
        since: since.to_string(),
        until: until.map(str::to_string),
    }
}

fn paths(change: &Change) -> Vec<&str> {
    let mut paths: Vec<&str> = change
        .diff
        .iter()
        .map(|c| c.path.to_str().unwrap())
        .collect();
    paths.sort();
    paths
}

fn roots(graph: &ProjectGraph) -> Vec<String> {
    let mut roots: Vec<String> = graph.projects.iter().map(|p| p.display_root()).collect();
    roots.sort();
    roots
}

const PATCH: &str = "\
diff --git a/src/a.rs b/src/a.rs
index 1111111..2222222 100644
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,2 +1,3 @@
 one
-two
+TWO
+three
diff --git a/new.rs b/new.rs
new file mode 100644
index 0000000..3333333
--- /dev/null
+++ b/new.rs
@@ -0,0 +1,2 @@
+x
+y
";

#[test]
fn against_a_ref_measures_up_to_the_working_tree() {
    let (dir, repo) = create_test_repo();
    commit(
        &repo,
        &[
            ("a.rs", "a\n"),
            ("Cargo.toml", "[package]\nname = \"app\"\n"),
        ],
        &[],
    );
    fs::write(dir.path().join("a.rs"), "changed\n").unwrap();
    fs::write(dir.path().join("untracked.rs"), "new\n").unwrap();
    let git_repo = GitRepo::open(dir.path()).unwrap();

    let change = Change::resolve(&git_repo, &refs("HEAD", None)).unwrap();

    assert_eq!(change.label, "diff against HEAD");
    assert_eq!(paths(&change), ["a.rs", "untracked.rs"]);
    assert!(change.has_file(&git_repo, Path::new("untracked.rs")));
    assert_eq!(roots(&change.graph(&git_repo).unwrap()), ["."]);
}

#[test]
fn between_two_refs_ignores_what_is_checked_out() {
    let (dir, repo) = create_test_repo();
    let base = commit(&repo, &[("a.rs", "a\n"), ("old.rs", "old\n")], &[]);
    let feature = commit(
        &repo,
        &[
            ("a.rs", "feature\n"),
            ("lib/Cargo.toml", "[package]\nname = \"lib\"\n"),
        ],
        &["old.rs"],
    );
    // Back at the base, with unrelated work in the tree.
    let base_commit = repo
        .find_commit(git2::Oid::from_str(&base).unwrap())
        .unwrap();
    repo.reset(base_commit.as_object(), git2::ResetType::Hard, None)
        .unwrap();
    fs::write(dir.path().join("scratch.rs"), "wip\n").unwrap();
    let git_repo = GitRepo::open(dir.path()).unwrap();

    let change = Change::resolve(&git_repo, &refs(&base, Some(&feature))).unwrap();

    assert_eq!(change.label, format!("diff {base}...{feature}"));
    assert_eq!(paths(&change), ["a.rs", "lib/Cargo.toml", "old.rs"]);
    // The state after the change is the tree of the feature commit.
    assert!(change.has_file(&git_repo, Path::new("lib/Cargo.toml")));
    assert!(!change.has_file(&git_repo, Path::new("old.rs")));
    assert!(!change.has_file(&git_repo, Path::new("scratch.rs")));
    assert!(!change.has_file(&git_repo, Path::new("lib")));
    assert_eq!(roots(&change.graph(&git_repo).unwrap()), ["lib"]);
}

#[test]
fn between_two_refs_starts_where_they_diverged() {
    let (_dir, repo) = create_test_repo();
    let base = commit(&repo, &[("a.rs", "a\n")], &[]);
    let upstream = commit(&repo, &[("upstream.rs", "u\n")], &[]);
    let base_commit = repo
        .find_commit(git2::Oid::from_str(&base).unwrap())
        .unwrap();
    repo.reset(base_commit.as_object(), git2::ResetType::Hard, None)
        .unwrap();
    let feature = commit(&repo, &[("feature.rs", "f\n")], &[]);
    let git_repo = GitRepo::open(repo.workdir().unwrap()).unwrap();

    let change = Change::resolve(&git_repo, &refs(&upstream, Some(&feature))).unwrap();

    // What upstream added since is not a deletion made by the feature.
    assert_eq!(paths(&change), ["feature.rs"]);
}

#[test]
fn a_patch_file_is_measured_against_the_working_tree() {
    let (dir, repo) = create_test_repo();
    commit(
        &repo,
        &[
            ("src/a.rs", "one\ntwo\n"),
            ("Cargo.toml", "[package]\nname = \"app\"\n"),
        ],
        &[],
    );
    let patch = dir.path().join("change.patch");
    fs::write(&patch, PATCH).unwrap();
    let git_repo = GitRepo::open(dir.path()).unwrap();

    let source = DiffSource::Patch {
        file: patch.clone(),
        base: None,
    };
    let change = Change::resolve(&git_repo, &source).unwrap();

    assert_eq!(change.label, format!("patch from {}", patch.display()));
    assert_eq!(paths(&change), ["new.rs", "src/a.rs"]);
    let a = change
        .diff
        .iter()
        .find(|c| c.path.ends_with("a.rs"))
        .unwrap();
    assert_eq!((a.added, a.deleted), (2, 1));
    // The patch is not applied: what it adds is not in the tree.
    assert!(!change.has_file(&git_repo, Path::new("new.rs")));
    assert!(change.has_file(&git_repo, Path::new("src/a.rs")));
    assert_eq!(roots(&change.graph(&git_repo).unwrap()), ["."]);
}

#[test]
fn a_missing_patch_file_is_an_error() {
    let (dir, repo) = create_test_repo();
    commit(&repo, &[("a.rs", "a\n")], &[]);
    let git_repo = GitRepo::open(dir.path()).unwrap();

    let source = DiffSource::Patch {
        file: dir.path().join("nope.patch"),
        base: None,
    };
    let err = Change::resolve(&git_repo, &source)
        .err()
        .unwrap()
        .to_string();
    assert!(err.contains("cannot read the patch"), "{err}");
}

#[test]
fn text_that_is_not_a_patch_is_an_error() {
    let (dir, repo) = create_test_repo();
    commit(&repo, &[("a.rs", "a\n")], &[]);
    let file = dir.path().join("notes.txt");
    fs::write(&file, "just some notes\n").unwrap();
    let git_repo = GitRepo::open(dir.path()).unwrap();

    let source = DiffSource::Patch { file, base: None };
    let err = Change::resolve(&git_repo, &source)
        .err()
        .unwrap()
        .to_string();
    assert!(err.contains("cannot read the patch"), "{err}");
}

#[test]
fn an_empty_patch_has_no_changes() {
    let (dir, repo) = create_test_repo();
    commit(&repo, &[("a.rs", "a\n")], &[]);
    let file = dir.path().join("empty.patch");
    fs::write(&file, "").unwrap();
    let git_repo = GitRepo::open(dir.path()).unwrap();

    let source = DiffSource::Patch { file, base: None };
    assert!(Change::resolve(&git_repo, &source).unwrap().diff.is_empty());
}

/// Commits a and b together `times` times.
fn co_change(repo: &Repository, times: usize) {
    for i in 0..times {
        let text = format!("v{i}\n");
        commit(repo, &[("a.rs", &text), ("b.rs", &text)], &[]);
    }
}

fn shared_with_a(change: &Change, git_repo: &GitRepo) -> usize {
    let targets: HashSet<PathBuf> = [PathBuf::from("a.rs")].into();
    let history = change.history(git_repo, None, &targets, 30).unwrap();
    history.commits.get(Path::new("a.rs")).copied().unwrap_or(0)
}

#[test]
fn history_of_a_patch_ends_at_head_or_where_the_base_diverged() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, 2);
    let base = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    co_change(&repo, 3);
    let patch = dir.path().join("p.patch");
    fs::write(&patch, PATCH).unwrap();
    let git_repo = GitRepo::open(dir.path()).unwrap();

    let at_head = DiffSource::Patch {
        file: patch.clone(),
        base: None,
    };
    let change = Change::resolve(&git_repo, &at_head).unwrap();
    assert_eq!(shared_with_a(&change, &git_repo), 5);

    let at_base = DiffSource::Patch {
        file: patch,
        base: Some(base),
    };
    let change = Change::resolve(&git_repo, &at_base).unwrap();
    assert_eq!(shared_with_a(&change, &git_repo), 2);
}

#[test]
fn history_between_two_refs_ends_where_they_diverged() {
    let (dir, repo) = create_test_repo();
    co_change(&repo, 2);
    let base = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    co_change(&repo, 3);
    let git_repo = GitRepo::open(dir.path()).unwrap();

    let change = Change::resolve(&git_repo, &refs(&base, Some("HEAD"))).unwrap();
    assert_eq!(shared_with_a(&change, &git_repo), 2);
}

#[cfg(unix)]
mod pull_requests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A program that answers `pr view` with `view` and `pr diff` with `diff`.
    fn fake_gh(dir: &Path, view: &str, diff: &str) -> String {
        fs::write(dir.join("view.json"), view).unwrap();
        fs::write(dir.join("pr.patch"), diff).unwrap();
        let program = dir.join("fake-gh");
        fs::write(
            &program,
            "#!/bin/sh\nhere=$(dirname \"$0\")\ncase \"$2\" in\n  view) cat \"$here/view.json\" ;;\n  diff) cat \"$here/pr.patch\" ;;\nesac\n",
        )
        .unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        program.to_str().unwrap().to_string()
    }

    fn view(base: &str, head: &str, merge: Option<&str>) -> String {
        let merge = merge.map_or("null".to_string(), |m| format!("{{\"oid\":\"{m}\"}}"));
        format!("{{\"baseRefOid\":\"{base}\",\"headRefOid\":\"{head}\",\"mergeCommit\":{merge}}}")
    }

    #[test]
    fn an_open_pull_request_is_measured_from_its_commits() {
        let (dir, repo) = create_test_repo();
        let base = commit(&repo, &[("a.rs", "a\n")], &[]);
        let head = commit(&repo, &[("a.rs", "pr\n"), ("pr.rs", "new\n")], &[]);
        let tools = tempfile::tempdir().unwrap();
        let gh = fake_gh(tools.path(), &view(&base, &head, None), "unused");
        let git_repo = GitRepo::open(dir.path()).unwrap();

        let change = Change::from_pull_request(&git_repo, &gh, 42).unwrap();

        assert_eq!(change.label, "PR #42");
        assert_eq!(paths(&change), ["a.rs", "pr.rs"]);
        assert!(change.has_file(&git_repo, Path::new("pr.rs")));
    }

    #[test]
    fn a_squash_merged_pull_request_is_measured_up_to_its_merge_commit() {
        let (dir, repo) = create_test_repo();
        let base = commit(&repo, &[("a.rs", "a\n")], &[]);
        // One commit on the base branch carries the whole change.
        let merge = commit(&repo, &[("a.rs", "merged\n"), ("b.rs", "new\n")], &[]);
        commit(&repo, &[("later.rs", "after the merge\n")], &[]);
        let tools = tempfile::tempdir().unwrap();
        // The head was never fetched.
        let head = "0123456789012345678901234567890123456789";
        let gh = fake_gh(tools.path(), &view(&base, head, Some(&merge)), "unused");
        let git_repo = GitRepo::open(dir.path()).unwrap();

        let change = Change::from_pull_request(&git_repo, &gh, 7).unwrap();

        assert_eq!(paths(&change), ["a.rs", "b.rs"]);
    }

    #[test]
    fn a_rebase_merged_pull_request_keeps_every_commit_of_it() {
        let (dir, repo) = create_test_repo();
        let base = commit(&repo, &[("a.rs", "a\n")], &[]);
        // Three commits replayed on the base branch; the last one is what
        // GitHub reports as the merge commit.
        commit(&repo, &[("one.rs", "1\n")], &[]);
        commit(&repo, &[("two.rs", "2\n")], &[]);
        let merge = commit(&repo, &[("three.rs", "3\n")], &[]);
        commit(&repo, &[("later.rs", "after the merge\n")], &[]);
        let tools = tempfile::tempdir().unwrap();
        let head = "0123456789012345678901234567890123456789";
        let gh = fake_gh(tools.path(), &view(&base, head, Some(&merge)), "unused");
        let git_repo = GitRepo::open(dir.path()).unwrap();

        let change = Change::from_pull_request(&git_repo, &gh, 8).unwrap();

        assert_eq!(paths(&change), ["one.rs", "three.rs", "two.rs"]);
    }

    #[test]
    fn a_pull_request_merged_with_a_merge_commit_is_measured_from_its_head() {
        let (dir, repo) = create_test_repo();
        let base = commit(&repo, &[("a.rs", "a\n")], &[]);
        let head = commit(&repo, &[("pr.rs", "new\n")], &[]);
        let merge = commit(&repo, &[("unrelated.rs", "resolved by hand\n")], &[]);
        let tools = tempfile::tempdir().unwrap();
        let gh = fake_gh(tools.path(), &view(&base, &head, Some(&merge)), "unused");
        let git_repo = GitRepo::open(dir.path()).unwrap();

        let change = Change::from_pull_request(&git_repo, &gh, 5).unwrap();

        assert_eq!(paths(&change), ["pr.rs"]);
    }

    #[test]
    fn a_pull_request_without_local_commits_is_measured_from_its_patch() {
        let (dir, repo) = create_test_repo();
        commit(&repo, &[("src/a.rs", "one\ntwo\n")], &[]);
        let tools = tempfile::tempdir().unwrap();
        let gh = fake_gh(tools.path(), &view("1111111", "2222222", None), PATCH);
        let git_repo = GitRepo::open(dir.path()).unwrap();

        let change = Change::from_pull_request(&git_repo, &gh, 9).unwrap();

        assert_eq!(change.label, "PR #9");
        assert_eq!(paths(&change), ["new.rs", "src/a.rs"]);
        // Measured against the working tree, where the patch is not applied.
        assert!(!change.has_file(&git_repo, Path::new("new.rs")));
    }

    #[test]
    fn a_missing_gh_stops_the_resolution() {
        let (dir, repo) = create_test_repo();
        commit(&repo, &[("a.rs", "a\n")], &[]);
        let git_repo = GitRepo::open(dir.path()).unwrap();

        let err = Change::from_pull_request(&git_repo, "kimun-no-such-gh", 1)
            .err()
            .unwrap()
            .to_string();
        assert!(err.contains("requires the GitHub CLI"), "{err}");
    }
}
