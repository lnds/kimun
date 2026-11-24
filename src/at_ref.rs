//! A git ref written to a temporary directory, read as the working tree is:
//! its submodules, and the ignore rules of the repository and of each
//! submodule, which the directory is marked as one for.

use std::error::Error;
use std::path::{Path, PathBuf};

use crate::git::{GitRepo, LeftOut};
use crate::walk::{self, ExcludeFilter, WalkConfig};

/// The tree of a ref, written where the walk of a command can read it.
pub struct RefTree {
    /// Keeps the directory alive while the walk reads it.
    _dir: tempfile::TempDir,
    /// The directory of the ref that corresponds to the path analyzed.
    pub path: PathBuf,
    /// The path analyzed, from the root of the repository.
    pub prefix: PathBuf,
    /// The filter both sides are walked with: that of the command, leaving
    /// out the repositories the working tree stops at and the submodules
    /// the ref could not read.
    pub filter: ExcludeFilter,
    /// The filter the ref is walked with: `filter`, the patterns of the
    /// global excludes file anchored where the working tree anchors them.
    pub ref_filter: ExcludeFilter,
    /// Whether a submodule is left out on both sides.
    pub left_out: bool,
}

impl RefTree {
    /// Write the tree of `git_ref`, with its submodules under the path
    /// analyzed when the filter of `cfg` reads them, and build the filter
    /// both sides are walked with. A submodule the ref could not read is
    /// warned about on stderr.
    pub fn write(
        repo: &GitRepo,
        cfg: &WalkConfig<'_>,
        git_ref: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let (_, prefix) = repo.walk_prefix(cfg.path)?;
        let reads = cfg.filter.reads_submodules();
        let unread = repo.extract_tree_to_dir(git_ref, dir.path(), reads, &prefix)?;
        let both = both_sides(repo, git_ref, &prefix, unread)?;
        let mut stops = stops_of(cfg, dir.path());
        stops.extend(both.iter().cloned());
        let path = dir.path().join(&prefix);
        let filter = cfg.filter.clone().leaving_out(stops);
        let base = dir.path().join(global_anchor_of(repo, cfg, &prefix));
        Ok(Self {
            ref_filter: filter.clone().anchoring_global_at(base),
            _dir: dir,
            path,
            prefix,
            left_out: !both.is_empty(),
            filter,
        })
    }
}

/// Where the walk of `cfg` anchors the patterns of the global excludes file,
/// from the root of the repository: the root, when the path analyzed is
/// written through it ([`walk::global_anchor`]); else the current
/// directory, when the path analyzed (`prefix`) lies under it. Never above
/// the root, so that no pattern meets the directories that hold the ref.
fn global_anchor_of(repo: &GitRepo, cfg: &WalkConfig<'_>, prefix: &Path) -> PathBuf {
    if walk::global_anchor(cfg.path).is_some() {
        return PathBuf::new();
    }
    std::env::current_dir()
        .ok()
        .and_then(|cwd| repo.walk_prefix(&cwd).ok())
        .map(|(_, rel)| rel)
        .filter(|rel| prefix.starts_with(rel))
        .unwrap_or_default()
}

/// The directories, relative to the walk of `cfg`, that no side reads apart
/// from the submodules left out: the repositories the working tree stops at
/// unless it reads them, so the ref stops there too, and the temporary
/// directory `tmp` when it lies inside the tree.
fn stops_of(cfg: &WalkConfig<'_>, tmp: &Path) -> Vec<PathBuf> {
    let mut stops = if cfg.filter.reads_submodules() {
        Vec::new()
    } else {
        walk::nested_repositories(cfg.path, cfg.filter.honors_gitignore())
    };
    let inside = |dir: &Path| dir.canonicalize().ok();
    if let (Some(root), Some(tmp)) = (inside(cfg.path), inside(tmp))
        && let Ok(relative) = tmp.strip_prefix(root)
    {
        stops.push(relative.to_path_buf());
    }
    stops
}

/// Warn about each submodule the ref could not read, and give those that
/// are left out on both sides, relative to the path analyzed: the ones that
/// are still submodules in the working tree. The others are left out at the
/// ref only. A submodule that holds the path analyzed is an error.
fn both_sides(
    repo: &GitRepo,
    git_ref: &str,
    prefix: &Path,
    unread: Vec<LeftOut>,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut both = Vec::new();
    for LeftOut { path, advice } in unread {
        if prefix.starts_with(&path) {
            return Err(format!(
                "the path analyzed lies in submodule {}, which {git_ref} cannot read: {advice}",
                path.display()
            )
            .into());
        }
        if repo.is_submodule_now(&path) {
            eprintln!(
                "warning: submodule {} is left out on both sides: {advice}",
                path.display()
            );
            both.extend(path.strip_prefix(prefix).map(Path::to_path_buf));
        } else {
            eprintln!(
                "warning: submodule {} is left out at {git_ref}: {advice}",
                path.display()
            );
        }
    }
    Ok(both)
}
