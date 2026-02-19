//! Report formatters for duplicate code detection.
//!
//! Provides three output modes:
//! - **Summary**: compact overview with duplication %, group count, and
//!   Rule of Three breakdown (critical vs. tolerable).
//! - **Detailed**: summary plus a listing of each duplicate group with
//!   file locations, severity label, and a code sample (up to 5 lines).
//! - **JSON**: machine-readable output combining metrics and group data.
//!
//! The Rule of Three analysis classifies duplicates as **Critical** (3+
//! occurrences, indicating a pattern that should be extracted into a shared
//! function/module) or **Tolerable** (2 occurrences, acceptable in many
//! codebases). This distinction guides refactoring priorities.
use serde::Serialize;

use super::detector::{DuplicateGroup, DuplicationSeverity};
use crate::report_helpers;

/// Counts of groups and duplicated lines split by severity level.
/// Computed in a single pass over the duplicate groups.
#[derive(Default)]
struct SeverityBreakdown {
    /// Groups with 3+ occurrences (should be extracted into shared code).
    critical_groups: usize,
    /// Groups with exactly 2 occurrences (often acceptable).
    tolerable_groups: usize,
    /// Total duplicated lines in critical groups.
    critical_lines: usize,
    /// Total duplicated lines in tolerable groups.
    tolerable_lines: usize,
}

/// Compute severity breakdown in a single pass over the groups.
fn severity_breakdown(groups: &[DuplicateGroup]) -> SeverityBreakdown {
    groups
        .iter()
        .fold(SeverityBreakdown::default(), |mut acc, g| {
            match g.severity {
                DuplicationSeverity::Critical => {
                    acc.critical_groups += 1;
                    acc.critical_lines += g.duplicated_lines();
                }
                DuplicationSeverity::Tolerable => {
                    acc.tolerable_groups += 1;
                    acc.tolerable_lines += g.duplicated_lines();
                }
            }
            acc
        })
}

/// Summary metrics for the duplication analysis.
///
/// Aggregated from all detected duplicate groups and used by both
/// the table and JSON output formatters.
#[derive(Serialize)]
pub struct DuplicationMetrics {
    /// Total production code lines analyzed (excluding blanks/comments).
    pub total_code_lines: usize,
    /// Lines that appear in at least one duplicate group.
    pub duplicated_lines: usize,
    /// Number of distinct duplicate groups found.
    pub duplicate_groups: usize,
    /// Number of files that contain at least one duplicated block.
    pub files_with_duplicates: usize,
    /// Line count of the largest single duplicate block.
    pub largest_block: usize,
}

impl DuplicationMetrics {
    /// Calculate duplication as a percentage of total code lines.
    /// Returns 0.0 when no code lines exist (avoids division by zero).
    pub fn percentage(&self) -> f64 {
        if self.total_code_lines == 0 {
            0.0
        } else {
            (self.duplicated_lines as f64 / self.total_code_lines as f64) * 100.0
        }
    }
}

/// Classify duplication percentage into a human-readable assessment label.
/// Thresholds are based on industry heuristics: <3% is exceptional for most
/// projects, while >20% signals systemic copy-paste patterns.
fn assessment(percentage: f64) -> &'static str {
    if percentage < 3.0 {
        "Excellent"
    } else if percentage < 5.0 {
        "Good"
    } else if percentage < 10.0 {
        "Moderate"
    } else if percentage < 20.0 {
        "High"
    } else {
        "Very High"
    }
}

/// Print a summary of duplication metrics with Rule of Three breakdown.
pub fn print_summary(metrics: &DuplicationMetrics, groups: &[DuplicateGroup]) {
    let separator = report_helpers::separator(68);
    let pct = metrics.percentage();

    println!("{separator}");
    println!(" Duplication Analysis");
    println!();
    println!(" Total code lines:     {:>42}", metrics.total_code_lines);
    println!(" Duplicated lines:     {:>42}", metrics.duplicated_lines);
    println!(" Duplication:          {:>41.1}%", pct);
    println!();
    println!(" Duplicate groups:     {:>42}", metrics.duplicate_groups);
    println!(
        " Files with duplicates:{:>42}",
        metrics.files_with_duplicates
    );
    if metrics.largest_block > 0 {
        println!(" Largest duplicate:    {:>37} lines", metrics.largest_block);
    }

    let sb = severity_breakdown(groups);
    if sb.critical_groups > 0 || sb.tolerable_groups > 0 {
        println!();
        println!(" Rule of Three Analysis:");
        if sb.critical_groups > 0 {
            println!(
                "   Critical duplicates (3+): {:>5} groups, {:>5} lines",
                sb.critical_groups, sb.critical_lines
            );
        }
        if sb.tolerable_groups > 0 {
            println!(
                "   Tolerable duplicates (2x):{:>5} groups, {:>5} lines",
                sb.tolerable_groups, sb.tolerable_lines
            );
        }
    }

    println!();
    println!(" Assessment:           {:>42}", assessment(pct));
    println!("{separator}");
}

