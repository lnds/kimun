//! Hotspot analysis — identifies high-risk files that change often and are complex.
//!
//! Combines git change frequency with cyclomatic or indentation complexity
//! to produce a hotspot score (commits x complexity). Files with high scores
//! are the most impactful refactoring targets.

mod report;

use std::cmp::Reverse;
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::cli::OutputMode;
use crate::git::GitRepo;
use crate::kind::Kind;
use crate::util::parse_since;
use crate::walk::{self, WalkConfig};
use report::{print_json, print_report, print_short, print_terse};

/// A file's hotspot data: how often it changes (commits) and how complex
/// it is, combined into a score = commits × complexity.
pub struct FileHotspot {
    pub path: PathBuf,
    pub language: String,
    pub commits: usize,
    pub complexity: usize,
    /// What `complexity` measures: the metric asked for, or `lines` for a
    /// document, where indentation and branches say nothing.
    pub metric: &'static str,
    pub score: usize,
}

/// The metric of a file: lines for a document, else the one asked for.
fn metric_of(kind: Kind, asked: &str) -> &'static str {
    match (kind, asked) {
        (Kind::Docs, _) => "lines",
        (_, "cycom") => "cycom",
        (_, "cogcom") => "cogcom",
        _ => "indent",
    }
}

/// The lines of a document that hold text.
fn text_lines(file_path: &Path) -> Result<Option<usize>, Box<dyn Error>> {
    let text = String::from_utf8_lossy(&std::fs::read(file_path)?).into_owned();
    Ok(Some(text.lines().filter(|l| !l.trim().is_empty()).count()))
}

/// Compute complexity for a file using the chosen metric.
/// Returns None if the file cannot be analyzed.
fn compute_complexity(
    file_path: &Path,
    spec: &crate::loc::language::LanguageSpec,
    metric: &str,
    exclude_tests: bool,
) -> Result<Option<usize>, Box<dyn Error>> {
    Ok(match metric {
        "lines" => text_lines(file_path)?,
        "cycom" => crate::cycom::analyze_file(file_path, spec, exclude_tests)?
            .map(|file| file.total_complexity),
        "cogcom" => crate::cogcom::analyze_file(file_path, spec, exclude_tests)?
            .map(|file| file.total_complexity),
        _ => crate::indent::analyze_file(file_path, spec, exclude_tests)?
            .map(|file| file.total_indent),
    })
}

/// The hotspots of the code files: commits since `since_ts` (a Unix time)
/// times complexity by `complexity_metric`. `None` when no commit is found.
pub(crate) fn collect(
    cfg: &WalkConfig<'_>,
    since_ts: Option<i64>,
    complexity_metric: &str,
) -> Result<Option<Vec<FileHotspot>>, Box<dyn Error>> {
    let git_repo = GitRepo::open(cfg.path)
        .map_err(|e| format!("not a git repository (or any parent): {e}"))?;
    let freqs = git_repo.file_frequencies(since_ts)?;
    if freqs.is_empty() {
        return Ok(None);
    }
    let freq_map: HashMap<PathBuf, usize> =
        freqs.into_iter().map(|f| (f.path, f.commits)).collect();

    let (walk_root, walk_prefix) = git_repo.walk_prefix(cfg.path)?;

    let exclude_tests = cfg.exclude_tests();
    let kinds = cfg.filter.kinds();
    let mut results: Vec<FileHotspot> = Vec::new();

    for (file_path, spec) in walk::code_files(&walk_root, cfg.exclude_tests(), cfg.filter) {
        let rel_path = GitRepo::to_git_path(&walk_root, &walk_prefix, &file_path);

        // Look up commits from git history (before expensive analysis)
        let commits = match freq_map.get(&rel_path) {
            Some(&c) => c,
            None => continue,
        };

        // Compute complexity (only for files with git history)
        let metric = metric_of(kinds.kind(&file_path, spec, &walk_root), complexity_metric);
        let complexity = match compute_complexity(&file_path, spec, metric, exclude_tests) {
            Ok(Some(c)) => c,
            Ok(None) => continue,
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
                continue;
            }
        };

        results.push(FileHotspot {
            path: rel_path,
            language: spec.name.to_string(),
            commits,
            complexity,
            metric,
            score: commits * complexity,
        });
    }
    Ok(Some(results))
}

/// Identify hotspot files by combining git change frequency with code
/// complexity. Opens the git repo, walks source files, computes complexity
/// per file, and sorts by the chosen metric (score, commits, or complexity).
pub fn run(
    cfg: &WalkConfig<'_>,
    output: OutputMode,
    top: Option<usize>,
    sort_by: &str,
    since: Option<&str>,
    complexity_metric: &str,
) -> Result<(), Box<dyn Error>> {
    let since_ts = since.map(parse_since).transpose()?;
    let Some(mut results) = collect(cfg, since_ts, complexity_metric)? else {
        if since.is_some() {
            eprintln!("No commits found in the specified time range.");
        } else {
            eprintln!("No commits found in the repository.");
        }
        return Ok(());
    };

    // Sort by chosen metric descending
    match sort_by {
        "commits" => results.sort_by_key(|r| Reverse(r.commits)),
        "complexity" => results.sort_by_key(|r| Reverse(r.complexity)),
        _ => results.sort_by_key(|r| Reverse(r.score)),
    }

    let shown = crate::report_helpers::shown_count(output, top, results.len());
    let listed = &results[..shown];
    match output {
        OutputMode::Json => print_json(listed)?,
        OutputMode::Short => print_short(&results),
        OutputMode::Terse => print_terse(&results),
        OutputMode::Github | OutputMode::Codeclimate => {
            return Err(crate::cli::ERR_CI_FORMAT_ONLY.into());
        }
        OutputMode::Table => print_report(listed),
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
