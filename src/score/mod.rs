/// Overall code health score computation.
///
/// Walks source files, computes per-file metrics (MI, cyclomatic, indent,
/// Halstead, file size), detects project-level duplication, normalizes
/// each dimension to 0–100, and produces a LOC-weighted aggregate score
/// with letter grade (A++ to F--).
pub(crate) mod analyzer;
mod collector;
mod normalize;
mod report;

use std::error::Error;
use std::path::Path;

use crate::dups;
use crate::walk;

use analyzer::{DimensionScore, FileScore, ProjectScore, compute_project_score, score_to_grade};
use collector::{FileMetrics, analyze_single_file};
use normalize::{
    normalize_complexity, normalize_duplication, normalize_file_size, normalize_halstead,
    normalize_indent, normalize_mi,
};
use report::{print_json, print_report};

/// Dimension weights (must sum to 1.0).
/// MI gets the most weight (30%) because it's the most comprehensive metric
/// (combines Halstead volume, cyclomatic complexity, LOC, and comment ratio).
/// Halstead Effort (15%) uses per-LOC normalization to avoid penalizing large files.
/// Comment Ratio was removed (verifysoft MI already includes a comment weight term).
const W_MI: f64 = 0.30;
const W_CYCOM: f64 = 0.20;
const W_DUP: f64 = 0.15;
const W_INDENT: f64 = 0.15;
const W_HAL: f64 = 0.15;
const W_SIZE: f64 = 0.05;

/// All per-file dimension weights (excludes duplication, which is project-level).
const FILE_WEIGHTS: [(f64, &str); 5] = [
    (W_MI, "MI"),
    (W_CYCOM, "Cycom"),
    (W_INDENT, "Indent"),
    (W_HAL, "Halstead"),
    (W_SIZE, "Size"),
];

/// Default score for missing dimensions (neutral).
const MISSING_DIM_SCORE: f64 = 50.0;

/// Entry point: compute and display the project health score.
/// Outputs either a formatted table or JSON depending on the `json` flag.
pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    bottom: usize,
    min_lines: usize,
) -> Result<(), Box<dyn Error>> {
    let score = compute_score(path, include_tests, bottom, min_lines)?;

    // Show target in header when user specified an explicit path (not ".")
    let target = path.to_str().filter(|s| *s != ".").map(|s| s.to_string());

    if json {
        print_json(&score, target.as_deref())?;
    } else {
        print_report(&score, bottom, target.as_deref());
    }

    Ok(())
}

/// Walk all source files, compute per-file and project-level metrics,
/// normalize each dimension, and produce the final `ProjectScore`.
fn compute_score(
    path: &Path,
    include_tests: bool,
    bottom: usize,
    min_lines: usize,
) -> Result<ProjectScore, Box<dyn Error>> {
    let exclude_tests = !include_tests;
    let mut file_metrics: Vec<FileMetrics> = Vec::new();
    let mut dup_files: Vec<dups::detector::NormalizedFile> = Vec::new();
    let mut total_code_lines: usize = 0;

    for (file_path, spec) in walk::source_files(path, exclude_tests) {
        if let Some(result) = analyze_single_file(&file_path, spec, exclude_tests) {
            total_code_lines += result.normalized_count;
            dup_files.push(result.dup_file);
            file_metrics.push(result.metrics);
        }
    }

    // Duplication (project-level)
    let dup_groups = if dup_files.is_empty() {
        Vec::new()
    } else {
        dups::detector::detect_duplicates(&dup_files, min_lines, true)
    };
    let duplicated_lines: usize = dup_groups.iter().map(|g| g.duplicated_lines()).sum();
    let dup_percent = if total_code_lines == 0 {
        0.0
    } else {
        duplicated_lines as f64 / total_code_lines as f64 * 100.0
    };

    let total_loc: usize = file_metrics.iter().map(|f| f.code_lines).sum();
    let files_analyzed = file_metrics.len();

    if files_analyzed == 0 {
        let dimensions = build_empty_dimensions();
        return Ok(ProjectScore {
            score: 0.0,
            grade: score_to_grade(0.0),
            files_analyzed: 0,
            total_loc: 0,
            dimensions,
            needs_attention: vec![],
        });
    }

    let dimensions = build_dimensions(&file_metrics, total_loc, dup_percent);
    let project_score = compute_project_score(&dimensions);
    let mut file_scores: Vec<FileScore> = file_metrics.iter().map(score_file).collect();
    file_scores.sort_by(|a, b| a.score.total_cmp(&b.score));
    file_scores.truncate(bottom);

    Ok(ProjectScore {
        score: project_score,
        grade: score_to_grade(project_score),
        files_analyzed,
        total_loc,
        dimensions,
        needs_attention: file_scores,
    })
}

