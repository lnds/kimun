/// Cognitive complexity analysis module (SonarSource, 2017).
///
/// Measures the difficulty of understanding code by penalizing
/// nested control flow and rewarding linear structures.
/// Levels: Simple, Moderate, Complex, VeryComplex, Extreme.
mod analyzer;
mod detection;
mod markers;
pub(crate) mod report;

use std::error::Error;
use std::path::Path;

use crate::loc::counter::LineKind;
use crate::loc::language::LanguageSpec;
use crate::util::read_and_classify;
use crate::walk::WalkConfig;
use analyzer::analyze;
use markers::cognitive_markers_for;
use report::{FileCogcomMetrics, print_json, print_per_function, print_report};

/// Analyze pre-read content (avoids re-reading the file).
pub(crate) fn analyze_content(
    lines: &[String],
    kinds: &[LineKind],
    spec: &LanguageSpec,
) -> Option<analyzer::FileCognitive> {
    let cm = cognitive_markers_for(spec.name)?;
    analyze(lines, kinds, cm)
}

/// Read a file from disk, classify lines, and compute cognitive complexity.
pub(crate) fn analyze_file(
    path: &Path,
    spec: &LanguageSpec,
) -> Result<Option<FileCogcomMetrics>, Box<dyn Error>> {
    let cm = match cognitive_markers_for(spec.name) {
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

    Ok(Some(FileCogcomMetrics {
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

/// Walk source files, compute cognitive complexity, filter/sort/truncate
/// results, and print as a table, per-function breakdown, or JSON.
pub fn run(
    cfg: &WalkConfig<'_>,
    json: bool,
    min_complexity: usize,
    top: usize,
    per_function: bool,
    sort_by: &str,
) -> Result<(), Box<dyn Error>> {
    let mut results = cfg.collect_analysis(analyze_file);

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