/// Maximum duplicate groups shown by default (use `--show-all` to override).
pub const DEFAULT_GROUP_LIMIT: usize = 20;

/// Compute how many duplicate groups to display based on the `--show-all` flag.
pub fn display_limit(total: usize, show_all: bool) -> usize {
    if show_all {
        total
    } else {
        DEFAULT_GROUP_LIMIT.min(total)
    }
}

/// Print the summary followed by a detailed listing of each duplicate group
/// with locations, severity label, and a code sample.
pub fn print_detailed(
    metrics: &DuplicationMetrics,
    groups: &[DuplicateGroup],
    total_groups: usize,
) {
    print_summary(metrics, groups);

    if groups.is_empty() {
        return;
    }

    let separator = report_helpers::separator(68);

    println!();
    println!(" Duplicate Groups (sorted by severity, then duplicated lines)");

    for (i, group) in groups.iter().enumerate() {
        let severity_label = match group.severity {
            DuplicationSeverity::Critical => "CRITICAL",
            DuplicationSeverity::Tolerable => "TOLERABLE",
        };
        println!();
        println!("{separator}");
        println!(
            " [{}] {}: {} lines, {} occurrences ({} duplicated lines)",
            i + 1,
            severity_label,
            group.line_count,
            group.locations.len(),
            group.duplicated_lines()
        );
        println!();
        for loc in &group.locations {
            println!(
                "   {}:{}-{}",
                loc.file_path.display(),
                loc.start_line,
                loc.end_line
            );
        }
        if !group.sample.is_empty() {
            println!();
            println!(" Sample:");
            for line in &group.sample {
                println!("   {line}");
            }
            if group.line_count > group.sample.len() {
                println!("   ...");
            }
        }
    }

    println!("{separator}");

    if groups.len() < total_groups {
        println!();
        println!(
            " Showing top {} of {} duplicate groups.",
            groups.len(),
            total_groups
        );
        println!(" Use --show-all to see all groups.");
    }
}

/// JSON-serializable wrapper combining metrics and duplicate group details.
#[derive(Serialize)]
struct JsonOutput<'a> {
    metrics: JsonMetrics,
    groups: &'a [DuplicateGroup],
}

/// JSON-serializable summary of duplication metrics with assessment label.
#[derive(Serialize)]
struct JsonMetrics {
    /// Total production code lines analyzed.
    total_code_lines: usize,
    /// Lines appearing in at least one duplicate group.
    duplicated_lines: usize,
    /// Duplication as percentage of total code lines.
    duplication_percentage: f64,
    /// Number of distinct duplicate groups.
    duplicate_groups: usize,
    /// Files containing at least one duplicated block.
    files_with_duplicates: usize,
    /// Line count of the largest single duplicate block.
    largest_block: usize,
    /// Human-readable quality label (Excellent/Good/Moderate/High/Very High).
    assessment: &'static str,
}

/// Serialize duplication metrics and groups to a pretty-printed JSON string.
pub fn format_json(
    metrics: &DuplicationMetrics,
    groups: &[DuplicateGroup],
) -> Result<String, Box<dyn std::error::Error>> {
    let pct = metrics.percentage();
    let output = JsonOutput {
        metrics: JsonMetrics {
            total_code_lines: metrics.total_code_lines,
            duplicated_lines: metrics.duplicated_lines,
            duplication_percentage: pct,
            duplicate_groups: metrics.duplicate_groups,
            files_with_duplicates: metrics.files_with_duplicates,
            largest_block: metrics.largest_block,
            assessment: assessment(pct),
        },
        groups,
    };
    Ok(serde_json::to_string_pretty(&output)?)
}

/// Print duplication metrics and groups as pretty-printed JSON to stdout.
pub fn print_json(
    metrics: &DuplicationMetrics,
    groups: &[DuplicateGroup],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", format_json(metrics, groups)?);
    Ok(())
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
