/// Per-file metric collection for the score module.
///
/// Reads and classifies each source file, computes per-file metrics
/// (cognitive complexity, indent, Halstead), normalizes content for
/// duplication detection, and bundles results for project-level scoring.
use std::path::Path;

use crate::cogcom;
use crate::dups;
use crate::hal;
use crate::indent;
use crate::loc::counter::LineKind;
use crate::util::{find_test_block_start, read_and_classify};

/// Per-file raw metrics collected during the walk.
pub struct FileMetrics {
    pub path: std::path::PathBuf,
    pub code_lines: usize,
    pub max_cognitive: Option<usize>,
    pub indent_stddev: Option<f64>,
    pub halstead_effort: Option<f64>,
}

/// Result of analyzing a single file for scoring: raw metrics,
/// normalized content for duplication detection, and line count.
pub struct SingleFileResult {
    pub metrics: FileMetrics,
    pub dup_file: dups::detector::NormalizedFile,
    pub normalized_count: usize,
}

/// Analyze a single source file: read, classify, compute metrics, normalize for dups.
/// Returns `None` for binary files, non-code files, or on I/O errors.
pub fn analyze_single_file(
    file_path: &Path,
    spec: &crate::loc::language::LanguageSpec,
    exclude_tests: bool,
) -> Option<SingleFileResult> {
    let (lines, kinds) = match read_and_classify(file_path, spec) {
        Ok(Some(v)) => v,
        Ok(None) => return None,
        Err(e) => {
            eprintln!("warning: {}: {e}", file_path.display());
            return None;
        }
    };

    let code_lines = kinds.iter().filter(|k| **k == LineKind::Code).count();

    let indent_stddev = indent::analyzer::analyze(&lines, &kinds, 4).map(|m| m.stddev);

    let hal_metrics = hal::analyze_content(&lines, &kinds, spec);
    let halstead_effort = hal_metrics.as_ref().map(|h| h.effort);

    let cogcom_result = cogcom::analyze_content(&lines, &kinds, spec);
    let max_cognitive = cogcom_result.as_ref().map(|c| c.max_complexity);

    // Skip non-code files (Markdown, TOML, JSON, etc.)
    if max_cognitive.is_none() && halstead_effort.is_none() {
        return None;
    }

    let dup_end = if exclude_tests {
        find_test_block_start(&lines)
    } else {
        lines.len()
    };
    let normalized = dups::normalize_content(&lines[..dup_end], &kinds[..dup_end]);
    let normalized_count = normalized.len();

    Some(SingleFileResult {
        metrics: FileMetrics {
            path: file_path.to_path_buf(),
            code_lines,
            max_cognitive,
            indent_stddev,
            halstead_effort,
        },
        dup_file: dups::detector::NormalizedFile {
            path: file_path.to_path_buf(),
            lines: normalized,
        },
        normalized_count,
    })
}
