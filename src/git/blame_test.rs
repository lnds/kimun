use super::*;
use std::fs;

use git2::Repository;

fn repo() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    (dir, repo)
}

/// Commit `files` as `name` on top of HEAD; return the commit id.
fn commit(repo: &Repository, name: &str, files: &[(&str, &str)]) -> git2::Oid {
    let sig = git2::Signature::new(
        name,
        &format!("{name}@x.org"),
        &git2::Time::new(1_700_000_000, 0),
    )
    .unwrap();
    let mut index = repo.index().unwrap();
    for (path, content) in files {
        fs::write(repo.workdir().unwrap().join(path), content).unwrap();
        index.add_path(Path::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &parents)
        .unwrap()
}

fn functions(indent: &str) -> String {
    (1..=5)
        .map(|i| {
            format!("pub fn f{i}(x: u32) -> u32 {{\n{indent}if x > {i} {{\n{indent}{indent}return {i};\n{indent}}}\n{indent}x\n}}\n")
        })
        .collect()
}

/// Lines per author name.
fn owners(git: &GitRepo, path: &str) -> Vec<(String, usize)> {
    let mut owners: Vec<(String, usize)> = git
        .blame_file(Path::new(path))
        .unwrap()
        .into_iter()
        .map(|b| (b.author, b.lines))
        .collect();
    owners.sort();
    owners
}

#[test]
fn porcelain_lines_join_into_runs() {
    let text = "\
1111111111111111111111111111111111111111 1 1 2
author Ana
author-mail <ana@x.org>
author-time 100
summary a
filename lib.rs
\tline one
1111111111111111111111111111111111111111 2 2
author Ana
author-mail <ana@x.org>
author-time 100
\tline two
2222222222222222222222222222222222222222 5 3 1
author Bob
author-mail <bob@x.org>
author-time 200
\tline three
";
    let hunks = parse_porcelain(text);
    let runs: Vec<(&str, &str, i64, usize, usize)> = hunks
        .iter()
        .map(|h| {
            (
                h.author.as_str(),
                h.email.as_str(),
                h.time,
                h.start_line,
                h.lines,
            )
        })
        .collect();
    assert_eq!(
        runs,
        [
            ("Ana", "ana@x.org", 100, 1, 2),
            ("Bob", "bob@x.org", 200, 3, 1)
        ]
    );
}

#[test]
fn a_missing_program_falls_back_to_libgit2() {
    let (dir, repo) = repo();
    commit(&repo, "Ana", &[("a.rs", "1\n2\n")]);
    let git = GitRepo::open(dir.path()).unwrap();
    assert!(
        git.blame_with("no-such-git-program", Path::new("a.rs"))
            .is_none()
    );
    let lines: usize = git
        .blame_with_libgit2(Path::new("a.rs"))
        .unwrap()
        .iter()
        .map(|h| h.lines)
        .sum();
    assert_eq!(lines, 2);
}

#[test]
fn a_reformat_listed_in_ignore_revs_keeps_the_author() {
    let (dir, repo) = repo();
    commit(&repo, "Ana", &[("lib.rs", &functions(""))]);
    let format = commit(&repo, "Fmt", &[("lib.rs", &functions("    "))]);
    commit(
        &repo,
        "Ana",
        &[(".git-blame-ignore-revs", &format!("{format}\n"))],
    );
    let git = GitRepo::open(dir.path()).unwrap();
    assert_eq!(owners(&git, "lib.rs"), [("Ana".to_string(), 30)]);
}

#[test]
fn code_moved_between_files_keeps_its_author() {
    let (dir, repo) = repo();
    commit(
        &repo,
        "Ana",
        &[
            ("base.rs", &functions("    ")),
            ("core.rs", "pub fn keep() {}\n"),
        ],
    );
    let moved = format!("pub fn keep() {{}}\n{}", functions("    "));
    commit(
        &repo,
        "Bob",
        &[("base.rs", "pub fn keep2() {}\n"), ("core.rs", &moved)],
    );
    let git = GitRepo::open(dir.path()).unwrap();
    assert_eq!(owners(&git, "core.rs"), [("Ana".to_string(), 31)]);
}

#[test]
fn blame_reads_the_committed_file() {
    let (dir, repo) = repo();
    commit(&repo, "Ana", &[("a.rs", "1\n2\n")]);
    fs::write(dir.path().join("a.rs"), "1\n2\n3\n").unwrap();
    let git = GitRepo::open(dir.path()).unwrap();
    assert_eq!(owners(&git, "a.rs"), [("Ana".to_string(), 2)]);
}

/// Commit `files` as `name` at `time` on top of HEAD.
fn commit_at(repo: &Repository, name: &str, time: i64, files: &[(&str, &str)]) {
    let sig =
        git2::Signature::new(name, &format!("{name}@x.org"), &git2::Time::new(time, 0)).unwrap();
    let mut index = repo.index().unwrap();
    for (path, content) in files {
        fs::write(repo.workdir().unwrap().join(path), content).unwrap();
        index.add_path(Path::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &parents)
        .unwrap();
}

fn first_author(git: &GitRepo, path: &str) -> String {
    git.blame_file(Path::new(path)).unwrap()[0].author.clone()
}

#[test]
fn a_tie_in_lines_goes_to_the_most_recent_author() {
    let (dir, repo) = repo();
    commit_at(&repo, "Bob", 1_000, &[("a.rs", "1\n2\n")]);
    commit_at(&repo, "Ana", 2_000, &[("a.rs", "1\n2\n3\n4\n")]);
    commit_at(&repo, "Ana", 1_000, &[("b.rs", "1\n2\n")]);
    commit_at(&repo, "Bob", 2_000, &[("b.rs", "1\n2\n3\n4\n")]);
    let git = GitRepo::open(dir.path()).unwrap();
    for _ in 0..5 {
        assert_eq!(first_author(&git, "a.rs"), "Ana");
        assert_eq!(first_author(&git, "b.rs"), "Bob");
    }
}

#[test]
fn a_tie_in_lines_and_time_goes_to_the_first_identity() {
    let (dir, repo) = repo();
    commit_at(&repo, "Bob", 1_000, &[("a.rs", "1\n2\n")]);
    commit_at(&repo, "Ana", 1_000, &[("a.rs", "1\n2\n3\n4\n")]);
    let git = GitRepo::open(dir.path()).unwrap();
    for _ in 0..5 {
        assert_eq!(first_author(&git, "a.rs"), "Ana");
    }
}

#[test]
fn blame_since_counts_the_lines_committed_since() {
    let (dir, repo) = repo();
    commit_at(&repo, "Ana", 1_000, &[("a.rs", "1\n2\n3\n")]);
    commit_at(&repo, "Ana", 3_000, &[("a.rs", "1\n2\n3\n4\n")]);
    commit_at(&repo, "Bob", 1_000, &[("b.rs", "1\n")]);
    let git = GitRepo::open(dir.path()).unwrap();
    let recent = git
        .blame_file_since(Path::new("a.rs"), Some(2_000))
        .unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!((recent[0].author.as_str(), recent[0].lines), ("Ana", 1));
    let none = git
        .blame_file_since(Path::new("b.rs"), Some(2_000))
        .unwrap();
    assert!(none.is_empty());
}

#[test]
fn a_malformed_ignore_revs_file_still_blames_through_git() {
    let (dir, repo) = repo();
    commit(&repo, "Ana", &[("a.rs", "1\n2\n")]);
    commit(
        &repo,
        "Ana",
        &[(".git-blame-ignore-revs", "not a revision\n")],
    );
    let git = GitRepo::open(dir.path()).unwrap();
    let lines: usize = git
        .blame_with("git", Path::new("a.rs"))
        .expect("blamed without the revisions to ignore")
        .iter()
        .map(|h| h.lines)
        .sum();
    assert_eq!(lines, 2);
}
