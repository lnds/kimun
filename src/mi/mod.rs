//! Maintainability Index computation (Visual Studio variant).
//!
//! Computes MI per file using the Visual Studio formula: no comment weight,
//! normalized to 0–100 scale, clamped at 0. Invoked via `cm mi`.
//!
//! This module directly calls `hal::analyze_file` and `cycom::analyze_file`
//! (pub(crate) functions). This creates tight coupling but avoids duplicating
//! file I/O and parsing logic. Changes to hal/cycom `analyze_file` signatures
//! must be coordinated with this module.
//!
//! Each file is read three times: once for LOC classification, once for
//! Halstead metrics (via `hal::analyze_file`), once for cyclomatic complexity
//! (via `cycom::analyze_file`). This is suboptimal but acceptable given the
//! existing per-module architecture where each analyzer owns its file I/O.

pub(crate) mod analyzer;
pub(crate) mod report;

use std::error::Error;
use std::path::Path;

use crate::loc::counter::LineKind;
use crate::loc::language::LanguageSpec;
use crate::report_helpers;
use crate::util::read_and_classify;
use crate::walk;
use analyzer::compute_mi;
use report::{FileMIMetrics, print_json, print_report};

fn analyze_file(path: &Path, spec: &LanguageSpec) -> Result<Option<FileMIMetrics>, Box<dyn Error>> {
    let (lines, kinds) = match read_and_classify(path, spec)? {
        Some(v) => v,
        None => return Ok(None),
    };
    let code_lines = kinds.iter().filter(|k| **k == LineKind::Code).count();

    let volume = match crate::hal::analyze_content(&lines, &kinds, spec) {
        Some(h) => h.volume,
        None => return Ok(None),
    };

    let complexity = match crate::cycom::analyze_content(&lines, &kinds, spec) {
        Some(c) => c.total_complexity,
        None => return Ok(None),
    };

    // compute_mi returns None only if code_lines==0, volume<=0, or complexity==0.
    // These should not occur when hal/cycom returned valid results, but guard anyway.
    let metrics = match compute_mi(volume, complexity, code_lines) {
        Some(m) => m,
        None => return Ok(None),
    };

    Ok(Some(FileMIMetrics {
        path: path.to_path_buf(),
        language: spec.name.to_string(),
        metrics,
    }))
}

pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    top: usize,
    sort_by: &str,
) -> Result<(), Box<dyn Error>> {
    let exclude_tests = !include_tests;
    let mut results = walk::collect_analysis(path, exclude_tests, analyze_file);

    // Sort: mi ascending (worst first), volume/complexity/loc descending
    match sort_by {
        "volume" => results.sort_by(|a, b| {
            b.metrics
                .halstead_volume
                .total_cmp(&a.metrics.halstead_volume)
        }),
        "complexity" => {
            results.sort_by(|a, b| {
                b.metrics
                    .cyclomatic_complexity
                    .cmp(&a.metrics.cyclomatic_complexity)
            });
        }
        "loc" => results.sort_by(|a, b| b.metrics.loc.cmp(&a.metrics.loc)),
        _ => results.sort_by(|a, b| a.metrics.mi_score.total_cmp(&b.metrics.mi_score)),
    }

    report_helpers::output_results(&mut results, top, json, print_json, print_report)
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
