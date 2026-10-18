//! Dimension scoring and per-file score computation.
//!
//! Handles the normalization and aggregation of per-file metrics into
//! dimension scores (LOC-weighted means) and individual file scores
//! (weighted sums across dimensions). Each dimension uses a piecewise
//! linear curve from `normalize.rs` to map raw metrics to 0–100.
//!
//! Supports three scoring models:
//! - **Cognitive** (default): 5 dimensions with cognitive complexity.
//! - **Cognitive 0.35** (v0.14 to v0.35): the same, with File Size at 15%.
//! - **Legacy** (v0.13): 6 dimensions with MI + cyclomatic complexity.

use super::ScoringModel;
use super::analyzer::{DimensionScore, FileScore, score_to_grade};
use super::collector::FileMetrics;
use super::normalize::{
    normalize_cognitive, normalize_complexity, normalize_duplication, normalize_file_size,
    normalize_halstead, normalize_indent, normalize_mi, normalize_mi_vs,
};

// ─── Cognitive model weights ─────────────────────────────────────────

/// Dimension weights of a cognitive model (must sum to 1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CognitiveWeights {
    pub cogcom: f64,
    pub dup: f64,
    pub indent: f64,
    pub hal: f64,
    pub size: f64,
}

impl CognitiveWeights {
    /// Sum of the per-file weights: all but duplication, which is project-level.
    pub fn file_sum(&self) -> f64 {
        self.cogcom + self.indent + self.hal + self.size
    }
}

/// Weights of the cognitive model (default). Cognitive Complexity weighs the
/// most, as the primary measure of understandability. File Size weighs the
/// least: size already drives the other dimensions, and defects grow less
/// than proportionally with the size of a module.
pub const COGNITIVE: CognitiveWeights = CognitiveWeights {
    cogcom: 0.32,
    dup: 0.21,
    indent: 0.16,
    hal: 0.21,
    size: 0.10,
};

/// Weights of the cognitive model from v0.14 to v0.35, File Size at 15%.
pub const COGNITIVE_0_35: CognitiveWeights = CognitiveWeights {
    cogcom: 0.30,
    dup: 0.20,
    indent: 0.15,
    hal: 0.20,
    size: 0.15,
};

// ─── Legacy model weights (v0.13) ────────────────────────────────────

/// Dimension weights for the legacy model (must sum to 1.0).
pub const W_MI: f64 = 0.30;
pub const W_CYCOM: f64 = 0.20;
pub const W_DUP_LEGACY: f64 = 0.15;
pub const W_INDENT_LEGACY: f64 = 0.15;
pub const W_HAL_LEGACY: f64 = 0.15;
pub const W_SIZE_LEGACY: f64 = 0.05;

/// All per-file dimension weights for legacy model (excludes duplication).
pub const FILE_WEIGHTS_LEGACY: [(f64, &str); 5] = [
    (W_MI, "MI"),
    (W_CYCOM, "Cycom"),
    (W_INDENT_LEGACY, "Indent"),
    (W_HAL_LEGACY, "Halstead"),
    (W_SIZE_LEGACY, "Size"),
];

// ─── Shared ──────────────────────────────────────────────────────────

/// Default score for missing dimensions (neutral).
pub const MISSING_DIM_SCORE: f64 = 50.0;

/// Build dimension scores for the active scoring model.
pub fn build_dimensions(
    file_metrics: &[FileMetrics],
    total_loc: usize,
    dup_percent: f64,
    model: &ScoringModel,
) -> Vec<DimensionScore> {
    match model {
        ScoringModel::Cognitive | ScoringModel::Cognitive035 | ScoringModel::ExperimentalMi => {
            build_cognitive_dimensions(file_metrics, total_loc, dup_percent, model)
        }
        ScoringModel::Legacy => build_legacy_dimensions(file_metrics, total_loc, dup_percent),
    }
}

/// Build dimension scores for an empty project.
pub fn build_empty_dimensions(model: &ScoringModel) -> Vec<DimensionScore> {
    build_dimensions(&[], 0, 0.0, model)
}

/// Score a single file using the active scoring model.
pub fn score_file(f: &FileMetrics, model: &ScoringModel) -> FileScore {
    match model {
        ScoringModel::Cognitive | ScoringModel::Cognitive035 | ScoringModel::ExperimentalMi => {
            score_file_cognitive(f, model)
        }
        ScoringModel::Legacy => score_file_legacy(f),
    }
}

// ─── Cognitive model (v0.14+) ────────────────────────────────────────

/// What measures the effort of reading a file in a cognitive model:
/// Halstead Effort, or in the experimental model the Maintainability Index
/// (Visual Studio variant), which weighs Halstead volume, cyclomatic
/// complexity and size together.
#[derive(Clone, Copy)]
enum Effort {
    Halstead,
    MaintainabilityIndex,
}

