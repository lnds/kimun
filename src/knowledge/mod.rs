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
use crate::walk::{self, WalkConfig};

use crate::report_helpers;
use analyzer::{FileOwnership, aggregate_by_author, compute_ownership};
use report::{print_json, print_report, print_summary_json, print_summary_report};

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

/// Options for knowledge map analysis.
pub struct KnowledgeOptions<'a> {
    pub json: bool,
    pub top: usize,
    pub sort_by: &'a str,
    pub since: Option<&'a str>,
    pub risk_only: bool,
    pub summary: bool,
    /// Filter to files owned by this author (case-insensitive substring match).
    pub author: Option<&'a str>,
}

/// Run knowledge map analysis: walk source files, blame each one,
/// compute ownership concentration and risk, then output results.
pub fn run(cfg: &WalkConfig<'_>, opts: &KnowledgeOptions<'_>) -> Result<(), Box<dyn Error>> {
    let git_repo = GitRepo::open(cfg.path)
        .map_err(|e| format!("not a git repository (or any parent): {e}"))?;

    let since_ts = opts.since.map(parse_since).transpose()?;

    // Collect recent authors (for knowledge loss detection)
    let recent_authors = if since_ts.is_some() {
        git_repo.recent_authors(since_ts)?
    } else {
        std::collections::HashSet::new()
    };

    let (walk_root, walk_prefix) = git_repo.walk_prefix(cfg.path)?;

    let mut results: Vec<FileOwnership> = Vec::new();

    for (file_path, spec) in walk::source_files(&walk_root, cfg.exclude_tests(), cfg.filter) {
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

    // Filter by author if requested (case-insensitive substring match on name or email)
    if let Some(author_filter) = opts.author {
        let lower = author_filter.to_lowercase();
        results.retain(|f| {
            f.primary_owner.to_lowercase().contains(&lower)
                || f.primary_email.to_lowercase().contains(&lower)
        });
    }

    // Filter risk-only if requested
    if opts.risk_only {
        results.retain(|f| f.knowledge_loss);
    }

    // Sort
    match opts.sort_by {
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

    if opts.summary {
        let mut authors = aggregate_by_author(&results);
        // In summary mode sort_by maps: concentration→files owned, diffusion→lines, risk→worst risk
        match opts.sort_by {
            "diffusion" => authors.sort_by(|a, b| b.total_lines.cmp(&a.total_lines)),
            "risk" => authors.sort_by(|a, b| a.worst_risk.sort_key().cmp(&b.worst_risk.sort_key())),
            _ => authors.sort_by(|a, b| b.files_owned.cmp(&a.files_owned)),
        }
        let limit = opts.top.min(authors.len());
        let authors = &authors[..limit];
        if opts.json {
            print_summary_json(authors)
        } else {
            print_summary_report(authors);
            Ok(())
        }
    } else {
        report_helpers::output_results(&mut results, opts.top, opts.json, print_json, print_report)
    }
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
