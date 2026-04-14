//! Code smell detection module.
//!
//! Detects common code smells per file: long functions, long parameter lists,
//! TODO/FIXME debt, magic numbers, and commented-out code.

mod analyzer;
mod report;
mod rules;

use std::error::Error;
use std::path::{Path, PathBuf};

use crate::cli::OutputMode;
use crate::cycom::markers::markers_for;
use crate::loc::language::{LanguageSpec, detect};
use crate::util::read_and_classify;
use crate::walk::WalkConfig;

use analyzer::detect_smells;
use report::{FileSmellMetrics, print_github, print_json, print_report, print_short, print_terse};

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

/// Analyze an explicit list of file paths for smells.
/// Skips paths that are not recognized source files or no longer exist.
/// Used by `--files` and `--since-ref` to limit analysis to a PR's changed files.
pub fn run_on_files(
    paths: &[PathBuf],
    output: OutputMode,
    top: usize,
    max_lines: usize,
    max_params: usize,
) -> Result<(), Box<dyn Error>> {
    let mut results: Vec<FileSmellMetrics> = Vec::new();

    for path in paths {
        let spec = match detect(path) {
            Some(s) => s,
            None => continue,
        };
        match analyze_file(path, spec, max_lines, max_params) {
            Ok(Some(m)) => results.push(m),
            Ok(None) => {}
            Err(e) => eprintln!("warning: {}: {e}", path.display()),
        }
    }

    if results.is_empty() {
        if output == OutputMode::Json {
            return report::print_json(&[]);
        }
        println!("No recognized source files in the provided list.");
        return Ok(());
    }

    results.sort_by(|a, b| b.total.cmp(&a.total));
    results.truncate(top);

    dispatch_output(output, &results)
}

/// Walk source files, detect smells, sort by count, and output.
pub fn run(
    cfg: &WalkConfig<'_>,
    output: OutputMode,
    top: usize,
    max_lines: usize,
    max_params: usize,
) -> Result<(), Box<dyn Error>> {
    let mut results =
        cfg.collect_analysis(|path, spec| analyze_file(path, spec, max_lines, max_params));

    // Sort by smell count descending
    results.sort_by(|a, b| b.total.cmp(&a.total));
    results.truncate(top);

    dispatch_output(output, &results)
}

fn dispatch_output(output: OutputMode, results: &[FileSmellMetrics]) -> Result<(), Box<dyn Error>> {
    match output {
        OutputMode::Json => print_json(results)?,
        OutputMode::Github => print_github(results),
        OutputMode::Short => print_short(results),
        OutputMode::Terse => print_terse(results),
        OutputMode::Table => print_report(results),
    }
    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
