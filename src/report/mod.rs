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

use crate::cli::OutputMode;
use crate::walk::WalkConfig;

pub use data::*;

pub use builder::build_report;

/// Entry point: build the combined report and print it.
/// Output format is selected by `output`: table (markdown), JSON, short, or terse.
pub fn run(
    cfg: &WalkConfig<'_>,
    output: OutputMode,
    top: usize,
    min_lines: usize,
) -> Result<(), Box<dyn Error>> {
    let report = build_report(cfg, top, min_lines)?;

    match output {
        OutputMode::Json => json::print_json(&report)?,
        OutputMode::Short => print_short(&report),
        OutputMode::Terse => print_terse(&report),
        OutputMode::Table => markdown::print_markdown(&report),
    }

    Ok(())
}

/// Print a compact single-line summary of the combined report.
fn print_short(report: &ProjectReport) {
    let total_code: usize = report.loc.iter().map(|l| l.code).sum();
    let total_comment: usize = report.loc.iter().map(|l| l.comment).sum();
    let total_files: usize = report.loc.iter().map(|l| l.files).sum();
    println!(
        "report files:{} loc:{} cmt:{} dup:{:.1}% dup_groups:{} langs:{}",
        total_files,
        total_code,
        total_comment,
        report.duplication.duplication_percentage,
        report.duplication.duplicate_groups,
        report.loc.len()
    );
}

/// Print only the total lines of code (headline metric for the combined report).
fn print_terse(report: &ProjectReport) {
    let total_code: usize = report.loc.iter().map(|l| l.code).sum();
    println!("{total_code}");
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
