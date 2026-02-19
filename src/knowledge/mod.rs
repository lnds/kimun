//! Knowledge maps: code ownership analysis via git blame.
//!
//! Identifies bus factor risk by analyzing who owns each file's code.
//! Risk levels: Critical (one person >80%), High (60-80%), Medium
//! (2-3 people >80% combined), Low (well-distributed). Generated
//! files (lock files, minified JS) are automatically excluded.

pub mod analyzer;
mod report;

use std::error::Error;
use std::path::Path;

use crate::git::GitRepo;
use crate::util::parse_since;
use crate::walk;

use crate::report_helpers;
use analyzer::{FileOwnership, compute_ownership};
use report::{print_json, print_report};

/// Check if a file is machine-generated (lock files, minified assets,
/// protobuf output) and should be excluded from ownership analysis.
fn is_generated(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };

    matches!(
        file_name,
        "Cargo.lock"
            | "package-lock.json"
            | "yarn.lock"
            | "pnpm-lock.yaml"
            | "Gemfile.lock"
            | "poetry.lock"
            | "composer.lock"
            | "Pipfile.lock"
            | "go.sum"
    ) || file_name.ends_with(".min.js")
        || file_name.ends_with(".min.css")
        || file_name.ends_with(".bundle.js")
        || file_name.ends_with(".pb.go")
        || file_name.ends_with("_pb2.py")
        || file_name.contains(".generated.")
}

/// Run knowledge map analysis: walk source files, blame each one,
/// compute ownership concentration and risk, then output results.
pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    top: usize,
    sort_by: &str,
    since: Option<&str>,
    risk_only: bool,
) -> Result<(), Box<dyn Error>> {
    let git_repo =
        GitRepo::open(path).map_err(|e| format!("not a git repository (or any parent): {e}"))?;

    let since_ts = since.map(parse_since).transpose()?;

    // Collect recent authors (for knowledge loss detection)
    let recent_authors = if since_ts.is_some() {
        git_repo.recent_authors(since_ts)?
    } else {
        std::collections::HashSet::new()
    };

    let (walk_root, walk_prefix) = git_repo.walk_prefix(path)?;

    let exclude_tests = !include_tests;
    let mut results: Vec<FileOwnership> = Vec::new();

    for (file_path, spec) in walk::source_files(&walk_root, exclude_tests) {
        if is_generated(&file_path) {
            continue;
        }

        let rel_path = GitRepo::to_git_path(&walk_root, &walk_prefix, &file_path);

        // Run blame
        let blames = match git_repo.blame_file(&rel_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("warning: blame {}: {e}", rel_path.display());
                continue;
            }
        };

        let ownership = compute_ownership(rel_path, spec.name, &blames, &recent_authors);
        results.push(ownership);
    }

    // Filter risk-only if requested
    if risk_only {
        results.retain(|f| f.knowledge_loss);
    }

    // Sort
    match sort_by {
        "diffusion" => results.sort_by(|a, b| b.contributors.cmp(&a.contributors)),
        "risk" => results.sort_by(|a, b| {
            a.risk.sort_key().cmp(&b.risk.sort_key()).then_with(|| {
                b.ownership_pct
                    .partial_cmp(&a.ownership_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        }),
        _ => {
            // concentration: highest ownership % first
            results.sort_by(|a, b| {
                b.ownership_pct
                    .partial_cmp(&a.ownership_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    report_helpers::output_results(&mut results, top, json, print_json, print_report)
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
