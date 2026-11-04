//! Impact of a diff: how spread the change is and which files usually change
//! with it and were left out.
//!
//! Both measures come from git alone, so they apply to any language. The diff
//! runs from the merge base with a ref to the working tree, and the history
//! consulted ends at that merge base: the change is never evidence for itself.

mod analyzer;
mod copies;
mod copies_report;
mod documentation;
mod documentation_report;
mod entry;
mod function_reach;
mod help;
mod inert;
mod narrowing;
mod pr;
mod projects;
mod protection;
mod reading;
mod removed;
mod removed_report;
mod report;
mod role;
mod routes;
mod source;
mod structural;
mod structural_json;
mod structural_report;
mod values;

use std::collections::HashSet;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::cli::OutputMode;
use crate::config::KimunConfig;
use crate::deps::heex;
use crate::deps::layout::is_manifest;
use crate::git::{ChangeKind, FileDiffStat, GitRepo};
use crate::projects::ProjectGraph;
use crate::util::{is_generated, parse_since};

use analyzer::{
    Diffusion, MissingCoChange, Target, Thresholds, compute_diffusion, missing_co_changes,
};
use documentation::{Signal, Stale};
use entry::EntryPoints;
pub use help::HELP;
use inert::Inert;
use projects::ProjectRadius;
use source::Change;
pub use source::DiffSource;
use structural::Structural;

/// Options for impact analysis.
#[derive(Debug, Clone)]
pub struct ImpactOptions {
    pub output: OutputMode,
    /// The change to measure.
    pub source: DiffSource,
    /// Bound on the history consulted (e.g. `6m`).
    pub since: Option<String>,
    pub min_confidence: f64,
    pub min_shared: usize,
    /// Commits touching more files than this are not evidence of co-change.
    pub max_changeset: usize,
    /// How far the structural radius is followed, in files from the change.
    pub max_distance: Option<usize>,
    pub top: usize,
    /// Print only the projects the diff changes or reaches, one per line.
    pub affected: bool,
    /// Fail when a document that covers or names a changed file was not
    /// updated.
    pub fail_on_stale_docs: bool,
}

/// The options the command line asks for. A pull request comes first, then
/// a patch, then the refs: the parser already refuses the mixes that make
/// no sense.
pub fn options(args: &crate::cli::ImpactArgs) -> ImpactOptions {
    let source = match (args.pr, &args.diff, &args.since_ref) {
        (Some(number), _, _) => DiffSource::PullRequest(number),
        (None, Some(file), base) => DiffSource::Patch {
            file: file.clone(),
            base: base.clone(),
        },
        (None, None, since) => DiffSource::Refs {
            since: since.clone().unwrap_or_else(|| "HEAD".to_string()),
            until: args.until_ref.clone(),
        },
    };
    ImpactOptions {
        output: args.format,
        source,
        since: args.since.clone(),
        min_confidence: args.min_confidence,
        min_shared: args.min_shared,
        max_changeset: args.max_changeset,
        max_distance: args.max_distance.map(|d| d as usize),
        top: args.top,
        affected: args.affected,
        fail_on_stale_docs: args.fail_on_stale_docs,
    }
}

