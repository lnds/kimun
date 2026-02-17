/// Halstead complexity metrics module.
///
/// Tokenizes source code (excluding comments and multi-line strings),
/// counts distinct and total operators/operands, and computes volume,
/// difficulty, effort, estimated bugs, and implementation time.
mod analyzer;
pub(crate) mod report;
mod rules;
mod string_mask;
mod tokenizer;

use std::error::Error;
use std::path::Path;

use crate::loc::counter::LineKind;
use crate::loc::language::LanguageSpec;
use crate::report_helpers;
use crate::util::read_and_classify;
use crate::walk;
use analyzer::compute;
use report::{FileHalsteadMetrics, print_json, print_report};
use string_mask::multi_line_string_mask;
use tokenizer::{count_tokens, rules_for};

/// Analyze pre-read content (avoids re-reading the file).
pub(crate) fn analyze_content(
    lines: &[String],
    kinds: &[LineKind],
    spec: &LanguageSpec,
) -> Option<analyzer::HalsteadMetrics> {
    let rules = rules_for(spec.name)?;
    let string_mask = multi_line_string_mask(lines, spec);

    let code_lines: Vec<&str> = lines
        .iter()
        .zip(kinds.iter())
        .zip(string_mask.iter())
        .filter(|((_, k), in_string)| **k == LineKind::Code && !*in_string)
        .map(|((line, _), _)| line.as_str())
        .collect();

    if code_lines.is_empty() {
        return None;
    }

    let counts = count_tokens(&code_lines, rules);
    compute(&counts)
}

/// Read a file from disk, classify lines, tokenize code, and compute
/// Halstead metrics. Returns `None` for binary or unsupported files.
pub(crate) fn analyze_file(
    path: &Path,
    spec: &LanguageSpec,
) -> Result<Option<FileHalsteadMetrics>, Box<dyn Error>> {
    let rules = match rules_for(spec.name) {
        Some(r) => r,
        None => return Ok(None),
    };

    let (lines, kinds) = match read_and_classify(path, spec)? {
        Some(v) => v,
        None => return Ok(None),
    };
    let string_mask = multi_line_string_mask(&lines, spec);

    // Collect only code lines that are not inside multi-line strings
    let code_lines: Vec<&str> = lines
        .iter()
        .zip(&kinds)
        .zip(&string_mask)
        .filter(|((_, k), in_string)| **k == LineKind::Code && !*in_string)
        .map(|((line, _), _)| line.as_str())
        .collect();

    if code_lines.is_empty() {
        return Ok(None);
    }

    let counts = count_tokens(&code_lines, rules);
    let metrics = match compute(&counts) {
        Some(m) => m,
        None => return Ok(None),
    };

    Ok(Some(FileHalsteadMetrics {
        path: path.to_path_buf(),
        language: spec.name.to_string(),
        metrics,
    }))
}

/// Walk source files, compute Halstead metrics for each, sort by the
/// chosen metric (effort, volume, or bugs), and print results.
pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    top: usize,
    sort_by: &str,
) -> Result<(), Box<dyn Error>> {
    let exclude_tests = !include_tests;
    let mut results = walk::collect_analysis(path, exclude_tests, analyze_file);

    // Sort by chosen metric descending
    match sort_by {
        "volume" => results.sort_by(|a, b| {
            b.metrics
                .volume
                .partial_cmp(&a.metrics.volume)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "bugs" => results.sort_by(|a, b| {
            b.metrics
                .bugs
                .partial_cmp(&a.metrics.bugs)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        _ => results.sort_by(|a, b| {
            b.metrics
                .effort
                .partial_cmp(&a.metrics.effort)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }

    report_helpers::output_results(&mut results, top, json, print_json, print_report)
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
