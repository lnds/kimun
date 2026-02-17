/// Data structures for the combined project report.
///
/// These structs are populated by the report builder and consumed
/// by both the markdown and JSON formatters.
use serde::Serialize;

use crate::loc::report::LanguageReport;

/// Comprehensive project report combining all code metrics.
///
/// Contains LOC breakdown, duplication summary, and per-file entries
/// for indentation, Halstead, cyclomatic, and MI (both variants).
/// Each per-file section includes `total_count` (before truncation)
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

/// Project-level duplication summary with line counts and percentages.
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

/// Per-file indentation complexity entry for the combined report.
#[derive(Debug, Serialize)]
pub struct IndentEntry {
    pub path: String,
    pub code_lines: usize,
    pub stddev: f64,
    pub max_depth: usize,
    pub complexity: String,
}

/// Per-file Halstead metrics entry for the combined report.
#[derive(Debug, Serialize)]
pub struct HalsteadEntry {
    pub path: String,
    pub volume: f64,
    pub effort: f64,
    pub bugs: f64,
    pub time: f64,
}

/// Per-file cyclomatic complexity entry for the combined report.
#[derive(Debug, Serialize)]
pub struct CycomEntry {
    pub path: String,
    pub functions: usize,
    pub total: usize,
    pub max: usize,
    pub avg: f64,
    pub level: String,
}

/// Per-file Visual Studio MI entry for the combined report.
#[derive(Debug, Serialize)]
pub struct MiVisualStudioEntry {
    pub path: String,
    pub mi_score: f64,
    pub level: String,
}

/// Per-file verifysoft MI entry for the combined report.
#[derive(Debug, Serialize)]
pub struct MiVerifysoftEntry {
    pub path: String,
    pub mi_score: f64,
    pub level: String,
}