/// Build the six dimension scores from per-file metrics, using LOC-weighted
/// means for per-file dimensions and project-level normalization for duplication.
fn build_dimensions(
    file_metrics: &[FileMetrics],
    total_loc: usize,
    dup_percent: f64,
) -> Vec<DimensionScore> {
    let mi_dim = weighted_mean(file_metrics, total_loc, |f| f.mi_score.map(normalize_mi));
    let cycom_dim = weighted_mean(file_metrics, total_loc, |f| {
        f.max_complexity.map(normalize_complexity)
    });
    let indent_dim = weighted_mean(file_metrics, total_loc, |f| {
        f.indent_stddev.map(normalize_indent)
    });
    let hal_dim = weighted_mean(file_metrics, total_loc, |f| {
        f.halstead_effort
            .map(|e| normalize_halstead(e, f.code_lines))
    });
    let size_dim = weighted_mean(file_metrics, total_loc, |f| {
        Some(normalize_file_size(f.code_lines))
    });
    let dup_dim = normalize_duplication(dup_percent);

    let dim = |name, weight, score| DimensionScore {
        name,
        weight,
        score,
        grade: score_to_grade(score),
    };

    vec![
        dim("Maintainability Index", W_MI, mi_dim),
        dim("Cyclomatic Complexity", W_CYCOM, cycom_dim),
        dim("Duplication", W_DUP, dup_dim),
        dim("Indentation Complexity", W_INDENT, indent_dim),
        dim("Halstead Effort", W_HAL, hal_dim),
        dim("File Size", W_SIZE, size_dim),
    ]
}

/// Normalize an optional metric, push an issue if the score is below threshold.
fn score_dim<T: Copy>(
    value: Option<T>,
    normalize: impl Fn(T) -> f64,
    label: impl Fn(T) -> String,
    issues: &mut Vec<String>,
) -> f64 {
    match value {
        Some(v) => {
            let s = normalize(v);
            if s < 60.0 {
                issues.push(label(v));
            }
            s
        }
        None => MISSING_DIM_SCORE,
    }
}

/// Score a single file across all per-file dimensions (excludes duplication)
/// and collect human-readable issue strings for metrics below threshold.
fn score_file(f: &FileMetrics) -> FileScore {
    let mut issues: Vec<String> = Vec::new();
    let file_weight_sum: f64 = FILE_WEIGHTS.iter().map(|(w, _)| w).sum();

    let mi_s = score_dim(
        f.mi_score,
        normalize_mi,
        |v| format!("MI: {v:.0}"),
        &mut issues,
    );
    let cycom_s = score_dim(
        f.max_complexity,
        normalize_complexity,
        |v| format!("Complexity: {v}"),
        &mut issues,
    );
    let indent_s = score_dim(
        f.indent_stddev,
        normalize_indent,
        |v| format!("Indent: {v:.1}"),
        &mut issues,
    );
    let hal_s = score_dim(
        f.halstead_effort,
        |e| normalize_halstead(e, f.code_lines),
        |v| format!("Effort: {v:.0}"),
        &mut issues,
    );

    let size_s = normalize_file_size(f.code_lines);
    if f.code_lines > 1000 {
        issues.push(format!("Size: {} LOC", f.code_lines));
    }

    let weighted_sum =
        mi_s * W_MI + cycom_s * W_CYCOM + indent_s * W_INDENT + hal_s * W_HAL + size_s * W_SIZE;
    let file_score = weighted_sum / file_weight_sum;

    FileScore {
        path: f.path.clone(),
        score: file_score,
        grade: score_to_grade(file_score),
        loc: f.code_lines,
        issues,
    }
}

/// LOC-weighted mean of a normalized dimension across all files.
fn weighted_mean(
    files: &[FileMetrics],
    total_loc: usize,
    score_fn: impl Fn(&FileMetrics) -> Option<f64>,
) -> f64 {
    if total_loc == 0 {
        return 0.0;
    }
    let mut weighted_sum = 0.0;
    let mut weight_sum = 0usize;
    for f in files {
        if let Some(s) = score_fn(f) {
            let w = f.code_lines.max(1); // at least 1 to count the file
            weighted_sum += s * w as f64;
            weight_sum += w;
        }
    }
    if weight_sum == 0 {
        0.0
    } else {
        weighted_sum / weight_sum as f64
    }
}

/// Build dimension scores for an empty project (all zeros).
fn build_empty_dimensions() -> Vec<DimensionScore> {
    build_dimensions(&[], 0, 0.0)
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
