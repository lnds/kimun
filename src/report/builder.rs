//! Report builder: walks the file tree once and constructs a `ProjectReport`.
//!
//! Reads each source file once via the analyzer, collects per-file metrics
//! for every dimension (LOC, indentation, Halstead, cyclomatic, MI), and
//! runs project-level duplication detection after the walk completes.
//! Sections are sorted (worst first) and truncated to `top` entries.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::Path;

use crate::dups;
use crate::loc::counter::FileStats;
use crate::loc::report::LanguageReport;
use crate::util::hash_file;
use crate::walk::WalkConfig;

use super::analyzer::analyze_file_for_report;
use super::data::*;

// --- Section descriptions (used in both markdown and JSON output) ---
// These short paragraphs appear in the report to explain each metric
// section for users unfamiliar with the specific measurements.

/// Section description for duplicate code detection (shown in report output).
pub const DESC_DUPS: &str = "Duplicate code detection using sliding-window fingerprinting. \
    Identical normalized code blocks across files are grouped. \
    High duplication suggests opportunities for refactoring shared logic.";

/// Section description for indentation complexity.
pub const DESC_INDENT: &str = "Indentation complexity measures how deeply nested code is. \
    StdDev of indentation depth per file indicates structural complexity. \
    Higher values suggest deeply nested control flow that may be hard to follow.";

/// Section description for Halstead complexity metrics.
pub const DESC_HALSTEAD: &str = "Halstead complexity metrics based on operators and operands. \
    Volume = N * log2(n) measures implementation size. \
    Effort = D * V estimates mental effort. \
    Bugs = V / 3000 estimates delivered defects. \
    Time = E / 18 estimates development time in seconds. \
    Reference: Halstead, M.H. (1977) Elements of Software Science.";

/// Section description for cyclomatic complexity.
pub const DESC_CYCOM: &str = "Cyclomatic complexity counts linearly independent paths through code. \
    Total = sum of all function complexities. Max = highest single function. \
    Levels: simple (1-10), moderate (11-20), complex (21-50), highly complex (>50). \
    Reference: McCabe, T.J. (1976) A Complexity Measure.";

/// Section description for MI (Visual Studio variant, 0–100 scale).
pub const DESC_MI_VS: &str = "Maintainability Index (Visual Studio variant). \
    Formula: MI = max(0, (171 - 5.2*ln(V) - 0.23*G - 16.2*ln(LOC)) * 100/171). \
    Normalized 0-100 scale, no comment weight. \
    Thresholds: green (20+), yellow (10-19), red (0-9). \
    Reference: Oman, P. & Hagemeister, J. (1992).";

/// Section description for MI (verifysoft variant with comment weight).
pub const DESC_MI_VF: &str = "Maintainability Index (Verifysoft variant with comment weight). \
    Formula: MI = MIwoc + 50*sin(sqrt(2.46*rad(PerCM))). \
    Unbounded scale; comment percentage boosts the score. \
    Thresholds: good (85+), moderate (65-84), difficult (<65). \
    Reference: verifysoft.com Maintainability Index.";

/// Sort a vector, record its total length, then truncate to `top` entries.
fn sort_truncate<T>(
    v: &mut Vec<T>,
    top: usize,
    cmp: impl Fn(&T, &T) -> std::cmp::Ordering,
) -> usize {
    v.sort_by(cmp);
    let total = v.len();
    v.truncate(top);
    total
}

