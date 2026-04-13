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

/// Entry point: build the combined report and print it as markdown or JSON.
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
        OutputMode::Github => {
            return Err(crate::cli::ERR_GITHUB_ONLY.into());
        }
        OutputMode::Table => markdown::print_markdown(&report),
    }

    Ok(())
}

/// Print combined report as a single compact line.
fn print_short(report: &ProjectReport) {
    let total_code: usize = report.loc.iter().map(|l| l.code).sum();
    let langs = report.loc.len();
    let files: usize = report.loc.iter().map(|l| l.files).sum();
    println!(
        "report files:{files} langs:{langs} code:{total_code} dups:{:.1}%",
        report.duplication.duplication_percentage,
    );
}

/// Print only the total code lines.
fn print_terse(report: &ProjectReport) {
    let total_code: usize = report.loc.iter().map(|l| l.code).sum();
    println!("{total_code}");
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
