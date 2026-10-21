//! Temporal coupling analysis — finds files that change together in git.
//!
//! Identifies implicit dependencies between files by analyzing commit
//! co-occurrence. High coupling between unrelated files may indicate
//! hidden dependencies that should be made explicit or decoupled.

pub mod analyzer;
mod report;

use std::cmp::Reverse;
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::cli::OutputMode;
use crate::git::{FileFrequency, GitRepo};
use crate::util::parse_since;
use crate::walk::{self, ExcludeFilter};
use analyzer::compute_coupling;
use report::{print_json, print_report, print_short, print_terse};

/// Check whether a git-relative path is inside a test, example or benchmark
/// directory, or is a test file.
fn is_test_path(path: &Path) -> bool {
    for component in path.components() {
        if let Some(name) = component.as_os_str().to_str()
            && walk::is_non_production_dir(name)
        {
            return true;
        }
    }
    walk::is_test_file(path)
}

/// Options for temporal coupling analysis.
pub struct TcOptions<'a> {
    pub output: OutputMode,
    pub include_tests: bool,
    pub top: Option<usize>,
    pub sort_by: &'a str,
    pub since: Option<&'a str>,
    /// Commits a file needs to be paired.
    pub min_degree: usize,
    pub min_strength: Option<f64>,
    /// Commits a pair of files needs to share.
    pub min_shared: usize,
    /// Commits touching more files than this are no evidence of coupling.
    pub max_changeset: usize,
    /// The path filters of the command line, applied to the paths of history.
    pub filter: &'a ExcludeFilter,
}

/// Whether the filters keep a path of history: no excluded directory on
/// the way, and the file itself not excluded by extension or glob.
fn kept_by(filter: &ExcludeFilter, path: &Path) -> bool {
    let dirs = path.parent().into_iter().flat_map(Path::components);
    let excluded_dir = dirs
        .filter_map(|c| c.as_os_str().to_str())
        .any(|name| filter.excludes_dir(name));
    !excluded_dir && !filter.excludes_file(path, Path::new(""))
}

/// The commits of each file that may be paired: one that still exists, is
/// code, has `min_degree` commits, passes the filters and, unless asked
/// for, is no test.
fn eligible_files(
    freqs: Vec<FileFrequency>,
    git_repo: &GitRepo,
    opts: &TcOptions<'_>,
) -> HashMap<PathBuf, usize> {
    freqs
        .into_iter()
        .filter(|f| f.commits >= opts.min_degree)
        .filter(|f| walk::is_code_path(&f.path, opts.filter) && git_repo.has_file(&f.path))
        .filter(|f| kept_by(opts.filter, &f.path))
        .filter(|f| opts.include_tests || !is_test_path(&f.path))
        .map(|f| (f.path, f.commits))
        .collect()
}

pub fn run(path: &Path, opts: &TcOptions<'_>) -> Result<(), Box<dyn Error>> {
    if opts.min_degree == 0 {
        return Err("--min-degree must be at least 1".into());
    }

    let git_repo =
        GitRepo::open(path).map_err(|e| format!("not a git repository (or any parent): {e}"))?;

    let since_ts = opts.since.map(parse_since).transpose()?;

    let freqs = git_repo.file_frequencies(since_ts)?;
    if freqs.is_empty() {
        if opts.since.is_some() {
            eprintln!("No commits found in the specified time range.");
        } else {
            eprintln!("No commits found in the repository.");
        }
        return Ok(());
    }
    let freq_map = eligible_files(freqs, &git_repo, opts);

    if freq_map.is_empty() {
        let min_degree = opts.min_degree;
        eprintln!("No files with >= {min_degree} commits found. Try a lower --min-degree value.");
        return Ok(());
    }

    let co_changes = git_repo.co_changing_commits(since_ts, opts.max_changeset)?;
    if co_changes.is_empty() {
        eprintln!("No commits with multiple files found.");
        return Ok(());
    }

    let mut results = compute_coupling(&co_changes, &freq_map, opts.min_degree, opts.min_shared);

    if let Some(min_s) = opts.min_strength {
        results.retain(|r| r.strength >= min_s);
    }

    let total = results.len();

    match opts.sort_by {
        "shared" => results.sort_by_key(|r| Reverse(r.shared_commits)),
        _ => results.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }

    results.truncate(crate::report_helpers::shown_count(
        opts.output,
        opts.top,
        total,
    ));

    match opts.output {
        OutputMode::Json => print_json(&results)?,
        OutputMode::Short => print_short(&results, total),
        OutputMode::Terse => print_terse(total),
        OutputMode::Github | OutputMode::Codeclimate => {
            return Err(crate::cli::ERR_CI_FORMAT_ONLY.into());
        }
        OutputMode::Table => print_report(&results, total),
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
