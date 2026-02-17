use serde::Serialize;

use super::detector::{DuplicateGroup, DuplicationSeverity};
use crate::report_helpers;

/// Breakdown of duplicate groups and lines by severity level.
#[derive(Default)]
struct SeverityBreakdown {
    critical_groups: usize,
    tolerable_groups: usize,
    critical_lines: usize,
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
#[derive(Serialize)]
pub struct DuplicationMetrics {
    pub total_code_lines: usize,
    pub duplicated_lines: usize,
    pub duplicate_groups: usize,
    pub files_with_duplicates: usize,
    pub largest_block: usize,
}

impl DuplicationMetrics {
    pub fn percentage(&self) -> f64 {
        if self.total_code_lines == 0 {
            0.0
        } else {
            (self.duplicated_lines as f64 / self.total_code_lines as f64) * 100.0
        }
    }
}

/// Classify duplication percentage into a human-readable assessment label.
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

#[derive(Serialize)]
struct JsonOutput<'a> {
    metrics: JsonMetrics,
    groups: &'a [DuplicateGroup],
}

#[derive(Serialize)]
struct JsonMetrics {
    total_code_lines: usize,
    duplicated_lines: usize,
    duplication_percentage: f64,
    duplicate_groups: usize,
    files_with_duplicates: usize,
    largest_block: usize,
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
