//! Git repository access via libgit2.
//!
//! Provides file change frequencies, co-changing commit analysis,
//! git blame for ownership, and recent author detection — all used
//! by the hotspots, knowledge, and temporal coupling modules.
//! The `GitRepo` wrapper encapsulates `git2::Repository` and its
//! resolved working directory root, providing a safe API for walking
//! commits, diffing trees, and resolving paths between the filesystem
//! walk and git's path namespace.
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use git2::{BlameOptions, DiffOptions, ObjectType, Repository, Sort, Tree};

/// Wrapper around a `git2::Repository` with its resolved root path.
pub struct GitRepo {
    repo: Repository,
    root: PathBuf,
}

/// How often a file was changed in git history.
pub struct FileFrequency {
    /// Repository-relative file path.
    pub path: PathBuf,
    /// Number of non-merge commits that touched this file.
    pub commits: usize,
    /// Unix timestamp of the earliest commit touching this file.
    pub first_commit: i64,
    /// Unix timestamp of the most recent commit touching this file.
    pub last_commit: i64,
}

/// Per-author blame contribution for a single file.
pub struct BlameInfo {
    /// Author display name from git signature.
    pub author: String,
    /// Author email from git signature.
    pub email: String,
    /// Number of lines attributed to this author.
    pub lines: usize,
    /// Unix timestamp of this author's most recent commit to the file.
    pub last_commit_time: i64,
}

impl GitRepo {
    /// Open the git repository that contains `path`.
    pub fn open(path: &Path) -> Result<Self, Box<dyn Error>> {
        let repo = Repository::discover(path)?;
        let root = repo
            .workdir()
            .ok_or("bare repositories are not supported")?
            .to_path_buf();
        Ok(Self { repo, root })
    }