impl Effort {
    fn of(model: &ScoringModel) -> Self {
        match model {
            ScoringModel::ExperimentalMi => Self::MaintainabilityIndex,
            _ => Self::Halstead,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Halstead => "Halstead Effort",
            Self::MaintainabilityIndex => "Maintainability Index",
        }
    }

    /// The file's score on this dimension, 0–100.
    fn score(self, f: &FileMetrics) -> Option<f64> {
        match self {
            Self::Halstead => f
                .halstead_effort
                .map(|e| normalize_halstead(e, f.code_lines)),
            Self::MaintainabilityIndex => f.mi_vs.map(normalize_mi_vs),
        }
    }
}

/// Build the five cognitive-model dimension scores from per-file metrics.
fn build_cognitive_dimensions(
    file_metrics: &[FileMetrics],
    total_loc: usize,
    dup_percent: f64,
    model: &ScoringModel,
) -> Vec<DimensionScore> {
    let w = model.cognitive_weights();
    let effort = Effort::of(model);
    let cogcom_dim = weighted_mean(file_metrics, total_loc, |f| {
        f.max_cognitive.map(normalize_cognitive)
    });
    let indent_dim = weighted_mean(file_metrics, total_loc, |f| {
        f.indent_stddev.map(normalize_indent)
    });
    let hal_dim = weighted_mean(file_metrics, total_loc, |f| effort.score(f));
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
        dim("Cognitive Complexity", w.cogcom, cogcom_dim),
        dim("Duplication", w.dup, dup_dim),
        dim("Indentation Complexity", w.indent, indent_dim),
        dim(effort.name(), w.hal, hal_dim),
        dim("File Size", w.size, size_dim),
    ]
}

/// Score a single file using the cognitive model.
/// Excludes duplication (project-level only). The weighted sum uses absolute
/// dimension weights divided by the per-file weight sum, so the result is
/// renormalized to 0–100 but not directly comparable to the project score.
fn score_file_cognitive(f: &FileMetrics, model: &ScoringModel) -> FileScore {
    let w = model.cognitive_weights();
    let mut issues: Vec<String> = Vec::new();

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
    let hal_s = match Effort::of(model) {
        Effort::Halstead => score_dim(
            f.halstead_effort,
            |e| normalize_halstead(e, f.code_lines),
            |v| format!("Effort: {v:.0}"),
            &mut issues,
        ),
        Effort::MaintainabilityIndex => score_dim(
            f.mi_vs,
            normalize_mi_vs,
            |v| format!("MI: {v:.0}"),
            &mut issues,
        ),
    };

    let size_s = normalize_file_size(f.code_lines);
    if f.code_lines > 1000 {
        issues.push(format!("Size: {} LOC", f.code_lines));
    }

    let weighted_sum = cogcom_s * w.cogcom + indent_s * w.indent + hal_s * w.hal + size_s * w.size;
    let file_score = weighted_sum / w.file_sum();

    FileScore {
        path: f.path.clone(),
        score: file_score,
        grade: score_to_grade(file_score),
        loc: f.code_lines,
        issues,
    }
}

// ─── Legacy model (v0.13) ────────────────────────────────────────────

/// Build the six legacy-model dimension scores from per-file metrics.
fn build_legacy_dimensions(
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
        dim("Duplication", W_DUP_LEGACY, dup_dim),
        dim("Indentation Complexity", W_INDENT_LEGACY, indent_dim),
        dim("Halstead Effort", W_HAL_LEGACY, hal_dim),
        dim("File Size", W_SIZE_LEGACY, size_dim),
    ]
}

/// Score a single file using the legacy model.
/// Excludes duplication (project-level only). The weighted sum uses absolute
/// dimension weights divided by the per-file weight sum, so the result is
/// renormalized to 0–100 but not directly comparable to the project score.
fn score_file_legacy(f: &FileMetrics) -> FileScore {
    let mut issues: Vec<String> = Vec::new();
    let file_weight_sum: f64 = FILE_WEIGHTS_LEGACY.iter().map(|(w, _)| w).sum();

    let mi_s = score_dim(
        f.mi_score,
        normalize_mi,
        |v| format!("MI: {v:.1}"),
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

    let weighted_sum = mi_s * W_MI
        + cycom_s * W_CYCOM
        + indent_s * W_INDENT_LEGACY
        + hal_s * W_HAL_LEGACY
        + size_s * W_SIZE_LEGACY;
    let file_score = weighted_sum / file_weight_sum;

    FileScore {
        path: f.path.clone(),
        score: file_score,
        grade: score_to_grade(file_score),
        loc: f.code_lines,
        issues,
    }
}

// ─── Shared helpers ──────────────────────────────────────────────────

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

#[cfg(test)]
#[path = "scoring_test.rs"]
mod tests;
