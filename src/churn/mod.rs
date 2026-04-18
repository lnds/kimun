/// Code churn analysis — pure change frequency per source file.
///
/// Walks source files, resolves each one's commit count and first/last
/// timestamps via git, computes a commits-per-month rate, and classifies
/// as High / Medium / Low. Unlike hotspots (churn × complexity), churn
/// shows velocity alone — useful for finding "moving targets".
pub mod analyzer;
mod report;

use std::cmp::Reverse;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

use crate::cli::OutputMode;
use crate::git::GitRepo;
use crate::util::parse_since;
use crate::walk::{self, WalkConfig};
use analyzer::{FileChurn, classify};
use report::{print_json, print_report, print_short, print_terse};

/// Run code churn analysis and print results.
///
/// Sorts by `sort_by` ("commits", "rate", or "file"), truncates to `top`,
/// and optionally restricts to commits after `since` (e.g. "6m", "1y").
pub fn run(
    cfg: &WalkConfig<'_>,
    output: OutputMode,
    top: usize,
    sort_by: &str,
    since: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let git = GitRepo::open(cfg.path)
        .map_err(|e| format!("not a git repository (or any parent): {e}"))?;

    let since_ts = since.map(parse_since).transpose()?;
    let freqs = git.file_frequencies(since_ts)?;

    if freqs.is_empty() {
        if since.is_some() {
            eprintln!("No commits found in the specified time range.");
        } else {
            eprintln!("No commits found in the repository.");
        }
        return Ok(());
    }

    let freq_map: HashMap<PathBuf, _> = freqs.into_iter().map(|f| (f.path.clone(), f)).collect();

    let (walk_root, walk_prefix) = git.walk_prefix(cfg.path)?;

    let mut files: Vec<FileChurn> = walk::source_files(&walk_root, cfg.exclude_tests(), cfg.filter)
        .into_iter()
        .filter_map(|(file_path, spec)| {
            let rel = GitRepo::to_git_path(&walk_root, &walk_prefix, &file_path);
            freq_map.get(&rel).map(|freq| {
                classify(
                    rel,
                    spec.name,
                    freq.commits,
                    freq.first_commit,
                    freq.last_commit,
                )
            })
        })
        .collect();

    match sort_by {
        "rate" => files.sort_by(|a, b| b.rate.partial_cmp(&a.rate).unwrap()),
        "file" => files.sort_by_key(|f| f.path.clone()),
        _ => files.sort_by_key(|f| Reverse(f.commits)),
    }

    files.truncate(top);

    match output {
        OutputMode::Json => print_json(&files),
        OutputMode::Short => print_short(&files),
        OutputMode::Terse => print_terse(&files),
        OutputMode::Github | OutputMode::Codeclimate => {
            return Err(crate::cli::ERR_CI_FORMAT_ONLY.into());
        }
        OutputMode::Table => print_report(&files),
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
