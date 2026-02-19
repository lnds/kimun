//! Temporal coupling analysis — finds files that change together in git.
//!
//! Identifies implicit dependencies between files by analyzing commit
//! co-occurrence. High coupling between unrelated files may indicate
//! hidden dependencies that should be made explicit or decoupled.

pub mod analyzer;
mod report;

use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::git::GitRepo;
use crate::util::parse_since;
use crate::walk;
use analyzer::compute_coupling;
use report::{print_json, print_report};

/// Check whether a git-relative path is inside a test directory or is a test file.
fn is_test_path(path: &Path) -> bool {
    for component in path.components() {
        if let Some(name) = component.as_os_str().to_str()
            && walk::TEST_DIRS.contains(&name)
        {
            return true;
        }
    }
    walk::is_test_file(path)
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    top: usize,
    sort_by: &str,
    since: Option<&str>,
    min_degree: usize,
    min_strength: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    if min_degree == 0 {
        return Err("--min-degree must be at least 1".into());
    }

    let git_repo =
        GitRepo::open(path).map_err(|e| format!("not a git repository (or any parent): {e}"))?;

    let since_ts = since.map(parse_since).transpose()?;

    // Build freq_map: path → commits, filtering by min_degree and optionally test files
    let freqs = git_repo.file_frequencies(since_ts)?;
    if freqs.is_empty() {
        if since.is_some() {
            eprintln!("No commits found in the specified time range.");
        } else {
            eprintln!("No commits found in the repository.");
        }
        return Ok(());
    }
    let exclude_tests = !include_tests;
    let freq_map: HashMap<PathBuf, usize> = freqs
        .into_iter()
        .filter(|f| f.commits >= min_degree)
        .filter(|f| !exclude_tests || !is_test_path(&f.path))
        .map(|f| (f.path, f.commits))
        .collect();

    if freq_map.is_empty() {
        eprintln!("No files with >= {min_degree} commits found. Try a lower --min-degree value.");
        return Ok(());
    }

    // Get co-changing commit groups
    let co_changes = git_repo.co_changing_commits(since_ts)?;
    if co_changes.is_empty() {
        eprintln!("No commits with multiple files found.");
        return Ok(());
    }

    let mut results = compute_coupling(&co_changes, &freq_map, min_degree);

    // Filter by min_strength if specified
    if let Some(min_s) = min_strength {
        results.retain(|r| r.strength >= min_s);
    }

    let total = results.len();

    // Sort by chosen metric
    match sort_by {
        "shared" => results.sort_by(|a, b| b.shared_commits.cmp(&a.shared_commits)),
        _ => results.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }

    results.truncate(top);

    if json {
        print_json(&results)?;
    } else {
        print_report(&results, total);
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
