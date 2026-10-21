//! The paths the history has held and the commits of each, read in one
//! pass over the log.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ops::ControlFlow;
use std::path::PathBuf;

use super::GitRepo;

/// The commits of each file, as `(commit, time)`: see
/// [`GitRepo::commits_by_file`].
pub type CommitsByFile = HashMap<PathBuf, Vec<(usize, i64)>>;

impl GitRepo {
    /// Every file path the history up to HEAD has held, deleted ones
    /// included.
    pub fn paths_in_history(&self) -> Result<HashSet<PathBuf>, Box<dyn Error>> {
        Ok(self.commits_by_file()?.into_keys().collect())
    }

    /// The commits of each file the history up to HEAD has held, newest
    /// first, as `(commit, time)`: commits are numbered, so one that
    /// touched several files can be counted once. Read through `git log`,
    /// far faster on a long history, or, failing that, libgit2.
    pub fn commits_by_file(&self) -> Result<CommitsByFile, Box<dyn Error>> {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["-c", "core.quotePath=false", "log", "--name-only"])
            .args(["--format=%x00%ct", "--no-renames", "HEAD"])
            .output()
            .ok()
            .filter(|output| output.status.success());
        let mut files = CommitsByFile::new();
        if let Some(output) = output {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut commit = (0, 0);
            for line in text.lines().filter(|line| !line.is_empty()) {
                match line.strip_prefix('\0') {
                    Some(time) => commit = (commit.0 + 1, time.parse().unwrap_or(0)),
                    None => files.entry(PathBuf::from(line)).or_default().push(commit),
                }
            }
            return Ok(files);
        }
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        let mut number = 0;
        self.walk(revwalk, None, |commit| {
            number += 1;
            for path in self.changed_files(commit)? {
                let time = commit.time().seconds();
                files.entry(path).or_default().push((number, time));
            }
            Ok(ControlFlow::Continue(()))
        })?;
        Ok(files)
    }
}
