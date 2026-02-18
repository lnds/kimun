/// Git repository access via libgit2.
///
/// Provides file change frequencies, co-changing commit analysis,
/// git blame for ownership, and recent author detection — all used
/// by the hotspots, knowledge, and temporal coupling modules.
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use git2::{BlameOptions, DiffOptions, Repository, Sort};

/// Wrapper around a `git2::Repository` with its resolved root path.
pub struct GitRepo {
    repo: Repository,
    root: PathBuf,
}

/// How often a file was changed in git history.
pub struct FileFrequency {
    pub path: PathBuf,
    pub commits: usize,
    pub first_commit: i64,
    pub last_commit: i64,
}

/// Per-author blame contribution for a single file.
pub struct BlameInfo {
    pub author: String,
    pub email: String,
    pub lines: usize,
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

    /// Return the working directory root of the repository.
    pub fn root(&self) -> &Path {
        &self.root
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
