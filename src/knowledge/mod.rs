//! Knowledge maps: code ownership analysis via git blame.
//!
//! Identifies bus factor risk by analyzing who owns each file's code.
//! Risk levels: Critical (one person >80%), High (60-80%), Medium
//! (2-3 people >80% combined), Low (well-distributed). Generated
//! files (lock files, minified JS) are automatically excluded.
//! `--bus-factor` counts how many authors have to leave before most of
//! the code has nobody who wrote it (`analyzer::compute_bus_factor`).

pub mod analyzer;
mod report;

use std::cmp::Reverse;
use std::collections::HashSet;
use std::error::Error;

use crate::git::{BlameInfo, GitRepo};
use crate::util::{is_generated_file, parse_since};
use crate::walk::{self, WalkConfig};

use crate::cli::OutputMode;
use crate::report_helpers;
use analyzer::{FileOwnership, aggregate_by_author, compute_bus_factor, compute_ownership};
use report::{
    print_bus_factor_json, print_bus_factor_report, print_bus_factor_short, print_bus_factor_terse,
    print_json, print_report, print_short, print_summary_json, print_summary_report,
    print_summary_short, print_summary_terse, print_terse,
};

/// Options for knowledge map analysis.
pub struct KnowledgeOptions<'a> {
    pub output: OutputMode,
    pub top: Option<usize>,
    pub sort_by: &'a str,
    pub since: Option<&'a str>,
    pub risk_only: bool,
    pub summary: bool,
    pub bus_factor: bool,
    /// Share of the code, in lines, left without an author that the bus
    /// factor counts to.
    pub bus_factor_threshold: f64,
    /// Filter to files owned by this author (case-insensitive substring match).
    pub author: Option<&'a str>,
}

/// Sort file ownership results by the given sort key.
/// "diffusion" sorts by contributor count, "risk" by risk level then ownership,
/// and anything else (default "concentration") by highest ownership percentage first.
fn sort_results(results: &mut [FileOwnership], sort_by: &str) {
    match sort_by {
        "diffusion" => results.sort_by_key(|r| Reverse(r.contributors)),
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
}

/// The ownership of each file, and the blame it was computed from.
type Collected = (Vec<FileOwnership>, Vec<Vec<BlameInfo>>);

/// Blame each code file and compute its ownership. Generated files and
/// data or prose are left out, and so are the lines of bot accounts, which
/// hold no knowledge; a file only bots wrote is left out too.
fn collect(
    cfg: &WalkConfig<'_>,
    git_repo: &GitRepo,
    recent_authors: Option<&HashSet<String>>,
) -> Result<Collected, Box<dyn Error>> {
    let (walk_root, walk_prefix) = git_repo.walk_prefix(cfg.path)?;
    let mut results = Vec::new();
    let mut file_blames = Vec::new();
    let files = walk::source_files(&walk_root, cfg.exclude_tests(), cfg.filter)
        .into_iter()
        .filter(|(path, spec)| spec.is_code() && !is_generated_file(path));
    for (file_path, spec) in files {
        let rel_path = GitRepo::to_git_path(&walk_root, &walk_prefix, &file_path);
        let mut blames = match git_repo.blame_file(&rel_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("warning: blame {}: {e}", rel_path.display());
                continue;
            }
        };
        blames.retain(|b| !b.is_bot());
        if blames.is_empty() {
            continue;
        }
        results.push(compute_ownership(
            rel_path,
            spec.name,
            &blames,
            recent_authors,
        ));
        file_blames.push(blames);
    }
    Ok((results, file_blames))
}

/// Run knowledge map analysis: walk source files, blame each one,
/// compute ownership concentration and risk, then output results.
pub fn run(cfg: &WalkConfig<'_>, opts: &KnowledgeOptions<'_>) -> Result<(), Box<dyn Error>> {
    let git_repo = GitRepo::open(cfg.path)
        .map_err(|e| format!("not a git repository (or any parent): {e}"))?;

    if opts.risk_only && opts.since.is_none() {
        return Err("--risk-only needs --since to tell who is inactive (e.g. --since 1y)".into());
    }
    let since_ts = opts.since.map(parse_since).transpose()?;

    // Authors with commits in the window; with no window, activity is unknown.
    let recent_authors = since_ts
        .map(|ts| git_repo.recent_authors(Some(ts)))
        .transpose()?;

    let (mut results, file_blames) = collect(cfg, &git_repo, recent_authors.as_ref())?;

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

    sort_results(&mut results, opts.sort_by);

    if opts.bus_factor {
        let bf = compute_bus_factor(
            &file_blames,
            recent_authors.as_ref(),
            opts.bus_factor_threshold,
        );
        return match opts.output {
            OutputMode::Json => print_bus_factor_json(&bf),
            OutputMode::Short => {
                print_bus_factor_short(&bf);
                Ok(())
            }
            OutputMode::Terse => {
                print_bus_factor_terse(&bf);
                Ok(())
            }
            OutputMode::Github | OutputMode::Codeclimate => {
                Err(crate::cli::ERR_CI_FORMAT_ONLY.into())
            }
            OutputMode::Table => {
                print_bus_factor_report(
                    &bf,
                    opts.top.unwrap_or(report_helpers::DEFAULT_TOP),
                    opts.since,
                );
                Ok(())
            }
        };
    }

    if opts.summary {
        let mut authors = aggregate_by_author(&results);
        // In summary mode sort_by maps: concentration→files owned, diffusion→lines, risk→worst risk
        match opts.sort_by {
            "diffusion" => authors.sort_by_key(|a| Reverse(a.total_lines)),
            "risk" => authors.sort_by_key(|a| a.worst_risk.sort_key()),
            _ => authors.sort_by_key(|a| Reverse(a.files_owned)),
        }
        let limit = report_helpers::shown_count(opts.output, opts.top, authors.len());
        let authors = &authors[..limit];
        match opts.output {
            OutputMode::Json => print_summary_json(authors),
            OutputMode::Short => {
                print_summary_short(authors);
                Ok(())
            }
            OutputMode::Terse => {
                print_summary_terse(authors);
                Ok(())
            }
            OutputMode::Github | OutputMode::Codeclimate => {
                Err(crate::cli::ERR_CI_FORMAT_ONLY.into())
            }
            OutputMode::Table => {
                print_summary_report(authors);
                Ok(())
            }
        }
    } else {
        let at_risk = results.iter().filter(|f| f.knowledge_loss).count();
        report_helpers::output_results(
            &results,
            opts.top,
            opts.output,
            print_json,
            |all, shown| print_report(&all[..shown], at_risk),
            print_short,
            print_terse,
        )
    }
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
