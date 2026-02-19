//! Piecewise linear normalization curves for the code health score.
//!
//! Each quality dimension (MI, complexity, duplication, indent, Halstead,
//! file size) uses a curve of breakpoints to map raw metric values to a
//! 0–100 score. Breakpoints are calibrated based on industry thresholds
//! and empirical testing across diverse codebases. Values between
//! breakpoints are linearly interpolated; values beyond the endpoints
//! are clamped.

/// A single point on a piecewise linear curve, mapping an `input` metric
/// value to an output `score` (0–100).
struct Breakpoint {
    /// Raw metric value (e.g. MI score, complexity count, dup percentage).
    input: f64,
    /// Corresponding normalized score on the 0–100 scale.
    score: f64,
}

/// Piecewise linear interpolation.  Values below the first breakpoint clamp to its score;
/// values above the last clamp to its score.
fn piecewise(value: f64, curve: &[Breakpoint]) -> f64 {
    debug_assert!(
        curve.windows(2).all(|w| w[0].input <= w[1].input),
        "Breakpoints must be sorted by input in ascending order"
    );
    if curve.is_empty() {
        return 0.0;
    }
    if value <= curve[0].input {
        return curve[0].score;
    }
    for w in curve.windows(2) {
        if value <= w[1].input {
            let frac = (value - w[0].input) / (w[1].input - w[0].input);
            return w[0].score + frac * (w[1].score - w[0].score);
        }
    }
    curve.last().unwrap().score
}

/// Maintainability Index curve: raw MI (0–171) mapped to 0–100.
/// Ranges: <0 → 0–30 (very poor), 0–40 → 30–50, 40–65 → 50–70,
/// 65–85 → 70–90 (good), 85–171 → 90–100 (excellent).
const MI_CURVE: &[Breakpoint] = &[
    Breakpoint {
        input: -100.0,
        score: 0.0,
    },
    Breakpoint {
        input: 0.0,
        score: 30.0,
    },
    Breakpoint {
        input: 40.0,
        score: 50.0,
    },
    Breakpoint {
        input: 65.0,
        score: 70.0,
    },
    Breakpoint {
        input: 85.0,
        score: 90.0,
    },
    Breakpoint {
        input: 171.0,
        score: 100.0,
    },
];

/// Cyclomatic complexity curve: max per-function complexity mapped to 0–100.
/// Simple (≤5) → 100, moderate (6–10) → 80–90, complex (10–20) → 50–80,
/// highly complex (20–50) → 5–50, extreme (50–100) → 5–0.
const COMPLEXITY_CURVE: &[Breakpoint] = &[
    Breakpoint {
        input: 5.0,
        score: 100.0,
    },
    Breakpoint {
        input: 6.0,
        score: 90.0,
    },
    Breakpoint {
        input: 10.0,
        score: 80.0,
    },
    Breakpoint {
        input: 20.0,
        score: 50.0,
    },
    Breakpoint {
        input: 50.0,
        score: 5.0,
    },
    Breakpoint {
        input: 100.0,
        score: 0.0,
    },
];

/// Duplication percentage curve: % of duplicated lines mapped to 0–100.
/// ≤5% → 100 (excellent), 10% → 80, 20% → 40, 40% → 10, 100% → 0.
const DUPLICATION_CURVE: &[Breakpoint] = &[
    Breakpoint {
        input: 5.0,
        score: 100.0,
    },
    Breakpoint {
        input: 10.0,
        score: 80.0,
    },
    Breakpoint {
        input: 20.0,
        score: 40.0,
    },
    Breakpoint {
        input: 40.0,
        score: 10.0,
    },
    Breakpoint {
        input: 100.0,
        score: 0.0,
    },
];

/// Indentation complexity curve: stddev of indent depth mapped to 0–100.
/// ≤1.0 → 100 (flat), 1.5 → 80, 2.0 → 50, 3.0 → 20, ≥5.0 → 0 (deeply nested).
const INDENT_CURVE: &[Breakpoint] = &[
    Breakpoint {
        input: 1.0,
        score: 100.0,
    },
    Breakpoint {
        input: 1.5,
        score: 80.0,
    },
    Breakpoint {
        input: 2.0,
        score: 50.0,
    },
    Breakpoint {
        input: 3.0,
        score: 20.0,
    },
    Breakpoint {
        input: 5.0,
        score: 0.0,
    },
];

/// Halstead effort-per-LOC curve: effort/code_lines mapped to 0–100.
/// ≤1000 → 100 (low cognitive load), 5000 → 70, 10000 → 40, ≥20000 → 0.
const HALSTEAD_EPL_CURVE: &[Breakpoint] = &[
    Breakpoint {
        input: 1000.0,
        score: 100.0,
    },
    Breakpoint {
        input: 5000.0,
        score: 70.0,
    },
    Breakpoint {
        input: 10000.0,
        score: 40.0,
    },
    Breakpoint {
        input: 20000.0,
        score: 0.0,
    },
];

/// File size curve: code lines mapped to 0–100.
/// ≤500 → 100 (ideal), 1000 → 60, 2000 → 20, ≥4000 → 0 (too large).
const FILE_SIZE_CURVE: &[Breakpoint] = &[
    Breakpoint {
        input: 500.0,
        score: 100.0,
    },
    Breakpoint {
        input: 1000.0,
        score: 60.0,
    },
    Breakpoint {
        input: 2000.0,
        score: 20.0,
    },
    Breakpoint {
        input: 4000.0,
        score: 0.0,
    },
];

/// Normalize a raw Maintainability Index value to a 0–100 score.
pub fn normalize_mi(mi_score: f64) -> f64 {
    piecewise(mi_score, MI_CURVE)
}

/// Normalize max cyclomatic complexity to a 0–100 score (lower complexity = higher score).
pub fn normalize_complexity(max_complexity: usize) -> f64 {
    piecewise(max_complexity as f64, COMPLEXITY_CURVE)
}

/// Normalize duplication percentage to a 0–100 score (lower duplication = higher score).
pub fn normalize_duplication(dup_percent: f64) -> f64 {
    piecewise(dup_percent, DUPLICATION_CURVE)
}

/// Normalize indentation stddev to a 0–100 score (lower stddev = higher score).
pub fn normalize_indent(stddev: f64) -> f64 {
    piecewise(stddev, INDENT_CURVE)
}

/// Normalize Halstead effort per LOC to a 0–100 score.
/// Returns 50 (neutral) when effort or code_lines is zero (missing data).
pub fn normalize_halstead(effort: f64, code_lines: usize) -> f64 {
    if effort <= 0.0 || code_lines == 0 {
        return 50.0; // neutral for missing data
    }
    piecewise(effort / code_lines as f64, HALSTEAD_EPL_CURVE)
}

/// Normalize file size (code lines) to a 0–100 score (smaller files score higher).
pub fn normalize_file_size(code_lines: usize) -> f64 {
    piecewise(code_lines as f64, FILE_SIZE_CURVE)
}

#[cfg(test)]
#[path = "normalize_test.rs"]
mod tests;
