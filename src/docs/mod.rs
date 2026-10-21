//! Measures of documentation, the files of kind `docs`.
//!
//! `km docs refs` finds the references of Markdown documents that lead
//! nowhere: links and images to missing files or headings, and paths in
//! code spans that the repository no longer has. `km docs stale` finds the
//! documents whose code changed after them.

pub mod context;
mod context_report;
pub mod describes;
mod help;
pub mod links;
mod links_report;
pub mod markdown;
pub mod refs;
mod report;
pub mod stale;
mod stale_report;

pub use help::{
    CONTEXT as CONTEXT_HELP, LINKS as LINKS_HELP, REFS as REFS_HELP, STALE as STALE_HELP,
};

use std::collections::HashSet;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::cli::{DocsCommands, OutputMode};
use crate::config::KimunConfig;
use crate::git::GitRepo;
use crate::kind::Kind;
use crate::walk::WalkConfig;
use refs::{Dead, Repository};

/// What `km docs refs` was given besides the walk and the output.
pub struct RefsOptions<'a> {
    /// Fail when a dead reference is found.
    pub fail_on_dead: bool,
    /// Keep only documents changed since this ref, and references to what
    /// changed since it.
    pub since_ref: Option<&'a str>,
}

/// The dead references of one document.
pub struct DocumentRefs {
    pub path: PathBuf,
    pub dead: Vec<Dead>,
}

/// What the documents refer to: how many documents and references were
/// read, and the documents with dead references.
pub struct RefsReport {
    pub documents: usize,
    pub references: usize,
    pub with_dead: Vec<DocumentRefs>,
}

impl RefsReport {
    pub fn dead(&self) -> usize {
        self.with_dead.iter().map(|d| d.dead.len()).sum()
    }
}

/// Run a `km docs` subcommand.
pub fn dispatch(command: DocsCommands) {
    match command {
        DocsCommands::Refs {
            common,
            fail_on_dead,
            since_ref,
        } => {
            let options = RefsOptions {
                fail_on_dead,
                since_ref: since_ref.as_deref(),
            };
            crate::on_kind::run(Kind::Docs, common, |cfg, output| {
                refs(cfg, output, &options)
            })
        }
        DocsCommands::Stale { common, top } => {
            crate::on_kind::run(Kind::Docs, common, |cfg, output| stale(cfg, output, top))
        }
        DocsCommands::Links { common, top } => {
            crate::on_kind::run(Kind::Docs, common, |cfg, output| links(cfg, output, top))
        }
        DocsCommands::Context {
            common,
            top,
            max_tokens,
        } => crate::on_kind::run(Kind::Docs, common, |cfg, output| {
            context_report::run(cfg, output, top, max_tokens)
        }),
    }
}

type CommandResult = Result<(), Box<dyn Error>>;

/// The graph of links between the documents.
pub fn links(cfg: &WalkConfig<'_>, output: OutputMode, top: Option<usize>) -> CommandResult {
    let documents: Vec<(PathBuf, String)> = documents(cfg)?
        .into_iter()
        .map(|d| (d.walked, d.text))
        .collect();
    let graph = links::Graph::build(&documents, |path| path.exists());
    let notes = documents.into_iter().map(|(path, _)| path).collect();
    let report = links_report::LinksReport::new(notes, graph);
    match output {
        OutputMode::Json => links_report::print_json(&report, top)?,
        OutputMode::Short => links_report::print_short(&report),
        OutputMode::Terse => println!("{}", report.graph.broken.len()),
        OutputMode::Github | OutputMode::Codeclimate => {
            return Err(crate::cli::ERR_CI_FORMAT_ONLY.into());
        }
        OutputMode::Table => links_report::print_report(&report, top),
    }
    Ok(())
}

/// The documents behind the code they describe.
pub fn stale(cfg: &WalkConfig<'_>, output: OutputMode, top: Option<usize>) -> CommandResult {
    let root = repository_root(cfg.path)?;
    let git =
        GitRepo::open(&root).map_err(|e| format!("not a git repository (or any parent): {e}"))?;
    let history = git.commits_by_file()?;
    let present = history.keys().filter(|path| root.join(path).is_file());
    let files = stale::Files::new(present.cloned());
    let rules = describes::Rules::new(&KimunConfig::load_from(&root).docs)?;
    let documents: Vec<(PathBuf, String)> = documents(cfg)?
        .into_iter()
        .filter_map(|d| Some((d.full.strip_prefix(&root).ok()?.to_path_buf(), d.text)))
        .collect();
    let found = stale::stale(&documents, &files, &history, &rules);
    match output {
        OutputMode::Json => stale_report::print_json(&found, top)?,
        OutputMode::Short => stale_report::print_short(&found, documents.len()),
        OutputMode::Terse => println!("{}", found.len()),
        OutputMode::Github | OutputMode::Codeclimate => {
            return Err(crate::cli::ERR_CI_FORMAT_ONLY.into());
        }
        OutputMode::Table => stale_report::print_report(&found, documents.len(), top),
    }
    Ok(())
}