/// Build the full project report by walking the file tree once and running
/// all analyzers per file. Returns the report struct for output or testing.
pub fn build_report(
    cfg: &WalkConfig<'_>,
    top: usize,
    min_lines: usize,
) -> Result<ProjectReport, Box<dyn Error>> {
    // Accumulators for each metric dimension. Populated during the file walk
    // and consumed when building the final ProjectReport struct.
    // Per-language LOC counters (language name → file count + stats).
    let mut stats_by_lang: HashMap<&'static str, (usize, FileStats)> = HashMap::new();
    // Content hashes for deduplication (same content → skip).
    let mut seen_hashes: HashSet<u64> = HashSet::new();
    // Normalized source for project-level duplicate detection.
    let mut dup_files: Vec<dups::detector::NormalizedFile> = Vec::new();
    // Running total of code lines for duplication percentage.
    let mut total_code_lines: usize = 0;

    // Per-file metric accumulators — populated during the walk.
    let mut indent_results: Vec<IndentEntry> = Vec::new();
    let mut hal_results: Vec<HalsteadEntry> = Vec::new();
    let mut cycom_results: Vec<CycomEntry> = Vec::new();
    let mut mi_vs_results: Vec<MiVisualStudioEntry> = Vec::new();
    let mut mi_vf_results: Vec<MiVerifysoftEntry> = Vec::new();

    // Walk source files, skipping test directories/files when requested.
    // Each file is analyzed once and its results distributed to all accumulators.
    for (file_path, spec) in cfg.source_files() {
        // Skip duplicate files (same content), matching km loc behavior.
        if let Some(h) = hash_file(&file_path)
            && !seen_hashes.insert(h)
        {
            continue;
        }
        let result = match analyze_file_for_report(&file_path, spec) {
            Some(r) => r,
            None => continue,
        };

        {
            let entry = stats_by_lang
                .entry(spec.name)
                .or_insert_with(|| (0, FileStats::default()));
            entry.0 += 1;
            entry.1.blank += result.blank;
            entry.1.comment += result.comment_lines;
            entry.1.code += result.code_lines;
        }

        // Collect normalized lines for cross-file duplication detection.
        total_code_lines += result.dup_normalized.len();
        dup_files.push(dups::detector::NormalizedFile {
            path: file_path.to_path_buf(),
            lines: result.dup_normalized,
        });

        // Push metric results into dimension-specific accumulators.
        if let Some(e) = result.indent {
            indent_results.push(e);
        }
        if let Some(e) = result.halstead {
            hal_results.push(e);
        }
        if let Some(e) = result.cycom {
            cycom_results.push(e);
        }
        if let Some(e) = result.mi_vs {
            mi_vs_results.push(e);
        }
        if let Some(e) = result.mi_vf {
            mi_vf_results.push(e);
        }
    }

    // Duplication detection runs at project level (cross-file), not per-file.
    // This happens after the walk so all normalized files are available.
    let dup_summary = build_dup_summary(&dup_files, total_code_lines, min_lines);

    // Build LOC reports sorted by code lines descending.
    let loc_reports = build_loc_reports(stats_by_lang);

    // Sort, record totals, and truncate each section.
    let indent_total = sort_truncate(&mut indent_results, top, |a, b| {
        b.stddev.total_cmp(&a.stddev)
    });
    let hal_total = sort_truncate(&mut hal_results, top, |a, b| b.effort.total_cmp(&a.effort));
    let cycom_total = sort_truncate(&mut cycom_results, top, |a, b| b.total.cmp(&a.total));
    let mi_vs_total = sort_truncate(&mut mi_vs_results, top, |a, b| {
        a.mi_score.total_cmp(&b.mi_score)
    });
    let mi_vf_total = sort_truncate(&mut mi_vf_results, top, |a, b| {
        a.mi_score.total_cmp(&b.mi_score)
    });

    Ok(ProjectReport {
        path: cfg.path.display().to_string(),
        top,
        include_tests: cfg.include_tests,
        min_lines,
        loc: loc_reports,
        duplication: dup_summary,
        indent: SectionResult {
            description: DESC_INDENT,
            total_count: indent_total,
            entries: indent_results,
        },
        halstead: SectionResult {
            description: DESC_HALSTEAD,
            total_count: hal_total,
            entries: hal_results,
        },
        cyclomatic: SectionResult {
            description: DESC_CYCOM,
            total_count: cycom_total,
            entries: cycom_results,
        },
        mi_visual_studio: SectionResult {
            description: DESC_MI_VS,
            total_count: mi_vs_total,
            entries: mi_vs_results,
        },
        mi_verifysoft: SectionResult {
            description: DESC_MI_VF,
            total_count: mi_vf_total,
            entries: mi_vf_results,
        },
    })
}

/// Run project-level duplication detection and build the summary.
///
/// Cross-file detection runs after all files have been normalized,
/// since duplicates can span different source files. The `min_lines`
/// parameter controls the minimum block size for detection (default 6).
fn build_dup_summary(
    dup_files: &[dups::detector::NormalizedFile],
    total_code_lines: usize,
    min_lines: usize,
) -> DupsSummary {
    // Detect duplicates across all normalized files (cross-file matching).
    let dup_groups = if dup_files.is_empty() {
        Vec::new()
    } else {
        dups::detector::detect_duplicates(dup_files, min_lines, true)
    };
    // Aggregate group-level stats into project-level summary.
    let duplicated_lines: usize = dup_groups.iter().map(|g| g.duplicated_lines()).sum();
    let largest_block = dup_groups.iter().map(|g| g.line_count).max().unwrap_or(0);
    let files_with_dups: HashSet<&Path> = dup_groups
        .iter()
        .flat_map(|g| g.locations.iter().map(|l| l.file_path.as_path()))
        .collect();
    let dup_percentage = if total_code_lines == 0 {
        0.0
    } else {
        duplicated_lines as f64 / total_code_lines as f64 * 100.0
    };

    DupsSummary {
        description: DESC_DUPS,
        total_code_lines,
        duplicated_lines,
        duplication_percentage: dup_percentage,
        duplicate_groups: dup_groups.len(),
        files_with_duplicates: files_with_dups.len(),
        largest_block,
    }
}

/// Convert per-language stats into sorted LOC report entries.
/// Sorted by code lines descending, with language name as tiebreaker.
fn build_loc_reports(
    stats_by_lang: HashMap<&'static str, (usize, FileStats)>,
) -> Vec<LanguageReport> {
    let mut reports: Vec<LanguageReport> = stats_by_lang
        .into_iter()
        .map(|(name, (files, fs))| LanguageReport {
            name: name.to_string(),
            files,
            blank: fs.blank,
            comment: fs.comment,
            code: fs.code,
        })
        .collect();
    reports.sort_by(|a, b| b.code.cmp(&a.code).then_with(|| a.name.cmp(&b.name)));
    reports
}
