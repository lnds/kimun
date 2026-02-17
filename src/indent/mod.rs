pub(crate) mod analyzer;
pub(crate) mod report;

use std::error::Error;
use std::path::Path;

use crate::loc::language::LanguageSpec;
use crate::util::read_and_classify;
use crate::walk;
use analyzer::analyze;
use report::{FileIndentMetrics, print_json, print_report};

const TAB_WIDTH: usize = 4;

pub(crate) fn analyze_file(
    path: &Path,
    spec: &LanguageSpec,
) -> Result<Option<FileIndentMetrics>, Box<dyn Error>> {
    let (lines, kinds) = match read_and_classify(path, spec)? {
        Some(v) => v,
        None => return Ok(None),
    };

    let metrics = match analyze(&lines, &kinds, TAB_WIDTH) {
        Some(m) => m,
        None => return Ok(None),
    };

    Ok(Some(FileIndentMetrics {
        path: path.to_path_buf(),
        code_lines: metrics.code_lines,
        stddev: metrics.stddev,
        max_depth: metrics.max_depth,
        total_indent: metrics.total_indent,
        complexity: metrics.complexity,
    }))
}

pub fn run(path: &Path, json: bool, include_tests: bool) -> Result<(), Box<dyn Error>> {
    let exclude_tests = !include_tests;
    let mut results = walk::collect_analysis(path, exclude_tests, analyze_file);

    // Sort by stddev descending
    results.sort_by(|a, b| b.stddev.total_cmp(&a.stddev));

    if json {
        print_json(&results)?;
    } else {
        print_report(&results);
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
