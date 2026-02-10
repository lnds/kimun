use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MILevel {
    Green,
    Yellow,
    Red,
}

impl MILevel {
    pub fn from_score(score: f64) -> Self {
        if score >= 20.0 {
            Self::Green
        } else if score >= 10.0 {
            Self::Yellow
        } else {
            Self::Red
        }
    }

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
/// Formula:
///   MI = MAX(0, (171 - 5.2 × ln(V) - 0.23 × G - 16.2 × ln(LOC)) × 100 / 171)
///
/// No comment-weight term. Result is normalized to a 0–100 scale.
///
/// Returns `None` if code_lines == 0, volume <= 0, or complexity == 0.
pub fn compute_mi(volume: f64, complexity: usize, code_lines: usize) -> Option<MIMetrics> {
    if code_lines == 0 || volume <= 0.0 || complexity == 0 {
        return None;
    }

    let raw =
        171.0 - 5.2 * volume.ln() - 0.23 * complexity as f64 - 16.2 * (code_lines as f64).ln();
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
mod tests {
    use super::*;

    /// Hand-computed reference: V=100, G=5, LOC=50
    ///   raw = 171 - 5.2*ln(100) - 0.23*5 - 16.2*ln(50)
    ///       = 171 - 23.947 - 1.15 - 63.375 = 82.528
    ///   MI  = max(0, 82.528 * 100 / 171) = 48.26
    #[test]
    fn reference_hand_computed() {
        let m = compute_mi(100.0, 5, 50).unwrap();
        assert!(
            (m.mi_score - 48.26).abs() < 0.1,
            "MI should be ~48.26, got {}",
            m.mi_score
        );
        assert_eq!(m.level, MILevel::Green, "48.26 should be Green");
    }

    /// Simple code: V=10, G=1, LOC=5
    ///   raw = 171 - 5.2*ln(10) - 0.23 - 16.2*ln(5)
    ///       = 171 - 11.97 - 0.23 - 26.06 = 132.74
    ///   MI  = 132.74 * 100 / 171 = 77.63
    #[test]
    fn simple_code_high_score() {
        let m = compute_mi(10.0, 1, 5).unwrap();
        assert!(
            (m.mi_score - 77.63).abs() < 0.1,
            "MI should be ~77.63, got {}",
            m.mi_score
        );
        assert_eq!(m.level, MILevel::Green);
    }

    /// Complex code: V=50000, G=100, LOC=2000
    ///   raw = 171 - 56.13 - 23.0 - 123.04 = -31.17
    ///   MI  = max(0, -31.17 * 100 / 171) = 0
    #[test]
    fn complex_code_clamped_to_zero() {
        let m = compute_mi(50000.0, 100, 2000).unwrap();
        assert!(
            m.mi_score.abs() < f64::EPSILON,
            "negative raw should clamp to 0, got {}",
            m.mi_score
        );
        assert_eq!(m.level, MILevel::Red);
    }

    #[test]
    fn score_never_exceeds_100() {
        let m = compute_mi(2.0, 1, 2).unwrap();
        assert!(
            m.mi_score <= 100.0,
            "MI should not exceed 100, got {}",
            m.mi_score
        );
    }

    #[test]
    fn score_never_negative() {
        let m = compute_mi(1_000_000.0, 500, 10000).unwrap();
        assert!(
            m.mi_score >= 0.0,
            "MI should never be negative, got {}",
            m.mi_score
        );
    }

    #[test]
    fn zero_code_returns_none() {
        assert!(
            compute_mi(100.0, 5, 0).is_none(),
            "code_lines=0 should return None"
        );
    }

    #[test]
    fn zero_volume_returns_none() {
        assert!(
            compute_mi(0.0, 5, 50).is_none(),
            "volume=0 should return None"
        );
    }

    #[test]
    fn negative_volume_returns_none() {
        assert!(
            compute_mi(-1.0, 5, 50).is_none(),
            "negative volume should return None"
        );
    }

    #[test]
    fn zero_complexity_returns_none() {
        assert!(
            compute_mi(100.0, 0, 50).is_none(),
            "complexity=0 should return None"
        );
    }

    #[test]
    fn level_boundaries() {
        assert_eq!(
            MILevel::from_score(20.0),
            MILevel::Green,
            "20.0 should be Green"
        );
        assert_eq!(
            MILevel::from_score(19.9),
            MILevel::Yellow,
            "19.9 should be Yellow"
        );
        assert_eq!(
            MILevel::from_score(10.0),
            MILevel::Yellow,
            "10.0 should be Yellow"
        );
        assert_eq!(MILevel::from_score(9.9), MILevel::Red, "9.9 should be Red");
        assert_eq!(MILevel::from_score(0.0), MILevel::Red, "0.0 should be Red");
    }

    #[test]
    fn extremely_large_volume() {
        let m = compute_mi(1_000_000.0, 10, 500).unwrap();
        assert!(
            m.mi_score.is_finite(),
            "MI should be finite even with V=1M, got {}",
            m.mi_score
        );
    }
}
