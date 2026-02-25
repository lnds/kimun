//! Combined report module (`km report` command).
//!
//! Walks all source files once, runs every analyzer (LOC, duplication,
//! indentation, Halstead, cyclomatic, MI), and produces a unified
//! markdown or JSON report with all metrics.
//!
//! The single-walk design avoids reading files multiple times — each file
//! is read and classified once by the analyzer, then all metric computations
//! share the same parsed content. Duplication detection happens at the
//! project level after all files have been normalized.

/// Per-file analysis results are collected in the analyzer submodule.
mod analyzer;
/// Report builder: walks files and constructs the `ProjectReport`.
mod builder;
/// Data structures for the combined report (sections, entries, summaries).
pub(crate) mod data;
/// JSON serialization of the combined report.
mod json;
/// Markdown (table) formatting of the combined report.
mod markdown;

use std::error::Error;
use std::path::Path;

pub use data::*;

pub use builder::build_report;

/// Entry point: build the combined report and print it as markdown or JSON.
pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    top: usize,
    min_lines: usize,
) -> Result<(), Box<dyn Error>> {
    let report = build_report(path, include_tests, top, min_lines)?;

    if json {
        json::print_json(&report)?;
    } else {
        markdown::print_markdown(&report);
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
