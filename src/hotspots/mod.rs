//! Hotspot analysis — identifies high-risk files that change often and are complex.
//!
//! Combines git change frequency with cyclomatic or indentation complexity
//! to produce a hotspot score (commits x complexity). Files with high scores
//! are the most impactful refactoring targets.

mod report;

use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::git::GitRepo;
use crate::util::parse_since;
use crate::walk;
use report::{print_json, print_report};

/// A file's hotspot data: how often it changes (commits) and how complex
/// it is, combined into a score = commits × complexity.
pub struct FileHotspot {
    pub path: PathBuf,
    pub language: String,
    pub commits: usize,
    pub complexity: usize,
    pub score: usize,
}

/// Compute complexity for a file using the chosen metric.
/// Returns None if the file cannot be analyzed.
fn compute_complexity(
    file_path: &Path,
    spec: &crate::loc::language::LanguageSpec,
    metric: &str,
) -> Result<Option<usize>, Box<dyn Error>> {
    match metric {
        "cycom" => match crate::cycom::analyze_file(file_path, spec)? {
            Some(c) => Ok(Some(c.total_complexity)),
            None => Ok(None),
        },
        "cogcom" => match crate::cogcom::analyze_file(file_path, spec)? {
            Some(c) => Ok(Some(c.total_complexity)),
            None => Ok(None),
        },
        _ => match crate::indent::analyze_file(file_path, spec)? {
            Some(m) => Ok(Some(m.total_indent)),
            None => Ok(None),
        },
    }
}

/// Identify hotspot files by combining git change frequency with code
/// complexity. Opens the git repo, walks source files, computes complexity
/// per file, and sorts by the chosen metric (score, commits, or complexity).
pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    top: usize,
    sort_by: &str,
    since: Option<&str>,
    complexity_metric: &str,
) -> Result<(), Box<dyn Error>> {
    let git_repo =
        GitRepo::open(path).map_err(|e| format!("not a git repository (or any parent): {e}"))?;

    let since_ts = since.map(parse_since).transpose()?;

    // Build a HashMap of relative path → commits
    let freqs = git_repo.file_frequencies(since_ts)?;
    if freqs.is_empty() {
        if since.is_some() {
            eprintln!("No commits found in the specified time range.");
        } else {
            eprintln!("No commits found in the repository.");
        }
        return Ok(());
    }
    let freq_map: HashMap<PathBuf, usize> =
        freqs.into_iter().map(|f| (f.path, f.commits)).collect();

    let (walk_root, walk_prefix) = git_repo.walk_prefix(path)?;

    let exclude_tests = !include_tests;
    let mut results: Vec<FileHotspot> = Vec::new();

    for (file_path, spec) in walk::source_files(&walk_root, exclude_tests) {
        let rel_path = GitRepo::to_git_path(&walk_root, &walk_prefix, &file_path);

        // Look up commits from git history (before expensive analysis)
        let commits = match freq_map.get(&rel_path) {
            Some(&c) => c,
            None => continue,
        };

        // Compute complexity (only for files with git history)
        let complexity = match compute_complexity(&file_path, spec, complexity_metric) {
            Ok(Some(c)) => c,
            Ok(None) => continue,
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
                continue;
            }
        };

        let score = commits * complexity;

        results.push(FileHotspot {
            path: rel_path,
            language: spec.name.to_string(),
            commits,
            complexity,
            score,
        });
    }

    // Sort by chosen metric descending
    match sort_by {
        "commits" => results.sort_by(|a, b| b.commits.cmp(&a.commits)),
        "complexity" => results.sort_by(|a, b| b.complexity.cmp(&a.complexity)),
        _ => results.sort_by(|a, b| b.score.cmp(&a.score)),
    }

    results.truncate(top);

    if json {
        print_json(&results, complexity_metric)?;
    } else {
        print_report(&results, complexity_metric);
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
