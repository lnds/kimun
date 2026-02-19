//! Per-file analysis for the combined report.
//!
//! Reads, classifies, and runs all metric analyzers (indentation,
//! Halstead, cyclomatic, MI) on a single file, producing a
//! `FileReportData` bundle for aggregation. Called once per source
//! file during the report builder's filesystem walk.
use std::path::Path;

use crate::cycom;
use crate::hal;
use crate::indent;
use crate::loc::counter::LineKind;
use crate::mi;
use crate::miv;
use crate::util::read_and_classify;

use super::data::{CycomEntry, HalsteadEntry, IndentEntry, MiVerifysoftEntry, MiVisualStudioEntry};

/// Per-file analysis results collected during report building.
///
/// Each field is `Option` because not all analyzers succeed on every file
/// (e.g., empty files produce no Halstead or cyclomatic metrics).
pub struct FileReportData {
    /// Number of blank lines in the file.
    pub blank: usize,
    /// Number of comment-only lines.
    pub comment_lines: usize,
    /// Number of code lines (lines with at least some executable content).
    pub code_lines: usize,
    /// Normalized code lines for project-level duplicate detection.
    pub dup_normalized: Vec<crate::dups::detector::NormalizedLine>,
    /// Indentation complexity result (stddev, max depth).
    pub indent: Option<IndentEntry>,
    /// Halstead metrics result (volume, effort, bugs, time).
    pub halstead: Option<HalsteadEntry>,
    /// Cyclomatic complexity result (total, max, average).
    pub cycom: Option<CycomEntry>,
    /// Visual Studio MI result (0–100 normalized scale).
    pub mi_vs: Option<MiVisualStudioEntry>,
    /// Verifysoft MI result (unbounded scale with comment weight).
    pub mi_vf: Option<MiVerifysoftEntry>,
}

/// Read, classify, and run all analyzers on a single file.
/// Returns `None` for binary files or on I/O errors.
pub fn analyze_file_for_report(
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

    let dup_normalized = crate::dups::normalize_content(&lines, &kinds);

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
