//! Maintainability Index computation (verifysoft variant with comment weight).
//!
//! Combines Halstead Volume, Cyclomatic Complexity, LOC, and comment ratio
//! into a single maintainability score per file. Invoked via `km miv`.
//!
//! This module directly calls `hal::analyze_file` and `cycom::analyze_file`
//! (pub(crate) functions). This creates tight coupling but avoids duplicating
//! file I/O and parsing logic. Changes to hal/cycom `analyze_file` signatures
//! must be coordinated with this module.
//!
//! Each file is read once via `read_and_classify`, then the resulting lines
//! and line kinds are passed to `hal::analyze_content` and
//! `cycom::analyze_content` for Halstead and cyclomatic analysis respectively.

pub(crate) mod analyzer;
pub(crate) mod report;

use std::cmp::Reverse;
use std::error::Error;
use std::path::Path;

use crate::loc::language::LanguageSpec;
use crate::report_helpers;
use crate::walk::WalkConfig;
use analyzer::compute_mi;
use report::{FileMIMetrics, print_json, print_report, print_short, print_terse};

fn analyze_file(
    path: &Path,
    spec: &LanguageSpec,
    exclude_tests: bool,
) -> Result<Option<FileMIMetrics>, Box<dyn Error>> {
    let Some(of) = crate::mi::inputs(path, spec, exclude_tests)? else {
        return Ok(None);
    };
    let metrics = compute_mi(of.volume, of.complexity, of.code_lines, of.comment_lines);
    Ok(metrics.map(|metrics| FileMIMetrics {
        path: path.to_path_buf(),
        language: spec.name.to_string(),
        metrics,
    }))
}

pub fn run(
    cfg: &WalkConfig<'_>,
    output: crate::cli::OutputMode,
    top: Option<usize>,
    sort_by: &str,
) -> Result<(), Box<dyn Error>> {
    let mut results = cfg.collect_analysis(analyze_file);

    // Sort: mi ascending (worst first), volume/complexity/loc descending
    match sort_by {
        "volume" => results.sort_by(|a, b| {
            b.metrics
                .halstead_volume
                .total_cmp(&a.metrics.halstead_volume)
        }),
        "complexity" => {
            results.sort_by_key(|r| Reverse(r.metrics.cyclomatic_complexity));
        }
        "loc" => results.sort_by_key(|r| Reverse(r.metrics.loc)),
        _ => results.sort_by(|a, b| a.metrics.mi_score.total_cmp(&b.metrics.mi_score)),
    }

    report_helpers::output_results(
        &results,
        top,
        output,
        print_json,
        print_report,
        print_short,
        print_terse,
    )
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