    /// Iterate non-merge commits in reverse chronological order, optionally
    /// filtered by a `since` timestamp. Calls `f` for each qualifying commit.
    /// The callback returns `ControlFlow::Continue(())` to keep walking or
    /// `ControlFlow::Break(())` to stop early.
    fn walk_commits(
        &self,
        since: Option<i64>,
        mut f: impl FnMut(&git2::Commit) -> Result<ControlFlow<()>, Box<dyn Error>>,
    ) -> Result<(), Box<dyn Error>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        for oid in revwalk {
            let commit = self.repo.find_commit(oid?)?;
            // Skip merge commits — they don't represent individual file changes.
            if commit.parent_count() > 1 {
                continue;
            }
            // Commits are in reverse chronological order; once we hit one
            // older than the cutoff, all remaining commits are older too.
            if since.is_some_and(|ts| commit.time().seconds() < ts) {
                break;
            }
            if f(&commit)?.is_break() {
                break;
            }
        }
        Ok(())
    }

    /// Count how many commits touched each file, with first/last timestamps.
    pub fn file_frequencies(
        &self,
        since: Option<i64>,
    ) -> Result<Vec<FileFrequency>, Box<dyn Error>> {
        let mut map: HashMap<PathBuf, FileFrequency> = HashMap::new();

        self.walk_commits(since, |commit| {
            let time = commit.time().seconds();
            let paths = self.changed_files(commit)?;
            for path in paths {
                map.entry(path.clone())
                    .and_modify(|f| {
                        f.commits += 1;
                        if time < f.first_commit {
                            f.first_commit = time;
                        }
                        if time > f.last_commit {
                            f.last_commit = time;
                        }
                    })
                    .or_insert(FileFrequency {
                        path,
                        commits: 1,
                        first_commit: time,
                        last_commit: time,
                    });
            }
            Ok(ControlFlow::Continue(()))
        })?;

        let mut result: Vec<FileFrequency> = map.into_values().collect();
        result.sort_by(|a, b| b.commits.cmp(&a.commits));
        Ok(result)
    }

    /// Collect groups of files that changed together in each commit.
    /// Only includes commits that touch 2+ files.
    pub fn co_changing_commits(
        &self,
        since: Option<i64>,
    ) -> Result<Vec<Vec<PathBuf>>, Box<dyn Error>> {
        let mut result = Vec::new();

        self.walk_commits(since, |commit| {
            let paths = self.changed_files(commit)?;
            if paths.len() >= 2 {
                result.push(paths);
            }
            Ok(ControlFlow::Continue(()))
        })?;

        Ok(result)
    }

    /// Run git blame on a file and return per-author contributions.
    /// `rel_path` is relative to the git root.
    pub fn blame_file(&self, rel_path: &Path) -> Result<Vec<BlameInfo>, Box<dyn Error>> {
        let mut opts = BlameOptions::new();
        let blame = self.repo.blame_file(rel_path, Some(&mut opts))?;

        let mut map: HashMap<String, BlameInfo> = HashMap::new();

        for hunk in blame.iter() {
            let sig = hunk.final_signature();
            let email = sig.email().unwrap_or("unknown").to_string();
            let author = sig.name().unwrap_or("unknown").to_string();
            // Use the signature timestamp directly — avoids an O(1) git
            // object lookup per hunk that would otherwise be O(N) total.
            let commit_time = sig.when().seconds();
            let lines = hunk.lines_in_hunk();

            // Use name+email as key to avoid collisions when multiple
            // authors share the same "unknown" email.
            let key = format!("{author} <{email}>");
            map.entry(key)
                .and_modify(|info| {
                    info.lines += lines;
                    if commit_time > info.last_commit_time {
                        info.last_commit_time = commit_time;
                    }
                })
                .or_insert(BlameInfo {
                    author,
                    email,
                    lines,
                    last_commit_time: commit_time,
                });
        }

        let mut result: Vec<BlameInfo> = map.into_values().collect();
        result.sort_by(|a, b| b.lines.cmp(&a.lines));
        Ok(result)
    }

    /// Collect authors who have commits since the given timestamp.
    pub fn recent_authors(&self, since: Option<i64>) -> Result<HashSet<String>, Box<dyn Error>> {
        let mut authors = HashSet::new();

        self.walk_commits(since, |commit| {
            if let Some(email) = commit.author().email() {
                authors.insert(email.to_string());
            }
            Ok(ControlFlow::Continue(()))
        })?;

        Ok(authors)
    }

    /// Canonicalize the git root and walk root, then compute the relative prefix
    /// that maps walk-relative paths to git-relative paths.
    ///
    /// Returns `(canonical_walk_root, prefix)`. For example:
    ///   - `git_root=/a/b`, `walk_root=/a/b/src` → prefix = `"src"`
    ///   - `git_root=/a/b`, `walk_root=/a/b`     → prefix = `""`
    pub fn walk_prefix(&self, walk_root: &Path) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
        let git_root = self
            .root
            .canonicalize()
            .map_err(|e| format!("cannot resolve git root: {e}"))?;
        let canonical_walk = walk_root
            .canonicalize()
            .map_err(|e| format!("cannot resolve target path {}: {e}", walk_root.display()))?;
        let prefix = canonical_walk
            .strip_prefix(&git_root)
            .unwrap_or(Path::new(""))
            .to_path_buf();
        Ok((canonical_walk, prefix))
    }

    /// Convert a file path from walk-relative to git-relative using a
    /// pre-computed prefix from [`walk_prefix`].
    pub fn to_git_path(walk_root: &Path, prefix: &Path, file_path: &Path) -> PathBuf {
        let rel = file_path.strip_prefix(walk_root).unwrap_or(file_path);
        if prefix.as_os_str().is_empty() {
            rel.to_path_buf()
        } else {
            prefix.join(rel)
        }
    }

    /// Extract the file tree at a given git ref (e.g. "HEAD", "main~3") into
    /// a destination directory. Writes blobs as files and recurses into subtrees.
    /// Skips submodules and symlinks.
    pub fn extract_tree_to_dir(&self, refspec: &str, dest: &Path) -> Result<(), Box<dyn Error>> {
        let obj = self
            .repo
            .revparse_single(refspec)
            .map_err(|e| format!("cannot resolve ref '{refspec}': {e}"))?;
        let commit = obj
            .peel_to_commit()
            .map_err(|e| format!("'{refspec}' is not a commit: {e}"))?;
        let tree = commit.tree()?;
        self.write_tree_recursive(&tree, dest)
    }

    /// Recursively write a git tree to a filesystem directory.
    fn write_tree_recursive(&self, tree: &Tree, dest: &Path) -> Result<(), Box<dyn Error>> {
        for entry in tree.iter() {
            let name = entry
                .name()
                .ok_or_else(|| format!("non-UTF-8 entry in tree: {:?}", entry.id()))?;
            let path = dest.join(name);

            match entry.kind() {
                Some(ObjectType::Blob) => {
                    let blob = self.repo.find_blob(entry.id())?;
                    fs::write(&path, blob.content())?;
                }
                Some(ObjectType::Tree) => {
                    let subtree = self.repo.find_tree(entry.id())?;
                    fs::create_dir_all(&path)?;
                    self.write_tree_recursive(&subtree, &path)?;
                }
                _ => {} // skip submodules, symlinks, etc.
            }
        }
        Ok(())
    }

    /// Diff a commit against its parent to get the list of changed file paths.
    fn changed_files(&self, commit: &git2::Commit) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let mut opts = DiffOptions::new();
        let diff =
            self.repo
                .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?;

        let mut paths = Vec::new();
        for delta in diff.deltas() {
            if let Some(path) = delta.new_file().path() {
                paths.push(path.to_path_buf());
            }
        }
        Ok(paths)
    }
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
