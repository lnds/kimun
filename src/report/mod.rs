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
        OutputMode::Terse => print_terse(&report),
        OutputMode::Short => print_short(&report),
        OutputMode::Json => json::print_json(&report)?,
        OutputMode::Table => markdown::print_markdown(&report),
    }

    Ok(())
}

/// Print compact single-line output for AI consumption.
fn print_short(report: &ProjectReport) {
    let total_files: usize = report.loc.iter().map(|r| r.files).sum();
    let total_code: usize = report.loc.iter().map(|r| r.code).sum();
    let total_comment: usize = report.loc.iter().map(|r| r.comment).sum();
    let dup_pct = report.duplication.duplication_percentage;
    // avg MI from verifysoft entries
    let avg_mi = if report.mi_verifysoft.entries.is_empty() {
        0.0
    } else {
        report
            .mi_verifysoft
            .entries
            .iter()
            .map(|e| e.mi_score)
            .sum::<f64>()
            / report.mi_verifysoft.entries.len() as f64
    };
    // avg cyclomatic
    let avg_cx = if report.cyclomatic.entries.is_empty() {
        0.0
    } else {
        let total: usize = report.cyclomatic.entries.iter().map(|e| e.total).sum();
        let fns: usize = report.cyclomatic.entries.iter().map(|e| e.functions).sum();
        if fns > 0 {
            total as f64 / fns as f64
        } else {
            0.0
        }
    };
    println!(
        "report files:{} loc:{} cmt:{} dup:{:.1}% avg_mi:{:.1} avg_cx:{:.1}",
        total_files, total_code, total_comment, dup_pct, avg_mi, avg_cx,
    );
}

/// Print only the headline metric (average MI as the single value).
fn print_terse(report: &ProjectReport) {
    let avg_mi = if report.mi_verifysoft.entries.is_empty() {
        0.0
    } else {
        report
            .mi_verifysoft
            .entries
            .iter()
            .map(|e| e.mi_score)
            .sum::<f64>()
            / report.mi_verifysoft.entries.len() as f64
    };
    println!("{:.1}", avg_mi);
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
