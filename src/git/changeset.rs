//! The change set between a ref and the working tree, and the history that
//! precedes it.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use git2::{Delta, Diff, DiffDelta, DiffFindOptions, DiffOptions, Patch, Tree};

use super::GitRepo;

/// How a file differs between a ref and the working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// A changed file with its line counts. Paths are repo-relative.
pub struct FileDiffStat {
    /// Current path, or the last known one for a deleted file.
    pub path: PathBuf,
    /// Where the file lived at the ref, or `None` if it is new.
    pub old_path: Option<PathBuf>,
    pub kind: ChangeKind,
    pub added: usize,
    pub deleted: usize,
}

/// How often some files changed, and what changed along with them.
#[derive(Default)]
pub struct CoChangeHistory {
    /// Commits that touched each target file.
    pub commits: HashMap<PathBuf, usize>,
    /// For each target file, the other files of its commits and how many
    /// commits they share.
    pub shared: HashMap<PathBuf, HashMap<PathBuf, usize>>,
    /// Commits of the targets left out for touching more files than the limit.
    pub skipped_commits: usize,
}

impl CoChangeHistory {
    /// Count one commit, given the files it touched, for each target among them.
    fn record(&mut self, paths: &[PathBuf], targets: &HashSet<PathBuf>) {
        for target in paths.iter().filter(|p| targets.contains(*p)) {
            *self.commits.entry(target.clone()).or_default() += 1;
            let shared = self.shared.entry(target.clone()).or_default();
            for other in paths.iter().filter(|p| *p != target) {
                *shared.entry(other.clone()).or_default() += 1;
            }
        }
    }
}

/// The kind of a delta, its path and its path at the ref. `None` for a
/// status that is not a change to a file (ignored, unreadable, conflicted).
fn classify(delta: &DiffDelta) -> Option<(ChangeKind, PathBuf, Option<PathBuf>)> {
    let old = delta.old_file().path().map(Path::to_path_buf);
    let new = delta.new_file().path().map(Path::to_path_buf);
    let (kind, path, old_path) = match delta.status() {
        Delta::Added | Delta::Untracked | Delta::Copied => (ChangeKind::Added, new, None),
        Delta::Modified | Delta::Typechange => (ChangeKind::Modified, new, old),
        Delta::Renamed => (ChangeKind::Renamed, new, old),
        Delta::Deleted => (ChangeKind::Deleted, old.clone(), old),
        _ => return None,
    };
    Some((kind, path?, old_path))
}

/// Lines added and deleted by the delta at `idx`. A binary file has no patch.
fn line_counts(diff: &Diff, idx: usize) -> Result<(usize, usize), Box<dyn Error>> {
    let Some(patch) = Patch::from_diff(diff, idx)? else {
        return Ok((0, 0));
    };
    let (_, added, deleted) = patch.line_stats()?;
    Ok((added, deleted))
}

impl GitRepo {
    /// Diff `tree` against the working tree, with renames detected. Untracked
    /// files are included; their lines only when `untracked_content` is set.
    pub(super) fn workdir_diff(
        &self,
        tree: &Tree,
        untracked_content: bool,
    ) -> Result<Diff<'_>, Box<dyn Error>> {
        let mut opts = DiffOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .show_untracked_content(untracked_content);
        let mut diff = self
            .repo
            .diff_tree_to_workdir_with_index(Some(tree), Some(&mut opts))?;
        let mut find = DiffFindOptions::new();
        find.renames(true).for_untracked(true);
        diff.find_similar(Some(&mut find))?;
        Ok(diff)
    }

    /// The commit where HEAD and `refspec` diverged. It is `refspec` itself
    /// when HEAD descends from it.
    fn merge_base(&self, refspec: &str) -> Result<git2::Commit<'_>, Box<dyn Error>> {
        let target = self
            .repo
            .revparse_single(refspec)
            .map_err(|e| format!("cannot resolve ref '{refspec}': {e}"))?
            .peel_to_commit()
            .map_err(|e| format!("'{refspec}' is not a commit: {e}"))?;
        let head = self.repo.head()?.peel_to_commit()?;
        let base = self
            .repo
            .merge_base(target.id(), head.id())
            .map_err(|e| format!("no common ancestor between HEAD and '{refspec}': {e}"))?;
        Ok(self.repo.find_commit(base)?)
    }

    /// Files that differ between the merge base with `refspec` and the working
    /// tree, with lines added and deleted. Covers uncommitted and untracked
    /// changes, and deletions. A binary file counts no lines.
    pub fn diff_stats_since(&self, refspec: &str) -> Result<Vec<FileDiffStat>, Box<dyn Error>> {
        let tree = self.merge_base(refspec)?.tree()?;
        let diff = self.workdir_diff(&tree, true)?;

        let mut stats = Vec::new();
        for (idx, delta) in diff.deltas().enumerate() {
            let Some((kind, path, old_path)) = classify(&delta) else {
                continue;
            };
            let (added, deleted) = line_counts(&diff, idx)?;
            stats.push(FileDiffStat {
                path,
                old_path,
                kind,
                added,
                deleted,
            });
        }
        Ok(stats)
    }

    /// Commit counts of `targets` and the files that changed with each of
    /// them, over the history up to the merge base with `refspec`. The commits
    /// made after it are the change under analysis and are left out.
    ///
    /// A commit touching more than `max_files` files is skipped altogether: a
    /// sweeping change (a reformat, a rename across the project) relates its
    /// files to each other by accident.
    pub fn co_change_history(
        &self,
        refspec: &str,
        since: Option<i64>,
        targets: &HashSet<PathBuf>,
        max_files: usize,
    ) -> Result<CoChangeHistory, Box<dyn Error>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(self.merge_base(refspec)?.id())?;
        let mut history = CoChangeHistory::default();

        self.walk(revwalk, since, |commit| {
            let paths = self.changed_files(commit)?;
            if paths.len() <= max_files {
                history.record(&paths, targets);
            } else if paths.iter().any(|p| targets.contains(p)) {
                history.skipped_commits += 1;
            }
            Ok(ControlFlow::Continue(()))
        })?;

        Ok(history)
    }

    /// The working tree directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Whether `rel_path` is a file in the working tree.
    pub fn has_file(&self, rel_path: &Path) -> bool {
        self.root.join(rel_path).is_file()
    }
}
