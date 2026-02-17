mod json;
mod markdown;

use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

use serde::Serialize;

use crate::cycom;
use crate::dups;
use crate::hal;
use crate::indent;
use crate::loc::counter::{FileStats, LineKind};
use crate::loc::report::LanguageReport;
use crate::mi;
use crate::miv;
use crate::util::read_and_classify;
use crate::walk;

/// Comprehensive project report combining all code metrics.
///
/// When serialized to JSON the structure is:
/// ```json
/// {
///   "path": "...",
///   "top": 20,
///   "include_tests": false,
///   "min_lines": 6,
///   "loc": [{ "name": "Rust", "files": 5, "blank": 20, "comment": 10, "code": 500 }],
///   "duplication": { "total_code_lines": ..., "duplicated_lines": ..., ... },
///   "indent": [{ "path": "...", "code_lines": ..., "stddev": ..., ... }],
///   "halstead": [{ "path": "...", "volume": ..., "effort": ..., "bugs": ... }],
///   "cyclomatic": [{ "path": "...", "functions": ..., "total": ..., ... }],
///   "mi_visual_studio": [{ "path": "...", "mi_score": ..., "level": "..." }],
///   "mi_verifysoft": [{ "path": "...", "mi_score": ..., "level": "..." }]
/// }
/// ```
///
/// Each per-file section includes `total_count` (total files before truncation)
/// and up to `top` entries sorted by the relevant metric.
#[derive(Debug, Serialize)]
pub struct ProjectReport {
    pub path: String,
    pub top: usize,
    pub include_tests: bool,
    pub min_lines: usize,
    pub loc: Vec<LanguageReport>,
    pub duplication: DupsSummary,
    pub indent: SectionResult<IndentEntry>,
    pub halstead: SectionResult<HalsteadEntry>,
    pub cyclomatic: SectionResult<CycomEntry>,
    pub mi_visual_studio: SectionResult<MiVisualStudioEntry>,
    pub mi_verifysoft: SectionResult<MiVerifysoftEntry>,
}

/// A section of per-file results with total count before truncation.
#[derive(Debug, Serialize)]
pub struct SectionResult<T> {
    pub description: &'static str,
    pub total_count: usize,
    pub entries: Vec<T>,
}

#[derive(Debug, Serialize)]
pub struct DupsSummary {
    pub description: &'static str,
    pub total_code_lines: usize,
    pub duplicated_lines: usize,
    pub duplication_percentage: f64,
    pub duplicate_groups: usize,
    pub files_with_duplicates: usize,
    pub largest_block: usize,
}

#[derive(Debug, Serialize)]
pub struct IndentEntry {
    pub path: String,
    pub code_lines: usize,
    pub stddev: f64,
    pub max_depth: usize,
    pub complexity: String,
}

#[derive(Debug, Serialize)]
pub struct HalsteadEntry {
    pub path: String,
    pub volume: f64,
    pub effort: f64,
    pub bugs: f64,
    pub time: f64,
}

#[derive(Debug, Serialize)]
pub struct CycomEntry {
    pub path: String,
    pub functions: usize,
    pub total: usize,
    pub max: usize,
    pub avg: f64,
    pub level: String,
}

#[derive(Debug, Serialize)]
pub struct MiVisualStudioEntry {
    pub path: String,
    pub mi_score: f64,
    pub level: String,
}

#[derive(Debug, Serialize)]
pub struct MiVerifysoftEntry {
    pub path: String,
    pub mi_score: f64,
    pub level: String,
}

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

/// Per-file analysis results collected by `analyze_file_for_report`.
struct FileReportData {
    blank: usize,
    comment_lines: usize,
    code_lines: usize,
    dup_normalized: Vec<dups::detector::NormalizedLine>,
    indent: Option<IndentEntry>,
    halstead: Option<HalsteadEntry>,
    cycom: Option<CycomEntry>,
    mi_vs: Option<MiVisualStudioEntry>,
    mi_vf: Option<MiVerifysoftEntry>,
}