/// A Markdown document of the walk.
struct Document {
    walked: PathBuf,
    full: PathBuf,
    text: String,
}

/// The Markdown documents of the walk.
fn documents(cfg: &WalkConfig<'_>) -> Result<Vec<Document>, Box<dyn Error>> {
    let mut documents = Vec::new();
    for (walked, spec) in cfg.source_files() {
        if spec.name != "Markdown" {
            continue;
        }
        let full = walked.canonicalize()?;
        match std::fs::read_to_string(&full) {
            Ok(text) => documents.push(Document { walked, full, text }),
            Err(err) => eprintln!("warning: {}: {err}", walked.display()),
        }
    }
    Ok(documents)
}

pub fn refs(
    cfg: &WalkConfig<'_>,
    output: OutputMode,
    options: &RefsOptions<'_>,
) -> Result<(), Box<dyn Error>> {
    let repository = repository(&repository_root(cfg.path)?);
    let changed = match options.since_ref {
        Some(since) => Some(changed_since(cfg.path, since)?),
        None => None,
    };
    let report = collect(cfg, &repository, changed.as_ref())?;
    match output {
        OutputMode::Json => report::print_json(&report)?,
        OutputMode::Short => report::print_short(&report),
        OutputMode::Terse => println!("{}", report.dead()),
        OutputMode::Github => report::print_github(&report),
        OutputMode::Codeclimate => report::print_codeclimate(&report)?,
        OutputMode::Table => report::print_report(&report),
    }
    match report.dead() {
        dead if options.fail_on_dead && dead > 0 => {
            Err(format!("{dead} dead reference(s) in the documentation").into())
        }
        _ => Ok(()),
    }
}

/// The root of the git repository holding `path`, or `path` itself.
pub(crate) fn repository_root(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let root = git2::Repository::discover(path)
        .ok()
        .and_then(|repo| repo.workdir().map(Path::to_path_buf))
        .unwrap_or_else(|| path.to_path_buf());
    Ok(root.canonicalize()?)
}

/// The repository under `root`, with its history when it has one.
fn repository(root: &Path) -> Repository {
    let git_root = root.to_path_buf();
    Repository::index(root, move || {
        GitRepo::open(&git_root)
            .ok()
            .and_then(|git| git.paths_in_history().ok())
    })
}

/// The paths changed since `since`, relative to the root, old names of
/// moved files included.
pub(crate) fn changed_since(path: &Path, since: &str) -> Result<HashSet<PathBuf>, Box<dyn Error>> {
    let git = GitRepo::open(path).map_err(|e| format!("not a git repository: {e}"))?;
    let mut changed = HashSet::new();
    for stat in git.diff_stats_since(since)? {
        changed.extend(stat.old_path);
        changed.insert(stat.path);
    }
    Ok(changed)
}

fn collect(
    cfg: &WalkConfig<'_>,
    repository: &Repository,
    changed: Option<&HashSet<PathBuf>>,
) -> Result<RefsReport, Box<dyn Error>> {
    let mut report = RefsReport {
        documents: 0,
        references: 0,
        with_dead: Vec::new(),
    };
    for Document {
        walked: path,
        full: document,
        text,
    } in documents(cfg)?
    {
        let checked = refs::check(&document, &text, repository);
        report.documents += 1;
        report.references += checked.references;
        let mut dead = checked.dead;
        if let Some(changed) = changed {
            let own = document
                .strip_prefix(repository.root())
                .unwrap_or(&document);
            if !changed.contains(own) {
                dead.retain(|d| names_a_change(&d.names, changed));
            }
        }
        if !dead.is_empty() {
            report.with_dead.push(DocumentRefs { path, dead });
        }
    }
    report.with_dead.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(report)
}

/// Whether a reference names a path that changed, whole or by its
/// trailing part.
fn names_a_change(names: &Path, changed: &HashSet<PathBuf>) -> bool {
    !names.as_os_str().is_empty()
        && changed
            .iter()
            .any(|path| path == names || path.ends_with(names))
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
