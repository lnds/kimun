//! Maintainability Index computation (Visual Studio variant).
//!
//! Computes MI per file using the Visual Studio formula: no comment weight,
//! normalized to 0–100 scale, clamped at 0. Invoked via `km mi`.
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

use crate::loc::counter::LineKind;
use crate::loc::language::LanguageSpec;
use crate::report_helpers;
use crate::util::read_and_classify;
use crate::walk::WalkConfig;
use analyzer::compute_mi;
use report::{FileMIMetrics, print_json, print_report, print_short, print_terse};

/// What either variant of the index is computed from.
pub(crate) struct Inputs {
    pub volume: f64,
    pub complexity: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
}

/// Read a file into the inputs of the index. `None` for a binary file and
/// for one without Halstead or cyclomatic metrics.
pub(crate) fn inputs(
    path: &Path,
    spec: &LanguageSpec,
    exclude_tests: bool,
) -> Result<Option<Inputs>, Box<dyn Error>> {
    let Some((lines, kinds)) = read_and_classify(path, spec, exclude_tests)? else {
        return Ok(None);
    };
    let count = |kind| kinds.iter().filter(|k| **k == kind).count();
    let volume = crate::hal::analyze_content(&lines, &kinds, spec).map(|h| h.volume);
    let complexity =
        crate::cycom::analyze_content(&lines, &kinds, spec).map(|c| c.total_complexity);
    Ok(volume.zip(complexity).map(|(volume, complexity)| Inputs {
        volume,
        complexity,
        code_lines: count(LineKind::Code),
        comment_lines: count(LineKind::Comment),
    }))
}

fn analyze_file(
    path: &Path,
    spec: &LanguageSpec,
    exclude_tests: bool,
) -> Result<Option<FileMIMetrics>, Box<dyn Error>> {
    let Some(of) = inputs(path, spec, exclude_tests)? else {
        return Ok(None);
    };
    let metrics = compute_mi(of.volume, of.complexity, of.code_lines);
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
