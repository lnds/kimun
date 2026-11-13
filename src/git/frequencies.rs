//! How often and when each file changed, following renames: the commits
//! of a file before it was moved count for it under its name at HEAD, and
//! a move that leaves the content as it was is no change.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use git2::{Delta, DiffFindOptions, DiffOptions};

use super::{FileFrequency, GitRepo};

/// The name at HEAD of each path a file had before a rename, filled as the
/// history is walked newest first.
#[derive(Default)]
struct Renames {
    current: HashMap<PathBuf, PathBuf>,
}

impl Renames {
    fn name(&self, path: &Path) -> PathBuf {
        self.current
            .get(path)
            .cloned()
            .unwrap_or_else(|| path.to_path_buf())
    }
}

impl FileFrequency {
    fn count(&mut self, time: i64) {
        self.commits += 1;
        self.first_commit = self.first_commit.min(time);
        self.last_commit = self.last_commit.max(time);
    }
}

impl GitRepo {
    /// The files a commit changed, by their name at HEAD. Renames found in
    /// it are recorded, so the older commits of a moved file count for it.
    fn changed_following(
        &self,
        commit: &git2::Commit,
        renames: &mut Renames,
    ) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        let parent_tree = match commit.parent_count() {
            0 => None,
            _ => Some(commit.parent(0)?.tree()?),
        };
        let mut opts = DiffOptions::new();
        let mut diff = self.repo.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&commit.tree()?),
            Some(&mut opts),
        )?;
        diff.find_similar(Some(DiffFindOptions::new().renames(true)))?;

        let mut paths = Vec::new();
        let mut moves = Vec::new();
        for delta in diff.deltas() {
            let Some(path) = delta.new_file().path() else {
                continue;
            };
            let name = renames.name(path);
            if delta.status() == Delta::Renamed {
                if let Some(old) = delta.old_file().path() {
                    moves.push((old.to_path_buf(), name.clone()));
                }
                if delta.old_file().id() == delta.new_file().id() {
                    continue;
                }
            }
            paths.push(name);
        }
        renames.current.extend(moves);
        Ok(paths)
    }

    /// Count how many commits touched each file, with first/last timestamps.
    pub fn file_frequencies(
        &self,
        since: Option<i64>,
    ) -> Result<Vec<FileFrequency>, Box<dyn Error>> {
        let mut map: HashMap<PathBuf, FileFrequency> = HashMap::new();
        let mut renames = Renames::default();

        self.walk_commits(since, |commit| {
            let time = commit.time().seconds();
            for path in self.changed_following(commit, &mut renames)? {
                map.entry(path.clone())
                    .or_insert(FileFrequency {
                        path,
                        commits: 0,
                        first_commit: time,
                        last_commit: time,
                    })
                    .count(time);
            }
            Ok(ControlFlow::Continue(()))
        })?;

        let mut result: Vec<FileFrequency> = map.into_values().collect();
        result.sort_by(|a, b| b.commits.cmp(&a.commits).then_with(|| a.path.cmp(&b.path)));
        Ok(result)
    }

    /// Return the most recent commit timestamp for each file in `targets`.
    ///
    /// Walks commits newest-first; the first time a target file appears is its
    /// last-modification time. Stops early once all targets have been resolved.
    /// Files not touched by any commit (e.g. untracked) are omitted from the result.
    pub fn last_modified_per_file(
        &self,
        targets: &[PathBuf],
    ) -> Result<HashMap<PathBuf, i64>, Box<dyn Error>> {
        let mut remaining: HashSet<&PathBuf> = targets.iter().collect();
        let mut result: HashMap<PathBuf, i64> = HashMap::new();
        let mut renames = Renames::default();

        self.walk_commits(None, |commit| {
            if remaining.is_empty() {
                return Ok(ControlFlow::Break(()));
            }
            let time = commit.time().seconds();
            for path in self.changed_following(commit, &mut renames)? {
                if remaining.remove(&path) {
                    result.insert(path, time);
                }
            }
            Ok(ControlFlow::Continue(()))
        })?;

        Ok(result)
    }
}