/// Impact of a change.
pub struct Impact {
    /// How the change is named: `diff against main`, `PR #12`.
    pub source: String,
    /// Projects of the repository reached by the diff.
    pub projects: ProjectRadius,
    /// Source files that use what the diff changed.
    pub structural: Structural,
    pub diffusion: Diffusion,
    pub thresholds: Thresholds,
    /// Files expected to change with the diff, strongest evidence first.
    /// Documents are in `documentation` instead.
    pub missing: Vec<MissingCoChange>,
    /// Documents that describe what changed and were not updated.
    pub documentation: Vec<Stale>,
    /// Groups of values outside the change that hold what it replaced.
    pub copies: Vec<copies::Copy>,
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
fn project_radius(graph: &ProjectGraph, change: &Change, inert: &Inert) -> ProjectRadius {
    let (still, moving): (Vec<&Path>, Vec<&Path>) = change
        .diff
        .iter()
        .flat_map(|c| std::iter::once(c.path.as_path()).chain(c.old_path.as_deref()))
        .partition(|path| inert.matches(path));
    let mut radius = projects::compute(graph, moving.into_iter());
    let still: std::collections::BTreeSet<&Path> = still.into_iter().collect();
    radius.inert = still.into_iter().map(Path::to_path_buf).collect();
    radius
}

/// What the repository declares inert, on top of documentation.
fn inert_of(git_repo: &GitRepo) -> Result<Inert, Box<dyn Error>> {
    Inert::new(&KimunConfig::load_from(git_repo.root()).impact.inert)
}

/// The source files that use what the change touches. Only the projects the
/// change affects are read: a file outside them cannot depend on it. When
/// the reach over projects is unknown, the whole repository is.
fn structural_radius(
    git_repo: &GitRepo,
    change: &Change,
    graph: &ProjectGraph,
    projects: &ProjectRadius,
    max_distance: Option<usize>,
) -> Result<Structural, Box<dyn Error>> {
    let scope: Vec<PathBuf> = match projects.changed.is_empty() || !projects.outside.is_empty() {
        true => Vec::new(),
        false => projects
            .affected()
            .into_iter()
            .map(|root| PathBuf::from(root.trim_start_matches('.')))
            .collect(),
    };
    let in_scope =
        |path: &Path| scope.is_empty() || scope.iter().any(|root| path.starts_with(root));
    // Manifests are read wherever they are: a config above every project,
    // such as a root `tsconfig.json`, says how their imports resolve.
    let measured = |p: &Path| structural::is_measured(p) || heex::is_template(p);
    let sources = change.sources(git_repo, |p| (measured(p) && in_scope(p)) || is_manifest(p))?;
    let changed: Vec<structural::Changed> = change.diff.iter().collect();
    let mut radius = structural::compute(
        sources,
        &changed,
        &|from, to| graph.may_use(from, to),
        max_distance,
    );
    let config = KimunConfig::load_from(git_repo.root()).impact;
    let entry = EntryPoints::new(&config.entry_points)?;
    radius.mark_entry_points(
        |path| entry.conventional(path),
        |path| entry.configured(path),
    );
    Ok(radius)
}

/// Whether a file outside a change may hold a copy of its values: one of a
/// known language, not a document nor a generated file.
fn may_hold_copies(path: &Path) -> bool {
    crate::loc::language::detect(path)
        .is_some_and(|spec| crate::kind::Kind::by_name(path, spec) != crate::kind::Kind::Docs)
        && !is_generated(path)
}

/// The groups of values outside the change that hold what it replaced.
/// The rest of the repository is read only when the change modifies such a
/// group.
fn copies_of(
    git_repo: &GitRepo,
    change: &Change,
    changes: &[FileDiffStat],
    in_diff: &HashSet<&Path>,
) -> Result<Vec<copies::Copy>, Box<dyn Error>> {
    let kept: Vec<&FileDiffStat> = changes
        .iter()
        .filter(|c| c.kind != ChangeKind::Deleted && may_hold_copies(&c.path))
        .collect();
    let wanted: HashSet<&Path> = kept.iter().map(|c| c.path.as_path()).collect();
    let heads = change.sources(git_repo, |p| wanted.contains(p))?;
    let edits: Vec<copies::Edit> = kept
        .iter()
        .flat_map(|c| {
            let head = heads.iter().find(|(p, _)| *p == c.path);
            head.map(|(_, text)| copies::edits(c, text))
                .unwrap_or_default()
        })
        .collect();
    if edits.is_empty() {
        return Ok(Vec::new());
    }
    let files = change.sources(git_repo, |p| !in_diff.contains(p) && may_hold_copies(p))?;
    Ok(copies::copies(&edits, &files))
}

/// Compute the impact of the change `opts.source` names.
fn analyze(path: &Path, opts: &ImpactOptions) -> Result<Impact, Box<dyn Error>> {
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
    let since_ts = opts.since.as_deref().map(parse_since).transpose()?;

    let change = Change::resolve(&git_repo, &opts.source)?;
    let graph = change.graph(&git_repo)?;
    let projects = project_radius(&graph, &change, &inert_of(&git_repo)?);
    let structural = structural_radius(&git_repo, &change, &graph, &projects, opts.max_distance)?;
    let (generated, changes): (Vec<_>, Vec<_>) = change
        .diff
        .iter()
        .cloned()
        .partition(|c| is_generated(&c.path));

    let targets = history_targets(&changes);
    let history_paths: HashSet<PathBuf> = targets.iter().map(|t| t.history_path.clone()).collect();
    let history = change.history(&git_repo, since_ts, &history_paths, opts.max_changeset)?;

    // A file already in the diff is not missing, under either of its names.
    let in_diff: HashSet<&Path> = changes
        .iter()
        .flat_map(|c| std::iter::once(c.path.as_path()).chain(c.old_path.as_deref()))
        .collect();
    let thresholds = Thresholds {
        min_confidence: opts.min_confidence,
        min_shared: opts.min_shared,
    };
    let (documents_missing, missing): (Vec<_>, Vec<_>) = missing_co_changes(
        &history,
        &targets,
        |p| !in_diff.contains(p) && !is_generated(p) && change.has_file(&git_repo, p),
        thresholds,
    )
    .into_iter()
    .partition(|m| documentation::is_document(&m.path));
    let rules = documentation::Rules::new(&KimunConfig::load_from(git_repo.root()).docs)?;
    let documents = change.sources(&git_repo, documentation::is_document)?;
    let documentation = documentation::stale(&documents, &changes, &documents_missing, &rules);
    let copies = copies_of(&git_repo, &change, &changes, &in_diff)?;

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
        source: change.label.clone(),
        projects,
        structural,
        diffusion: compute_diffusion(&changes),
        thresholds,
        missing,
        documentation,
        copies,
        without_history,
        generated_skipped: generated.len(),
        max_changeset: opts.max_changeset,
        skipped_commits: history.skipped_commits,
    })
}

/// Print the projects the diff calls to build and test, one per line. Only
/// the diff and the manifests are read: this is the path CI takes on every
/// push, and the history is of no use to it.
fn print_affected(path: &Path, source: &DiffSource) -> Result<(), Box<dyn Error>> {
    let git_repo = open_repo(path)?;
    let change = Change::resolve(&git_repo, source)?;
    let radius = project_radius(&change.graph(&git_repo)?, &change, &inert_of(&git_repo)?);
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
pub fn run(path: &Path, opts: &ImpactOptions) -> Result<(), Box<dyn Error>> {
    if opts.affected {
        return print_affected(path, &opts.source);
    }
    let impact = analyze(path, opts)?;

    match opts.output {
        OutputMode::Json => report::print_json(&impact, opts.top)?,
        OutputMode::Short => report::print_short(&impact),
        OutputMode::Terse => report::print_terse(&impact),
        OutputMode::Github => documentation_report::print_github(&impact.documentation),
        OutputMode::Codeclimate => documentation_report::print_codeclimate(&impact.documentation)?,
        OutputMode::Table => report::print_report(&impact, opts.top),
    }
    let left_behind = impact
        .documentation
        .iter()
        .filter(|s| s.signal != Signal::CoChanged)
        .count();
    match left_behind {
        n if opts.fail_on_stale_docs && n > 0 => {
            Err(format!("{n} document(s) describe what changed and were not updated").into())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