/// Read, classify, and run all analyzers on a single file.
/// Returns `None` for binary files or on I/O errors.
fn analyze_file_for_report(
    file_path: &Path,
    spec: &crate::loc::language::LanguageSpec,
) -> Option<FileReportData> {
    let (lines, kinds) = match read_and_classify(file_path, spec) {
        Ok(Some(v)) => v,
        Ok(None) => return None,
        Err(e) => {
            eprintln!("warning: {}: {e}", file_path.display());
            return None;
        }
    };

    let blank = kinds.iter().filter(|k| **k == LineKind::Blank).count();
    let comment_lines = kinds.iter().filter(|k| **k == LineKind::Comment).count();
    let code_lines = kinds.iter().filter(|k| **k == LineKind::Code).count();

    let dup_normalized = dups::normalize_content(&lines, &kinds);

    let indent = indent::analyzer::analyze(&lines, &kinds, 4).map(|m| IndentEntry {
        path: file_path.display().to_string(),
        code_lines: m.code_lines,
        stddev: m.stddev,
        max_depth: m.max_depth,
        complexity: m.complexity.as_str().to_string(),
    });

    let path_str = file_path.display().to_string();
    let (halstead, volume_opt) = if let Some(h) = hal::analyze_content(&lines, &kinds, spec) {
        let vol = h.volume;
        (
            Some(HalsteadEntry {
                path: path_str.clone(),
                volume: h.volume,
                effort: h.effort,
                bugs: h.bugs,
                time: h.time,
            }),
            Some(vol),
        )
    } else {
        (None, None)
    };

    let (cycom, complexity_opt) = if let Some(c) = cycom::analyze_content(&lines, &kinds, spec) {
        (
            Some(CycomEntry {
                path: path_str.clone(),
                functions: c.functions.len(),
                total: c.total_complexity,
                max: c.max_complexity,
                avg: c.avg_complexity,
                level: c.level.as_str().to_string(),
            }),
            Some(c.total_complexity),
        )
    } else {
        (None, None)
    };

    let (mi_vs, mi_vf) = if let (Some(volume), Some(complexity)) = (volume_opt, complexity_opt) {
        let vs =
            mi::analyzer::compute_mi(volume, complexity, code_lines).map(|m| MiVisualStudioEntry {
                path: path_str.clone(),
                mi_score: m.mi_score,
                level: m.level.as_str().to_string(),
            });
        let vf =
            miv::analyzer::compute_mi(volume, complexity, code_lines, comment_lines).map(|m| {
                MiVerifysoftEntry {
                    path: path_str,
                    mi_score: m.mi_score,
                    level: m.level.as_str().to_string(),
                }
            });
        (vs, vf)
    } else {
        (None, None)
    };

    Some(FileReportData {
        blank,
        comment_lines,
        code_lines,
        dup_normalized,
        indent,
        halstead,
        cycom,
        mi_vs,
        mi_vf,
    })
}

/// Build the full project report by walking the file tree once and running
/// all analyzers per file. Returns the report struct for output or testing.
pub fn build_report(
    path: &Path,
    include_tests: bool,
    top: usize,
    min_lines: usize,
) -> Result<ProjectReport, Box<dyn Error>> {
    // LOC aggregation — counts all files without deduplication
    // (standalone `cm loc` deduplicates by content hash; report does not).
    // Duplication is surfaced explicitly in the duplication section.
    let mut stats_by_lang: HashMap<&'static str, (usize, FileStats)> = HashMap::new();

    // Dups collection
    let mut dup_files: Vec<dups::detector::NormalizedFile> = Vec::new();
    let mut total_code_lines: usize = 0;

    // Per-file metrics
    let mut indent_results: Vec<IndentEntry> = Vec::new();
    let mut hal_results: Vec<HalsteadEntry> = Vec::new();
    let mut cycom_results: Vec<CycomEntry> = Vec::new();
    let mut mi_vs_results: Vec<MiVisualStudioEntry> = Vec::new();
    let mut mi_vf_results: Vec<MiVerifysoftEntry> = Vec::new();

    for (file_path, spec) in walk::source_files(path, !include_tests) {
        let result = match analyze_file_for_report(&file_path, spec) {
            Some(r) => r,
            None => continue,
        };

        // LOC
        {
            let entry = stats_by_lang
                .entry(spec.name)
                .or_insert_with(|| (0, FileStats::default()));
            entry.0 += 1;
            entry.1.blank += result.blank;
            entry.1.comment += result.comment_lines;
            entry.1.code += result.code_lines;
        }

        // Dups
        total_code_lines += result.dup_normalized.len();
        dup_files.push(dups::detector::NormalizedFile {
            path: file_path.to_path_buf(),
            lines: result.dup_normalized,
        });

        // Per-file metric results
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

    // --- Post-walk: Dups detection ---
    // Pass quiet=true to suppress boilerplate-skip notices in report context.
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

    // --- Build LOC reports ---
    let loc_reports: Vec<LanguageReport> = {
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
    };

    // --- Sort, record totals, and truncate ---
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

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
