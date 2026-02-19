//! Maintainability Index computation (Visual Studio variant).
//!
//! Implements the VS formula: `MI = MAX(0, (171 - 5.2*ln(V) - 0.23*G -
//! 16.2*ln(LOC)) * 100/171)`. Normalizes to 0–100 with no comment weight.
//! Traffic-light levels: Green (≥20), Yellow (10-19), Red (<10).

use serde::Serialize;

/// Traffic-light level for the Visual Studio MI scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MILevel {
    Green,
    Yellow,
    Red,
}

impl MILevel {
    /// Classify a normalized MI score (0–100) into a traffic-light level.
    pub fn from_score(score: f64) -> Self {
        if score >= 20.0 {
            Self::Green
        } else if score >= 10.0 {
            Self::Yellow
        } else {
            Self::Red
        }
    }

    /// Human-readable label for display in reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Red => "red",
        }
    }
}

impl std::fmt::Display for MILevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Per-file MI metrics: input values (volume, complexity, LOC) and
/// the computed MI score with its traffic-light classification.
#[derive(Debug, Clone)]
pub struct MIMetrics {
    pub halstead_volume: f64,
    pub cyclomatic_complexity: usize,
    pub loc: usize,
    pub mi_score: f64,
    pub level: MILevel,
}

/// Compute the Maintainability Index using the Visual Studio variant.
///
/// Formula (from Microsoft's Code Metrics documentation):
///   MI = MAX(0, (171 - 5.2 × ln(V) - 0.23 × G - 16.2 × ln(LOC)) × 100 / 171)
///
/// The `× 100 / 171` normalization maps the raw 0–171 range to a 0–100 scale,
/// as specified by Visual Studio's implementation.
/// <https://learn.microsoft.com/en-us/visualstudio/code-quality/code-metrics-maintainability-index-range-and-meaning>
///
/// No comment-weight term. Result is clamped at 0 (never negative).
///
/// Returns `None` if `code_lines == 0`, `volume <= 0`, or `complexity == 0`.
/// In practice `volume <= 0` means the Halstead tokenizer found no tokens
/// (e.g., file is empty or only comments), and `complexity == 0` means no
/// decision points were detected.
pub fn compute_mi(volume: f64, complexity: usize, code_lines: usize) -> Option<MIMetrics> {
    if code_lines == 0 || volume <= 0.0 || complexity == 0 {
        return None;
    }

    let raw =
        171.0 - 5.2 * volume.ln() - 0.23 * complexity as f64 - 16.2 * (code_lines as f64).ln();
    // Normalize to 0–100 and clamp at 0 (VS formula)
    let mi_score = f64::max(0.0, raw * 100.0 / 171.0);

    Some(MIMetrics {
        halstead_volume: volume,
        cyclomatic_complexity: complexity,
        loc: code_lines,
        mi_score,
        level: MILevel::from_score(mi_score),
    })
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
