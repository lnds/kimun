//! Repositories nested in the tree a command walks: submodules, and clones
//! or worktrees kept inside it.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The root of a walk, and whether it lies in a repository.
pub(super) struct Root {
    pub(super) path: PathBuf,
    in_repository: bool,
}

/// Whether a directory holds `.git`, a directory or a file.
fn holds_git(dir: &Path) -> bool {
    dir.join(".git").symlink_metadata().is_ok()
}

impl Root {
    /// The root of a walk of `path`. It lies in a repository when it or one
    /// of its ancestors holds `.git`.
    pub(super) fn of(path: &Path) -> Self {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        Self {
            path: path.to_path_buf(),
            in_repository: canonical.ancestors().any(holds_git),
        }
    }

    /// Whether a directory below the root is a repository nested in the
    /// tree: it holds `.git`, and the walk lies in a repository or a
    /// directory between the root and it holds `.git`.
    pub(super) fn is_nested_repository(&self, dir: &Path) -> bool {
        holds_git(dir)
            && (self.in_repository
                || dir
                    .ancestors()
                    .skip(1)
                    .take_while(|ancestor| *ancestor != self.path)
                    .any(holds_git))
    }

    /// Whether the walk stops at a directory: it is in `left_out`, relative
    /// to the root, or it is a nested repository and `submodules` is false.
    pub(super) fn leaves_out(
        &self,
        dir: &Path,
        left_out: &HashSet<PathBuf>,
        submodules: bool,
    ) -> bool {
        let relative = dir.strip_prefix(&self.path).unwrap_or(dir);
        left_out.contains(relative) || (!submodules && self.is_nested_repository(dir))
    }

    /// Whether the walk goes as far as `path`, relative to the root: no
    /// directory on the way is one it leaves out.
    pub(super) fn reaches(
        &self,
        path: &Path,
        left_out: &HashSet<PathBuf>,
        submodules: bool,
    ) -> bool {
        path.ancestors()
            .skip(1)
            .filter(|dir| !dir.as_os_str().is_empty())
            .all(|dir| !self.leaves_out(&self.path.join(dir), left_out, submodules))
    }

    /// The repositories nested in the tree among `entries`, a walk of the
    /// root that goes into them, by their path from the root; those inside
    /// them are not listed.
    pub(super) fn nested_repositories(
        &self,
        entries: impl Iterator<Item = ignore::DirEntry>,
    ) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = Vec::new();
        for entry in entries {
            let path = entry.path();
            let is_dir = entry.file_type().is_some_and(|ft| ft.is_dir());
            if is_dir
                && entry.depth() > 0
                && !found.iter().any(|dir| path.starts_with(dir))
                && self.is_nested_repository(path)
            {
                found.push(path.to_path_buf());
            }
        }
        found
            .iter()
            .map(|dir| dir.strip_prefix(&self.path).unwrap_or(dir).to_path_buf())
            .collect()
    }
}
