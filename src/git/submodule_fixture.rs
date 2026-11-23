//! A superproject with an initialized submodule, built offline with libgit2
//! alone, for the tests of every module that reads a ref.

use std::fs;
use std::path::Path;

use git2::{FileMode, Oid, Repository, Signature, Time};
use tempfile::TempDir;

/// A new repository holding `files`, committed.
pub(crate) fn superproject(files: &[(&str, &str)]) -> (TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    commit_files(&repo, files, "init");
    (dir, repo)
}

/// Write `files` in the working tree of `repo` and commit them on HEAD.
fn commit_files(repo: &Repository, files: &[(&str, &str)], message: &str) -> Oid {
    let root = repo.workdir().unwrap();
    let mut index = repo.index().unwrap();
    for (name, content) in files {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
        index.add_path(Path::new(name)).unwrap();
    }
    index.write().unwrap();
    commit_index(repo, message)
}

/// Commit what the index of `repo` holds on HEAD.
fn commit_index(repo: &Repository, message: &str) -> Oid {
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    commit_tree(repo, &tree, message)
}

/// Commit `tree` on HEAD.
fn commit_tree(repo: &Repository, tree: &git2::Tree, message: &str) -> Oid {
    let sig = Signature::new("Test", "test@test.com", &Time::new(1_700_000_000, 0)).unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, tree, &parents)
        .unwrap()
}

/// Add a submodule at `path` holding `files`, committed inside it and in
/// `outer`. Returns the commit of the submodule.
pub(crate) fn add_submodule(outer: &Repository, path: &str, files: &[(&str, &str)]) -> Oid {
    let url = "https://example.invalid/never-fetched.git";
    let mut module = outer.submodule(url, Path::new(path), true).unwrap();
    let head = commit_files(&module.open().unwrap(), files, "submodule");
    module.add_finalize().unwrap();
    commit_index(outer, &format!("add {path}"));
    head
}

/// Commit `files` inside the submodule at `path` and record its new commit
/// in `outer`. Returns the commit of the submodule.
pub(crate) fn bump_submodule(outer: &Repository, path: &str, files: &[(&str, &str)]) -> Oid {
    let mut module = outer.find_submodule(path).unwrap();
    let head = commit_files(&module.open().unwrap(), files, "bump");
    module.add_to_index(true).unwrap();
    commit_index(outer, &format!("bump {path}"));
    head
}

/// Commit on HEAD a gitlink named `name`, at the top of the tree, to `id`.
pub(crate) fn commit_gitlink(outer: &Repository, name: &str, id: Oid, message: &str) -> Oid {
    let head = outer.head().unwrap().peel_to_tree().unwrap();
    let mut builder = outer.treebuilder(Some(&head)).unwrap();
    builder
        .insert(name, id, i32::from(FileMode::Commit))
        .unwrap();
    let tree = outer.find_tree(builder.write().unwrap()).unwrap();
    commit_tree(outer, &tree, message)
}

/// A commit that no repository holds.
pub(crate) fn absent_commit() -> Oid {
    Oid::from_str("8a9b697000000000000000000000000000000000").unwrap()
}
