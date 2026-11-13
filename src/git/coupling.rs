//! The history temporal coupling reads: the commits of each file and the
//! groups of files changed together, both over the same commits.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ops::ControlFlow;
use std::path::PathBuf;

use super::GitRepo;

/// Commits of each file and the files of each commit that changed two or
/// more, counted over the same commits.
#[derive(Default)]
pub struct CouplingHistory {
    /// Commits that touched each file.
    pub commits: HashMap<PathBuf, usize>,
    /// The files of each commit that touched two or more.
    pub co_changes: Vec<Vec<PathBuf>>,
}

/// The commit a revert undoes, as `git revert` writes it: a message opening
/// with `Revert "` (or `Reapply "`, for the revert of a revert) that says
/// `This reverts commit <sha>`.
fn reverted_commit(commit: &git2::Commit) -> Option<git2::Oid> {
    let message = commit.message().ok()?;
    if !message.starts_with("Revert \"") && !message.starts_with("Reapply \"") {
        return None;
    }
    let after = message.split("This reverts commit ").nth(1)?;
    let sha: String = after.chars().take_while(char::is_ascii_hexdigit).collect();
    git2::Oid::from_str(&sha).ok()
}

/// The commits that are evidence of change, met newest first: a revert and
/// the commit it undoes together changed nothing.
#[derive(Default)]
struct Reverts {
    undone: HashSet<git2::Oid>,
}

impl Reverts {
    /// Whether `commit` counts. A revert that was itself reverted is undone
    /// and undoes nothing.
    fn counts(&mut self, commit: &git2::Commit) -> bool {
        if self.undone.remove(&commit.id()) {
            return false;
        }
        match reverted_commit(commit) {
            Some(undone) => {
                self.undone.insert(undone);
                false
            }
            None => true,
        }
    }
}

impl GitRepo {
    /// The commits of each file and the files changed together, over the
    /// same commits, so that a coupling strength divides by what its shared
    /// commits are counted from.
    ///
    /// Commits touching more than `max_changeset` files are left out: a
    /// sweeping change relates files by accident. So are a revert and the
    /// commit it reverts, which together changed nothing.
    pub fn coupling_history(
        &self,
        since: Option<i64>,
        max_changeset: usize,
    ) -> Result<CouplingHistory, Box<dyn Error>> {
        let mut history = CouplingHistory::default();
        let mut reverts = Reverts::default();
        self.walk_commits(since, |commit| {
            if !reverts.counts(commit) {
                return Ok(ControlFlow::Continue(()));
            }
            let paths = self.changed_files(commit)?;
            if paths.len() > max_changeset {
                return Ok(ControlFlow::Continue(()));
            }
            for path in &paths {
                *history.commits.entry(path.clone()).or_default() += 1;
            }
            if paths.len() >= 2 {
                history.co_changes.push(paths);
            }
            Ok(ControlFlow::Continue(()))
        })?;
        Ok(history)
    }
}
