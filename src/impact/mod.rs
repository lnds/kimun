//! Impact of a diff: how spread the change is and which files usually change
//! with it and were left out.
//!
//! Both measures come from git alone, so they apply to any language. The diff
//! runs from the merge base with a ref to the working tree, and the history
//! consulted ends at that merge base: the change is never evidence for itself.

mod analyzer;
mod projects;
mod report;

use std::collections::HashSet;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::cli::OutputMode;
use crate::git::{ChangeKind, FileDiffStat, GitRepo};
use crate::projects::ProjectGraph;
use crate::util::{is_generated, parse_since};

use analyzer::{
    Diffusion, MissingCoChange, Target, Thresholds, compute_diffusion, missing_co_changes,
};
use projects::ProjectRadius;

/// Options for impact analysis.
pub struct ImpactOptions<'a> {
    pub output: OutputMode,
    pub since_ref: &'a str,
    /// Bound on the history consulted (e.g. `6m`).
    pub since: Option<&'a str>,
    pub min_confidence: f64,
    pub min_shared: usize,
    /// Commits touching more files than this are not evidence of co-change.
    pub max_changeset: usize,
    pub top: usize,
    /// Print only the projects the diff changes or reaches, one per line.
    pub affected: bool,
}

/// Impact of the diff against a ref.
pub struct Impact {
    pub since_ref: String,
    /// Projects of the repository reached by the diff.
    pub projects: ProjectRadius,
    pub diffusion: Diffusion,
    pub thresholds: Thresholds,
    /// Files expected to change with the diff, strongest evidence first.
    pub missing: Vec<MissingCoChange>,
    /// Changed files with no commits in the history consulted, so nothing
    /// can be expected of them.
    pub without_history: Vec<PathBuf>,
    /// Generated files in the diff, left out of every measure.
    pub generated_skipped: usize,
    pub max_changeset: usize,
    /// Commits of the changed files ignored for exceeding `max_changeset`.
    pub skipped_commits: usize,
}

/// The changed files whose past commits can predict other changes. A new
/// file has no past; a renamed one keeps its history under the old path.
fn history_targets(changes: &[FileDiffStat]) -> Vec<Target> {
    changes
        .iter()
        .filter(|c| c.kind != ChangeKind::Added)
        .map(|c| Target {
            path: c.path.clone(),
            history_path: c.old_path.clone().unwrap_or_else(|| c.path.clone()),
        })
        .collect()
}

fn open_repo(path: &Path) -> Result<GitRepo, Box<dyn Error>> {
    Ok(GitRepo::open(path).map_err(|e| format!("not a git repository (or any parent): {e}"))?)
}

/// The reach of the diff over the projects of the repository. A file counts
/// under both of its paths when it was renamed, and a lock file counts too:
/// it changes what its project is built from.
fn project_radius(git_repo: &GitRepo, diff: &[FileDiffStat]) -> ProjectRadius {
    let graph = ProjectGraph::discover(git_repo.root());
    let files = diff
        .iter()
        .flat_map(|c| std::iter::once(c.path.as_path()).chain(c.old_path.as_deref()));
    projects::compute(&graph, files)
}

/// Compute the impact of the diff between `opts.since_ref` and the working tree.
fn analyze(path: &Path, opts: &ImpactOptions<'_>) -> Result<Impact, Box<dyn Error>> {
    if !(opts.min_confidence > 0.0 && opts.min_confidence <= 1.0) {
        return Err("--min-confidence must be greater than 0 and at most 1".into());
    }
    if opts.min_shared == 0 {
        return Err("--min-shared must be at least 1".into());
    }
    if opts.max_changeset == 0 {
        return Err("--max-changeset must be at least 1".into());
    }

    let git_repo = open_repo(path)?;
    let since_ts = opts.since.map(parse_since).transpose()?;

    let diff = git_repo.diff_stats_since(opts.since_ref)?;
    let projects = project_radius(&git_repo, &diff);
    let (generated, changes): (Vec<_>, Vec<_>) =
        diff.into_iter().partition(|c| is_generated(&c.path));

    let targets = history_targets(&changes);
    let history_paths: HashSet<PathBuf> = targets.iter().map(|t| t.history_path.clone()).collect();
    let history =
        git_repo.co_change_history(opts.since_ref, since_ts, &history_paths, opts.max_changeset)?;

    // A file already in the diff is not missing, under either of its names.
    let in_diff: HashSet<&Path> = changes
        .iter()
        .flat_map(|c| std::iter::once(c.path.as_path()).chain(c.old_path.as_deref()))
        .collect();
    let thresholds = Thresholds {
        min_confidence: opts.min_confidence,
        min_shared: opts.min_shared,
    };
    let missing = missing_co_changes(
        &history,
        &targets,
        |p| !in_diff.contains(p) && !is_generated(p) && git_repo.has_file(p),
        thresholds,
    );

    let with_history: HashSet<&Path> = targets
        .iter()
        .filter(|t| history.commits.contains_key(&t.history_path))
        .map(|t| t.path.as_path())
        .collect();
    let mut without_history: Vec<PathBuf> = changes
        .iter()
        .filter(|c| !with_history.contains(c.path.as_path()))
        .map(|c| c.path.clone())
        .collect();
    without_history.sort();

    Ok(Impact {
        since_ref: opts.since_ref.to_string(),
        projects,
        diffusion: compute_diffusion(&changes),
        thresholds,
        missing,
        without_history,
        generated_skipped: generated.len(),
        max_changeset: opts.max_changeset,
        skipped_commits: history.skipped_commits,
    })
}

/// Print the projects the diff calls to build and test, one per line. Only
/// the diff and the manifests are read: this is the path CI takes on every
/// push, and the history is of no use to it.
fn print_affected(path: &Path, since_ref: &str) -> Result<(), Box<dyn Error>> {
    let git_repo = open_repo(path)?;
    let radius = project_radius(&git_repo, &git_repo.diff_stats_since(since_ref)?);
    if !radius.outside.is_empty() {
        eprintln!(
            "note: {} changed file(s) belong to no project, so every project is listed",
            radius.outside.len()
        );
    }
    for project in radius.affected() {
        println!("{project}");
    }
    Ok(())
}

/// Run impact analysis and print it in the requested format.
pub fn run(path: &Path, opts: &ImpactOptions<'_>) -> Result<(), Box<dyn Error>> {
    if opts.affected {
        return print_affected(path, opts.since_ref);
    }
    let impact = analyze(path, opts)?;

    match opts.output {
        OutputMode::Json => report::print_json(&impact, opts.top)?,
        OutputMode::Short => report::print_short(&impact),
        OutputMode::Terse => report::print_terse(&impact),
        OutputMode::Github | OutputMode::Codeclimate => {
            return Err(crate::cli::ERR_CI_FORMAT_ONLY.into());
        }
        OutputMode::Table => report::print_report(&impact, opts.top),
    }
    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
