//! Where the change under analysis comes from, and what surrounds it.
//!
//! Every source answers the same four questions: which files changed, where
//! the history that precedes the change ends, where the manifests are read
//! from, and where a file is looked up to know it still exists.

use std::collections::HashSet;
use std::error::Error;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use crate::git::{CoChangeHistory, FileDiffStat, GitRepo, patch_stats};
use crate::projects::{self, ProjectGraph};

use super::pr;

/// What was asked to be measured.
#[derive(Debug, Clone, PartialEq)]
pub enum DiffSource {
    /// From where `since` and `until` diverged up to `until`, or up to the
    /// working tree when no `until` is given.
    Refs {
        since: String,
        until: Option<String>,
    },
    /// A patch in git format, from a file or from stdin (`-`). The history
    /// ends where `base` and HEAD diverged, or at HEAD without one.
    Patch { file: PathBuf, base: Option<String> },
    /// A GitHub pull request, by number.
    PullRequest(u64),
}

/// A change resolved against a repository.
pub struct Change {
    /// How reports name the change.
    pub label: String,
    pub diff: Vec<FileDiffStat>,
    /// The refs the preceding history is bounded by: it ends where they
    /// diverged, or at the second one when the first is missing.
    history: (Option<String>, String),
    /// The ref whose tree holds the state after the change. `None` when
    /// that state is the working tree.
    after: Option<String>,
}

/// The bytes of a patch file, or of stdin for `-`.
fn read_patch(file: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    if file != Path::new("-") {
        return Ok(std::fs::read(file)
            .map_err(|e| format!("cannot read the patch {}: {e}", file.display()))?);
    }
    let mut stdin = std::io::stdin();
    if stdin.is_terminal() {
        return Err("--diff - reads a patch from stdin, and none was piped in".into());
    }
    let mut patch = Vec::new();
    stdin.read_to_end(&mut patch)?;
    Ok(patch)
}

impl Change {
    fn from_refs(
        repo: &GitRepo,
        label: String,
        since: &str,
        until: Option<&str>,
    ) -> Result<Self, Box<dyn Error>> {
        let diff = match until {
            Some(until) => repo.diff_stats_between(since, until)?,
            None => repo.diff_stats_since(since)?,
        };
        Ok(Self {
            label,
            diff,
            history: (Some(since.to_string()), until.unwrap_or("HEAD").to_string()),
            after: until.map(str::to_string),
        })
    }

    fn from_patch(label: String, patch: &[u8], base: Option<&str>) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            label,
            diff: patch_stats(patch)?,
            history: (base.map(str::to_string), "HEAD".to_string()),
            after: None,
        })
    }

    /// A pull request, from its commits when the repository has them and
    /// from its patch otherwise.
    fn from_pull_request(repo: &GitRepo, gh: &str, number: u64) -> Result<Self, Box<dyn Error>> {
        let label = format!("PR #{number}");
        let info = pr::info(gh, repo.root(), number)?;
        match pr::plan(&info, |c| repo.has_commit(c), |a, d| repo.is_ancestor(a, d)) {
            pr::Plan::Refs { since, until } => Self::from_refs(repo, label, &since, Some(&until)),
            pr::Plan::Patch { base } => {
                eprintln!(
                    "note: the commits of PR #{number} are not in this repository, so its \
                     patch is measured against the working tree. Run `git fetch origin \
                     pull/{number}/head` to measure it against its own tree."
                );
                let patch = pr::patch(gh, repo.root(), number)?;
                Self::from_patch(label, &patch, base.as_deref())
            }
        }
    }

    /// Resolve `source` against `repo`.
    pub fn resolve(repo: &GitRepo, source: &DiffSource) -> Result<Self, Box<dyn Error>> {
        match source {
            DiffSource::Refs { since, until } => {
                let label = match until {
                    Some(until) => format!("diff {since}...{until}"),
                    None => format!("diff against {since}"),
                };
                Self::from_refs(repo, label, since, until.as_deref())
            }
            DiffSource::Patch { file, base } => {
                let label = match file.to_str() {
                    Some("-") => "patch from stdin".to_string(),
                    _ => format!("patch from {}", file.display()),
                };
                Self::from_patch(label, &read_patch(file)?, base.as_deref())
            }
            DiffSource::PullRequest(number) => Self::from_pull_request(repo, pr::GH, *number),
        }
    }

    /// The projects of the repository as they are after the change.
    pub fn graph(&self, repo: &GitRepo) -> Result<ProjectGraph, Box<dyn Error>> {
        match &self.after {
            None => Ok(ProjectGraph::discover(repo.root())),
            Some(after) => Ok(ProjectGraph::from_manifests(repo.files_at(
                after,
                projects::is_skipped,
                projects::is_manifest,
            )?)),
        }
    }

    /// Whether `path` is a file after the change.
    pub fn has_file(&self, repo: &GitRepo, path: &Path) -> bool {
        match &self.after {
            None => repo.has_file(path),
            Some(after) => repo.has_file_at(after, path),
        }
    }

    /// What changed along with `targets` in the history before the change.
    pub fn history(
        &self,
        repo: &GitRepo,
        since_ts: Option<i64>,
        targets: &HashSet<PathBuf>,
        max_files: usize,
    ) -> Result<CoChangeHistory, Box<dyn Error>> {
        let (since, until) = &self.history;
        repo.co_change_history((since.as_deref(), until), since_ts, targets, max_files)
    }
}

#[cfg(test)]
#[path = "source_test.rs"]
mod tests;
