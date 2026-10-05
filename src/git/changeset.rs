//! The change set between a ref and the working tree, and the history that
//! precedes it.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use git2::{Delta, Diff, DiffDelta, DiffFindOptions, DiffOptions, Patch, Tree};

use super::GitRepo;
use super::touched::Touched;

/// How a file differs between a ref and the working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// A changed file with its line counts. Paths are repo-relative.
#[derive(Debug, Clone, PartialEq)]
pub struct FileDiffStat {
    /// Current path, or the last known one for a deleted file.
    pub path: PathBuf,
    /// Where the file lived at the ref, or `None` if it is new.
    pub old_path: Option<PathBuf>,
    pub kind: ChangeKind,
    pub added: usize,
    pub deleted: usize,
    /// Lines of the file, as it is after the change, that the change
    /// touches: the ones it adds, and where it deletes. From 1, sorted.
    pub lines: Vec<usize>,
    /// The same, in the file as it was before the change.
    pub old_lines: Vec<usize>,
    /// A few of the lines the change adds, with their number: enough to
    /// tell whether a given text already holds the change.
    pub probe: Vec<(usize, String)>,
}

impl FileDiffStat {
    /// The lines the change touches in `text`, which is the file either
    /// after the change or before it. A patch need not be applied where it
    /// is measured; the lines it adds tell which of the two `text` is.
    pub fn lines_in(&self, text: &str) -> &[usize] {
        let lines: Vec<&str> = text.lines().collect();
        let applied = self
            .probe
            .iter()
            .all(|(number, added)| lines.get(number - 1).is_some_and(|l| l.trim_end() == added));
        if applied {
            &self.lines
        } else {
            &self.old_lines
        }
    }
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

/// Lines added and deleted by the delta at `idx`, and where. A binary file
/// has no patch.
fn line_counts(diff: &Diff, idx: usize) -> Result<(usize, usize, Touched), Box<dyn Error>> {
    let Some(patch) = Patch::from_diff(diff, idx)? else {
        return Ok((0, 0, Touched::default()));
    };
    let (_, added, deleted) = patch.line_stats()?;
    Ok((added, deleted, Touched::of(&patch)?))
}

/// The changed files of a diff, with their line counts.
fn stats_of(diff: &Diff) -> Result<Vec<FileDiffStat>, Box<dyn Error>> {
    let mut stats = Vec::new();
    for (idx, delta) in diff.deltas().enumerate() {
        let Some((kind, path, old_path)) = classify(&delta) else {
            continue;
        };
        let (added, deleted, touched) = line_counts(diff, idx)?;
        stats.push(FileDiffStat {
            path,
            old_path,
            kind,
            added,
            deleted,
            lines: touched.new,
            old_lines: touched.old,
            probe: touched.probe,
        });
    }
    Ok(stats)
}

/// The changed files of a patch in git format, as `git diff` and
/// `gh pr diff` print it.
pub fn patch_stats(patch: &[u8]) -> Result<Vec<FileDiffStat>, Box<dyn Error>> {
    let diff = Diff::from_buffer(patch).map_err(|e| format!("cannot read the patch: {e}"))?;
    stats_of(&diff)
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

    fn commit_of(&self, refspec: &str) -> Result<git2::Commit<'_>, Box<dyn Error>> {
        Ok(self
            .repo
            .revparse_single(refspec)
            .map_err(|e| format!("cannot resolve ref '{refspec}': {e}"))?
            .peel_to_commit()
            .map_err(|e| format!("'{refspec}' is not a commit: {e}"))?)
    }

    /// The commit where `a` and `b` diverged. It is `a` itself when `b`
    /// descends from it.
    fn merge_base(&self, a: &str, b: &str) -> Result<git2::Commit<'_>, Box<dyn Error>> {
        let base = self
            .repo
            .merge_base(self.commit_of(a)?.id(), self.commit_of(b)?.id())
            .map_err(|e| format!("no common ancestor between '{a}' and '{b}': {e}"))?;
        Ok(self.repo.find_commit(base)?)
    }

    /// Files that differ between the merge base with `refspec` and the working
    /// tree, with lines added and deleted. Covers uncommitted and untracked
    /// changes, and deletions. A binary file counts no lines.
    pub fn diff_stats_since(&self, refspec: &str) -> Result<Vec<FileDiffStat>, Box<dyn Error>> {
        let tree = self.merge_base(refspec, "HEAD")?.tree()?;
        stats_of(&self.workdir_diff(&tree, true)?)
    }

    /// Files that differ between the merge base of `since` and `until` and
    /// the tree of `until`: what `until` brings, whatever is checked out.
    pub fn diff_stats_between(
        &self,
        since: &str,
        until: &str,
    ) -> Result<Vec<FileDiffStat>, Box<dyn Error>> {
        let base = self.merge_base(since, until)?.tree()?;
        let tip = self.commit_of(until)?.tree()?;
        let mut diff = self.repo.diff_tree_to_tree(Some(&base), Some(&tip), None)?;
        diff.find_similar(Some(DiffFindOptions::new().renames(true)))?;
        stats_of(&diff)
    }

    /// Whether `refspec` names a commit of this repository.
    pub fn has_commit(&self, refspec: &str) -> bool {
        self.commit_of(refspec).is_ok()
    }

    /// Whether `ancestor` is in the history of `descendant`, and not the
    /// same commit.
    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> bool {
        let (Ok(ancestor), Ok(descendant)) = (self.commit_of(ancestor), self.commit_of(descendant))
        else {
            return false;
        };
        self.repo
            .graph_descendant_of(descendant.id(), ancestor.id())
            .unwrap_or(false)
    }

    /// Commit counts of `targets` and the files that changed with each of
    /// them, over the history that precedes a change. With `since`, the
    /// history ends where `since` and `until` diverged: the commits made
    /// after it are the change under analysis. Without it, it ends at `until`.
    ///
    /// A commit touching more than `max_files` files is skipped altogether: a
    /// sweeping change (a reformat, a rename across the project) relates its
    /// files to each other by accident.
    pub fn co_change_history(
        &self,
        (since, until): (Option<&str>, &str),
        since_ts: Option<i64>,
        targets: &HashSet<PathBuf>,
        max_files: usize,
    ) -> Result<CoChangeHistory, Box<dyn Error>> {
        let end = match since {
            Some(since) => self.merge_base(since, until)?,
            None => self.commit_of(until)?,
        };
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(end.id())?;
        let mut history = CoChangeHistory::default();

        self.walk(revwalk, since_ts, |commit| {
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
