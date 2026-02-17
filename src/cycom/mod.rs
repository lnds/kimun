mod analyzer;
mod detection;
mod markers;
pub(crate) mod report;

use std::error::Error;
use std::path::Path;

use crate::loc::counter::LineKind;
use crate::loc::language::LanguageSpec;
use crate::util::read_and_classify;
use crate::walk;
use analyzer::analyze;
use markers::markers_for;
use report::{FileCycomMetrics, print_json, print_per_function, print_report};

/// Analyze pre-read content (avoids re-reading the file).
pub(crate) fn analyze_content(
    lines: &[String],
    kinds: &[LineKind],
    spec: &LanguageSpec,
) -> Option<analyzer::FileComplexity> {
    let cm = markers_for(spec.name)?;
    analyze(lines, kinds, cm)
}

pub(crate) fn analyze_file(
    path: &Path,
    spec: &LanguageSpec,
) -> Result<Option<FileCycomMetrics>, Box<dyn Error>> {
    let cm = match markers_for(spec.name) {
        Some(m) => m,
        None => return Ok(None),
    };

    let (lines, kinds) = match read_and_classify(path, spec)? {
        Some(v) => v,
        None => return Ok(None),
    };

    let fc = match analyze(&lines, &kinds, cm) {
        Some(fc) => fc,
        None => return Ok(None),
    };

    Ok(Some(FileCycomMetrics {
        path: path.to_path_buf(),
        language: spec.name.to_string(),
        function_count: fc.functions.len(),
        avg_complexity: fc.avg_complexity,
        max_complexity: fc.max_complexity,
        total_complexity: fc.total_complexity,
        level: fc.level,
        functions: fc.functions,
    }))
}

pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    min_complexity: usize,
    top: usize,
    per_function: bool,
    sort_by: &str,
) -> Result<(), Box<dyn Error>> {
    let exclude_tests = !include_tests;
    let mut results = walk::collect_analysis(path, exclude_tests, analyze_file);

    // Filter by min_complexity
    if min_complexity > 1 {
        results.retain(|f| f.max_complexity >= min_complexity);
    }

    // Sort by chosen metric descending
    match sort_by {
        "max" => results.sort_by(|a, b| b.max_complexity.cmp(&a.max_complexity)),
        "avg" => results.sort_by(|a, b| {
            b.avg_complexity
                .partial_cmp(&a.avg_complexity)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        _ => results.sort_by(|a, b| b.total_complexity.cmp(&a.total_complexity)),
    }

    // Limit to top N
    results.truncate(top);

    if json {
        print_json(&results)?;
    } else if per_function {
        print_per_function(&results);
    } else {
        print_report(&results);
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
