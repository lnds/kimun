//! Code smell detection module.
//!
//! Detects common code smells per file: long functions, long parameter lists,
//! TODO/FIXME debt, magic numbers, and commented-out code.

mod analyzer;
mod report;
mod rules;

use std::error::Error;
use std::path::Path;

use crate::cycom::markers::markers_for;
use crate::loc::language::LanguageSpec;
use crate::util::read_and_classify;
use crate::walk::WalkConfig;

use analyzer::detect_smells;
use report::{FileSmellMetrics, print_json, print_report};

/// Read a file, classify lines, and detect smells.
fn analyze_file(
    path: &Path,
    spec: &LanguageSpec,
    max_lines: usize,
    max_params: usize,
) -> Result<Option<FileSmellMetrics>, Box<dyn Error>> {
    let markers = match markers_for(spec.name) {
        Some(m) => m,
        None => return Ok(None),
    };

    let (lines, kinds) = match read_and_classify(path, spec)? {
        Some(v) => v,
        None => return Ok(None),
    };

    let smells = match detect_smells(&lines, &kinds, markers, max_lines, max_params) {
        Some(s) => s,
        None => return Ok(None),
    };

    let total = smells.smells.len();
    Ok(Some(FileSmellMetrics {
        path: path.to_path_buf(),
        language: spec.name.to_string(),
        smells,
        total,
    }))
}

/// Walk source files, detect smells, sort by count, and output.
pub fn run(
    cfg: &WalkConfig<'_>,
    json: bool,
    top: usize,
    max_lines: usize,
    max_params: usize,
) -> Result<(), Box<dyn Error>> {
    let mut results =
        cfg.collect_analysis(|path, spec| analyze_file(path, spec, max_lines, max_params));

    // Sort by smell count descending
    results.sort_by(|a, b| b.total.cmp(&a.total));
    results.truncate(top);

    if json {
        print_json(&results)
    } else {
        print_report(&results);
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
