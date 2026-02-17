/// Combined report module (`cm report` command).
///
/// Walks all source files once, runs every analyzer (LOC, duplication,
/// indentation, Halstead, cyclomatic, MI), and produces a unified
/// markdown or JSON report with all metrics.
mod analyzer;
pub(crate) mod data;
mod json;
mod markdown;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::Path;

use crate::dups;
use crate::loc::counter::FileStats;
use crate::loc::report::LanguageReport;
use crate::util::hash_file;
use crate::walk;

pub use data::*;

use analyzer::analyze_file_for_report;

// --- Section descriptions (used in both markdown and JSON output) ---

const DESC_DUPS: &str = "Duplicate code detection using sliding-window fingerprinting. \
    Identical normalized code blocks across files are grouped. \
    High duplication suggests opportunities for refactoring shared logic.";

const DESC_INDENT: &str = "Indentation complexity measures how deeply nested code is. \
    StdDev of indentation depth per file indicates structural complexity. \
    Higher values suggest deeply nested control flow that may be hard to follow.";

const DESC_HALSTEAD: &str = "Halstead complexity metrics based on operators and operands. \
    Volume = N * log2(n) measures implementation size. \
    Effort = D * V estimates mental effort. \
    Bugs = V / 3000 estimates delivered defects. \
    Time = E / 18 estimates development time in seconds. \
    Reference: Halstead, M.H. (1977) Elements of Software Science.";

const DESC_CYCOM: &str = "Cyclomatic complexity counts linearly independent paths through code. \
    Total = sum of all function complexities. Max = highest single function. \
    Levels: simple (1-10), moderate (11-20), complex (21-50), highly complex (>50). \
    Reference: McCabe, T.J. (1976) A Complexity Measure.";

const DESC_MI_VS: &str = "Maintainability Index (Visual Studio variant). \
    Formula: MI = max(0, (171 - 5.2*ln(V) - 0.23*G - 16.2*ln(LOC)) * 100/171). \
    Normalized 0-100 scale, no comment weight. \
    Thresholds: green (20+), yellow (10-19), red (0-9). \
    Reference: Oman, P. & Hagemeister, J. (1992).";

const DESC_MI_VF: &str = "Maintainability Index (Verifysoft variant with comment weight). \
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

/// Build the full project report by walking the file tree once and running
/// all analyzers per file. Returns the report struct for output or testing.
pub fn build_report(
    path: &Path,
    include_tests: bool,
    top: usize,
    min_lines: usize,
) -> Result<ProjectReport, Box<dyn Error>> {
    // LOC aggregation — deduplicates files by content hash (consistent with cm loc).
    let mut stats_by_lang: HashMap<&'static str, (usize, FileStats)> = HashMap::new();
    let mut seen_hashes: HashSet<u64> = HashSet::new();
    let mut dup_files: Vec<dups::detector::NormalizedFile> = Vec::new();
    let mut total_code_lines: usize = 0;

    let mut indent_results: Vec<IndentEntry> = Vec::new();
    let mut hal_results: Vec<HalsteadEntry> = Vec::new();
    let mut cycom_results: Vec<CycomEntry> = Vec::new();
    let mut mi_vs_results: Vec<MiVisualStudioEntry> = Vec::new();
    let mut mi_vf_results: Vec<MiVerifysoftEntry> = Vec::new();

    for (file_path, spec) in walk::source_files(path, !include_tests) {
        // Skip duplicate files (same content), matching cm loc behavior.
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

        total_code_lines += result.dup_normalized.len();
        dup_files.push(dups::detector::NormalizedFile {
            path: file_path.to_path_buf(),
            lines: result.dup_normalized,
        });

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

    // Duplication detection (project-level).
    let dup_groups = if dup_files.is_empty() {
        Vec::new()
    } else {
        dups::detector::detect_duplicates(&dup_files, min_lines, true)
    };
    let duplicated_lines: usize = dup_groups.iter().map(|g| g.duplicated_lines()).sum();
    let largest_block = dup_groups.iter().map(|g| g.line_count).max().unwrap_or(0);
    let files_with_dups: std::collections::HashSet<&Path> = dup_groups
        .iter()
        .flat_map(|g| g.locations.iter().map(|l| l.file_path.as_path()))
        .collect();
    let dup_percentage = if total_code_lines == 0 {
        0.0
    } else {
        duplicated_lines as f64 / total_code_lines as f64 * 100.0
    };

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
        path: path.display().to_string(),
        top,
        include_tests,
        min_lines,
        loc: loc_reports,
        duplication: DupsSummary {
            description: DESC_DUPS,
            total_code_lines,
            duplicated_lines,
            duplication_percentage: dup_percentage,
            duplicate_groups: dup_groups.len(),
            files_with_duplicates: files_with_dups.len(),
            largest_block,
        },
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

/// Convert per-language stats into sorted LOC report entries.
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

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
