//! Maintainability Index computation (verifysoft variant with comment weight).
//!
//! Implements the SEI/verifysoft formula: `MI = MIwoc + MIcw` where
//! `MIwoc = 171 - 5.2*ln(V) - 0.23*G - 16.2*ln(LOC)` and
//! `MIcw = 50 * sin(sqrt(2.46 * radians(PerCM)))`. Unbounded scale;
//! comment percentage boosts the score via the MIcw term.

use serde::Serialize;

/// Quality level for the verifysoft MI scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MILevel {
    Good,
    Moderate,
    Difficult,
}

impl MILevel {
    /// Classify a raw MI score into a quality level.
    pub fn from_score(score: f64) -> Self {
        if score >= 85.0 {
            Self::Good
        } else if score >= 65.0 {
            Self::Moderate
        } else {
            Self::Difficult
        }
    }

    /// Human-readable label for display in reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Good => "good",
            Self::Moderate => "moderate",
            Self::Difficult => "difficult",
        }
    }
}

impl std::fmt::Display for MILevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Per-file MI metrics including both the without-comments (MIwoc) and
/// with-comments (MIcw) components, plus comment statistics.
#[derive(Debug, Clone)]
pub struct MIMetrics {
    pub halstead_volume: f64,
    pub cyclomatic_complexity: usize,
    pub loc: usize,
    pub comment_lines: usize,
    pub comment_percent: f64,
    pub mi_woc: f64,
    pub mi_cw: f64,
    pub mi_score: f64,
    pub level: MILevel,
}

/// Compute the Maintainability Index from constituent metrics.
///
/// Formula (SEI / verifysoft.com variant, matching radon's implementation):
///   MIwoc = 171 - 5.2 × ln(V) - 0.23 × G - 16.2 × ln(LOC)
///   MIcw  = 50 × sin(√(2.46 × radians(PerCM)))
///   MI    = MIwoc + MIcw
///
/// `PerCM` is the comment percentage (0–100), converted to radians before
/// use in the formula. This ensures `sin()` receives a small argument
/// (0–1.55 rad) that is always in the positive range, so comments always
/// boost the MI score.
///
/// Returns `None` if code_lines == 0, volume <= 0, or complexity == 0.
pub fn compute_mi(
    volume: f64,
    complexity: usize,
    code_lines: usize,
    comment_lines: usize,
) -> Option<MIMetrics> {
    if code_lines == 0 || volume <= 0.0 || complexity == 0 {
        return None;
    }

    // code_lines > 0 is guaranteed by the guard above
    let total_lines = code_lines + comment_lines;
    let comment_percent = comment_lines as f64 / total_lines as f64 * 100.0;

    let mi_woc =
        171.0 - 5.2 * volume.ln() - 0.23 * complexity as f64 - 16.2 * (code_lines as f64).ln();

    // Convert percentage to radians before applying the formula (matches radon)
    let mi_cw = 50.0 * (2.46 * comment_percent.to_radians()).sqrt().sin();

    let mi_score = mi_woc + mi_cw;

    Some(MIMetrics {
        halstead_volume: volume,
        cyclomatic_complexity: complexity,
        loc: code_lines,
        comment_lines,
        comment_percent,
        mi_woc,
        mi_cw,
        mi_score,
        level: MILevel::from_score(mi_score),
    })
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
