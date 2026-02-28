//! Dimension scoring and per-file score computation.
//!
//! Handles the normalization and aggregation of per-file metrics into
//! dimension scores (LOC-weighted means) and individual file scores
//! (weighted sums across dimensions). Each dimension uses a piecewise
//! linear curve from `normalize.rs` to map raw metrics to 0–100.

use super::analyzer::{DimensionScore, FileScore, score_to_grade};
use super::collector::FileMetrics;
use super::normalize::{
    normalize_cognitive, normalize_duplication, normalize_file_size, normalize_halstead,
    normalize_indent,
};

/// Dimension weights (must sum to 1.0).
/// Cognitive Complexity gets the most weight (30%) as the primary measure
/// of code understandability. Halstead Effort (20%) captures implementation
/// complexity. Duplication (20%) and File Size (15%) capture structural health.
/// Indentation (15%) captures nesting depth independently.
pub const W_COGCOM: f64 = 0.30;
pub const W_DUP: f64 = 0.20;
pub const W_INDENT: f64 = 0.15;
pub const W_HAL: f64 = 0.20;
pub const W_SIZE: f64 = 0.15;

/// All per-file dimension weights (excludes duplication, which is project-level).
pub const FILE_WEIGHTS: [(f64, &str); 4] = [
    (W_COGCOM, "Cogcom"),
    (W_INDENT, "Indent"),
    (W_HAL, "Halstead"),
    (W_SIZE, "Size"),
];

/// Default score for missing dimensions (neutral).
pub const MISSING_DIM_SCORE: f64 = 50.0;

/// Build the five dimension scores from per-file metrics, using LOC-weighted
/// means for per-file dimensions and project-level normalization for duplication.
pub fn build_dimensions(
    file_metrics: &[FileMetrics],
    total_loc: usize,
    dup_percent: f64,
) -> Vec<DimensionScore> {
    let cogcom_dim = weighted_mean(file_metrics, total_loc, |f| {
        f.max_cognitive.map(normalize_cognitive)
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
        dim("Cognitive Complexity", W_COGCOM, cogcom_dim),
        dim("Duplication", W_DUP, dup_dim),
        dim("Indentation Complexity", W_INDENT, indent_dim),
        dim("Halstead Effort", W_HAL, hal_dim),
        dim("File Size", W_SIZE, size_dim),
    ]
}

/// Build dimension scores for an empty project (all zeros).
pub fn build_empty_dimensions() -> Vec<DimensionScore> {
    build_dimensions(&[], 0, 0.0)
}

/// Normalize an optional metric and push an issue if the score is below 60.
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
pub fn score_file(f: &FileMetrics) -> FileScore {
    let mut issues: Vec<String> = Vec::new();
    let file_weight_sum: f64 = FILE_WEIGHTS.iter().map(|(w, _)| w).sum();

    let cogcom_s = score_dim(
        f.max_cognitive,
        normalize_cognitive,
        |v| format!("Cognitive: {v}"),
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

    let weighted_sum = cogcom_s * W_COGCOM + indent_s * W_INDENT + hal_s * W_HAL + size_s * W_SIZE;
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
///
/// Files with more code lines have proportionally more influence on
/// the dimension score. Files where the metric is unavailable (None)
/// are excluded from the mean.
pub fn weighted_mean(
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
