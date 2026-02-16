use std::fmt;
use std::path::PathBuf;

use serde::Serialize;

/// Letter grade from A++ (97-100) to F-- (0-39).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Grade {
    #[serde(rename = "A++")]
    APlusPlus,
    #[serde(rename = "A+")]
    APlus,
    #[serde(rename = "A")]
    A,
    #[serde(rename = "A-")]
    AMinus,
    #[serde(rename = "B+")]
    BPlus,
    #[serde(rename = "B")]
    B,
    #[serde(rename = "B-")]
    BMinus,
    #[serde(rename = "C+")]
    CPlus,
    #[serde(rename = "C")]
    C,
    #[serde(rename = "C-")]
    CMinus,
    #[serde(rename = "D+")]
    DPlus,
    #[serde(rename = "D")]
    D,
    #[serde(rename = "D-")]
    DMinus,
    #[serde(rename = "F")]
    F,
    #[serde(rename = "F--")]
    FMinusMinus,
}

impl Grade {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::APlusPlus => "A++",
            Self::APlus => "A+",
            Self::A => "A",
            Self::AMinus => "A-",
            Self::BPlus => "B+",
            Self::B => "B",
            Self::BMinus => "B-",
            Self::CPlus => "C+",
            Self::C => "C",
            Self::CMinus => "C-",
            Self::DPlus => "D+",
            Self::D => "D",
            Self::DMinus => "D-",
            Self::F => "F",
            Self::FMinusMinus => "F--",
        }
    }
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn score_to_grade(score: f64) -> Grade {
    if score >= 97.0 {
        Grade::APlusPlus
    } else if score >= 93.0 {
        Grade::APlus
    } else if score >= 90.0 {
        Grade::A
    } else if score >= 87.0 {
        Grade::AMinus
    } else if score >= 83.0 {
        Grade::BPlus
    } else if score >= 80.0 {
        Grade::B
    } else if score >= 77.0 {
        Grade::BMinus
    } else if score >= 73.0 {
        Grade::CPlus
    } else if score >= 70.0 {
        Grade::C
    } else if score >= 67.0 {
        Grade::CMinus
    } else if score >= 63.0 {
        Grade::DPlus
    } else if score >= 60.0 {
        Grade::D
    } else if score >= 57.0 {
        Grade::DMinus
    } else if score >= 40.0 {
        Grade::F
    } else {
        Grade::FMinusMinus
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DimensionScore {
    pub name: &'static str,
    pub weight: f64,
    pub score: f64,
    pub grade: Grade,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileScore {
    pub path: PathBuf,
    pub score: f64,
    pub grade: Grade,
    pub loc: usize,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectScore {
    pub score: f64,
    pub grade: Grade,
    pub files_analyzed: usize,
    pub total_loc: usize,
    pub dimensions: Vec<DimensionScore>,
    pub needs_attention: Vec<FileScore>,
}

/// Weighted sum of dimension scores.
pub fn compute_project_score(dimensions: &[DimensionScore]) -> f64 {
    dimensions.iter().map(|d| d.score * d.weight).sum()
}

// --- Normalization functions ---
// Each takes a raw metric value and returns a score in 0-100.

/// Normalize Maintainability Index (verifysoft variant).
/// Calibrated against 18 well-known open source projects.
/// Piecewise mapping:
///   85+ (good) → 90-100, 65-85 (moderate) → 70-90,
///   40-65 (difficult) → 50-70, 0-40 (very difficult) → 30-50,
///   <0 (pathological) → 0-30.
pub fn normalize_mi(mi_score: f64) -> f64 {
    if mi_score >= 171.0 {
        100.0
    } else if mi_score >= 85.0 {
        // good: 85→90, 171→100
        90.0 + (mi_score - 85.0) * 10.0 / 86.0
    } else if mi_score >= 65.0 {
        // moderate: 65→70, 85→90
        70.0 + (mi_score - 65.0)
    } else if mi_score >= 40.0 {
        // difficult: 40→50, 65→70
        50.0 + (mi_score - 40.0) * 20.0 / 25.0
    } else if mi_score >= 0.0 {
        // very difficult: 0→30, 40→50
        30.0 + mi_score * 20.0 / 40.0
    } else {
        // pathological: -100→0, 0→30
        (30.0 + mi_score * 30.0 / 100.0).max(0.0)
    }
}

/// Normalize max cyclomatic complexity per file.
/// Piecewise: ≤5→100, 6-10→80-90, 11-20→50-80, 21-50→5-50, 51+→0.
pub fn normalize_complexity(max_complexity: usize) -> f64 {
    let c = max_complexity as f64;
    if c <= 5.0 {
        100.0
    } else if c <= 10.0 {
        // 6→90, 10→80 — linear interpolation
        90.0 - (c - 6.0) * 10.0 / 4.0
    } else if c <= 20.0 {
        // 11→80, 20→50
        80.0 - (c - 10.0) * 30.0 / 10.0
    } else if c <= 50.0 {
        // 21→50, 50→5
        50.0 - (c - 20.0) * 45.0 / 30.0
    } else {
        0.0
    }
}

/// Normalize project duplication percentage.
/// Calibrated: <5%→100, 5-10%→80-100, 10-20%→40-80, 20-40%→10-40, ≥40%→0-10.
pub fn normalize_duplication(dup_percent: f64) -> f64 {
    if dup_percent < 5.0 {
        100.0
    } else if dup_percent < 10.0 {
        // 5→100, 10→80
        100.0 - (dup_percent - 5.0) * 20.0 / 5.0
    } else if dup_percent < 20.0 {
        // 10→80, 20→40
        80.0 - (dup_percent - 10.0) * 40.0 / 10.0
    } else if dup_percent < 40.0 {
        // 20→40, 40→10
        40.0 - (dup_percent - 20.0) * 30.0 / 20.0
    } else {
        // 40→10, 100→0
        (10.0 - (dup_percent - 40.0) * 10.0 / 60.0).max(0.0)
    }
}

/// Normalize indentation complexity (stddev).
/// <1.0→100, 1.0-1.5→80-100, 1.5-2.0→50-80, 2.0-3.0→20-50, ≥3.0→0-20.
pub fn normalize_indent(stddev: f64) -> f64 {
    if stddev < 1.0 {
        100.0
    } else if stddev < 1.5 {
        // 1.0→100, 1.5→80
        100.0 - (stddev - 1.0) * 20.0 / 0.5
    } else if stddev < 2.0 {
        // 1.5→80, 2.0→50
        80.0 - (stddev - 1.5) * 30.0 / 0.5
    } else if stddev < 3.0 {
        // 2.0→50, 3.0→20
        50.0 - (stddev - 2.0) * 30.0 / 1.0
    } else {
        // 3.0→20, linear to 0
        (20.0 - (stddev - 3.0) * 20.0 / 2.0).max(0.0)
    }
}

/// Normalize Halstead effort per LOC (effort / code_lines).
/// Piecewise: ≤1000→100, 1000-5000→70-100, 5000-10000→40-70, 10000-20000→0-40, ≥20000→0.
pub fn normalize_halstead(effort: f64, code_lines: usize) -> f64 {
    if effort <= 0.0 || code_lines == 0 {
        return 50.0; // neutral for missing data
    }
    let epl = effort / code_lines as f64;
    if epl <= 1000.0 {
        100.0
    } else if epl <= 5000.0 {
        // 1000→100, 5000→70
        100.0 - (epl - 1000.0) * 30.0 / 4000.0
    } else if epl <= 10000.0 {
        // 5000→70, 10000→40
        70.0 - (epl - 5000.0) * 30.0 / 5000.0
    } else if epl <= 20000.0 {
        // 10000→40, 20000→0
        40.0 - (epl - 10000.0) * 40.0 / 10000.0
    } else {
        0.0
    }
}

/// Normalize file size (code lines). Optimal range ≤500 → 100.
/// Calibrated: penalizes only files >500 LOC. Most well-structured files are <500.
pub fn normalize_file_size(code_lines: usize) -> f64 {
    let loc = code_lines as f64;
    if loc <= 500.0 {
        100.0
    } else if loc <= 1000.0 {
        // 500→100, 1000→60
        100.0 - (loc - 500.0) * 40.0 / 500.0
    } else if loc <= 2000.0 {
        // 1000→60, 2000→20
        60.0 - (loc - 1000.0) * 40.0 / 1000.0
    } else {
        // >2000: continues down to 0
        (20.0 - (loc - 2000.0) * 20.0 / 2000.0).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- normalize_mi ---

    #[test]
    fn test_normalize_mi_pathological() {
        // -100 → 0
        let s = normalize_mi(-100.0);
        assert!((s - 0.0).abs() < 0.01, "MI=-100 should score 0, got {s}");
    }

    #[test]
    fn test_normalize_mi_zero() {
        // 0 → 30
        let s = normalize_mi(0.0);
        assert!((s - 30.0).abs() < 0.01, "MI=0 should score 30, got {s}");
    }

    #[test]
    fn test_normalize_mi_very_difficult() {
        // MI=20 → 30 + 20*20/40 = 40
        let s = normalize_mi(20.0);
        assert!((s - 40.0).abs() < 0.01, "MI=20 should score 40, got {s}");
    }

    #[test]
    fn test_normalize_mi_difficult() {
        // MI=40 → 50
        let s = normalize_mi(40.0);
        assert!((s - 50.0).abs() < 0.01, "MI=40 should score 50, got {s}");
    }

    #[test]
    fn test_normalize_mi_moderate_boundary() {
        // MI=65 → 70
        let s = normalize_mi(65.0);
        assert!((s - 70.0).abs() < 0.01, "MI=65 should score 70, got {s}");
    }

    #[test]
    fn test_normalize_mi_good_boundary() {
        // MI=85 → 90
        let s = normalize_mi(85.0);
        assert!((s - 90.0).abs() < 0.01, "MI=85 should score 90, got {s}");
    }

    #[test]
    fn test_normalize_mi_max() {
        let s = normalize_mi(171.0);
        assert!((s - 100.0).abs() < 0.01, "MI=171 should score 100, got {s}");
    }

    #[test]
    fn test_normalize_mi_below_min() {
        let s = normalize_mi(-200.0);
        assert!((s - 0.0).abs() < 0.01, "MI=-200 should clamp to 0, got {s}");
    }

    #[test]
    fn test_normalize_mi_above_max() {
        let s = normalize_mi(200.0);
        assert!(
            (s - 100.0).abs() < 0.01,
            "MI=200 should clamp to 100, got {s}"
        );
    }

    #[test]
    fn test_normalize_mi_monotonic() {
        // Score should increase monotonically with MI
        let values = [
            -100.0, -50.0, 0.0, 20.0, 40.0, 65.0, 75.0, 85.0, 120.0, 171.0,
        ];
        for window in values.windows(2) {
            let lo = normalize_mi(window[0]);
            let hi = normalize_mi(window[1]);
            assert!(
                hi >= lo,
                "normalize_mi should be monotonic: MI={} → {lo}, MI={} → {hi}",
                window[0],
                window[1]
            );
        }
    }

    // --- normalize_complexity ---

    #[test]
    fn test_normalize_complexity_1() {
        assert!((normalize_complexity(1) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_complexity_5() {
        assert!((normalize_complexity(5) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_complexity_10() {
        let s = normalize_complexity(10);
        assert!((s - 80.0).abs() < 0.01, "complexity=10 → 80, got {s}");
    }

    #[test]
    fn test_normalize_complexity_20() {
        let s = normalize_complexity(20);
        assert!((s - 50.0).abs() < 0.01, "complexity=20 → 50, got {s}");
    }

    #[test]
    fn test_normalize_complexity_50() {
        let s = normalize_complexity(50);
        assert!((s - 5.0).abs() < 0.01, "complexity=50 → 5, got {s}");
    }

    #[test]
    fn test_normalize_complexity_100() {
        assert!((normalize_complexity(100) - 0.0).abs() < 0.01);
    }

    // --- normalize_duplication ---

    #[test]
    fn test_normalize_duplication_0() {
        assert!((normalize_duplication(0.0) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_duplication_5() {
        assert!((normalize_duplication(5.0) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_duplication_10() {
        assert!((normalize_duplication(10.0) - 80.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_duplication_20() {
        assert!((normalize_duplication(20.0) - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_duplication_40() {
        assert!((normalize_duplication(40.0) - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_duplication_70() {
        let s = normalize_duplication(70.0);
        assert!(s >= 0.0 && s < 10.0, "dup=70% should be near 0, got {s}");
    }

    #[test]
    fn test_normalize_duplication_100() {
        assert!((normalize_duplication(100.0) - 0.0).abs() < 0.01);
    }

    // --- normalize_indent ---

    #[test]
    fn test_normalize_indent_low() {
        assert!((normalize_indent(0.5) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_indent_1() {
        assert!((normalize_indent(1.0) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_indent_1_5() {
        assert!((normalize_indent(1.5) - 80.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_indent_2() {
        assert!((normalize_indent(2.0) - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_indent_3() {
        assert!((normalize_indent(3.0) - 20.0).abs() < 0.01);
    }

    // --- normalize_halstead ---

    #[test]
    fn test_normalize_halstead_low_effort_per_loc() {
        // 100 effort / 100 lines = 1 epl → 100
        assert!((normalize_halstead(100.0, 100) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_halstead_1000_epl() {
        // 100000 effort / 100 lines = 1000 epl → 100
        assert!((normalize_halstead(100000.0, 100) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_halstead_5000_epl() {
        // 500000 effort / 100 lines = 5000 epl → 70
        let s = normalize_halstead(500000.0, 100);
        assert!((s - 70.0).abs() < 0.01, "epl=5000 → 70, got {s}");
    }

    #[test]
    fn test_normalize_halstead_10000_epl() {
        // 1000000 effort / 100 lines = 10000 epl → 40
        let s = normalize_halstead(1000000.0, 100);
        assert!((s - 40.0).abs() < 0.01, "epl=10000 → 40, got {s}");
    }

    #[test]
    fn test_normalize_halstead_20000_epl() {
        // 2000000 effort / 100 lines = 20000 epl → 0
        assert!((normalize_halstead(2000000.0, 100) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_halstead_zero_loc() {
        // Missing data → neutral score
        assert!((normalize_halstead(1000.0, 0) - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_halstead_zero_effort() {
        assert!((normalize_halstead(0.0, 100) - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_halstead_boundary_continuity() {
        // Just above each boundary should score slightly lower
        let at_1000 = normalize_halstead(100000.0, 100);
        let above_1000 = normalize_halstead(100100.0, 100);
        assert!(
            above_1000 < at_1000,
            "epl=1001 should score lower than epl=1000"
        );

        let at_5000 = normalize_halstead(500000.0, 100);
        let above_5000 = normalize_halstead(500100.0, 100);
        assert!(
            above_5000 < at_5000,
            "epl=5001 should score lower than epl=5000"
        );
    }

    // --- normalize_file_size ---

    #[test]
    fn test_normalize_file_size_small() {
        assert!((normalize_file_size(10) - 100.0).abs() < 0.01);
        assert!((normalize_file_size(50) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_file_size_optimal() {
        assert!((normalize_file_size(300) - 100.0).abs() < 0.01);
        assert!((normalize_file_size(500) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_file_size_1000() {
        let s = normalize_file_size(1000);
        assert!((s - 60.0).abs() < 0.01, "1000 LOC → 60, got {s}");
    }

    #[test]
    fn test_normalize_file_size_2000() {
        let s = normalize_file_size(2000);
        assert!((s - 20.0).abs() < 0.01, "2000 LOC → 20, got {s}");
    }

    // --- score_to_grade ---

    #[test]
    fn test_score_to_grade_all_boundaries() {
        assert_eq!(score_to_grade(100.0), Grade::APlusPlus);
        assert_eq!(score_to_grade(97.0), Grade::APlusPlus);
        assert_eq!(score_to_grade(96.9), Grade::APlus);
        assert_eq!(score_to_grade(93.0), Grade::APlus);
        assert_eq!(score_to_grade(92.9), Grade::A);
        assert_eq!(score_to_grade(90.0), Grade::A);
        assert_eq!(score_to_grade(89.9), Grade::AMinus);
        assert_eq!(score_to_grade(87.0), Grade::AMinus);
        assert_eq!(score_to_grade(86.9), Grade::BPlus);
        assert_eq!(score_to_grade(83.0), Grade::BPlus);
        assert_eq!(score_to_grade(82.9), Grade::B);
        assert_eq!(score_to_grade(80.0), Grade::B);
        assert_eq!(score_to_grade(79.9), Grade::BMinus);
        assert_eq!(score_to_grade(77.0), Grade::BMinus);
        assert_eq!(score_to_grade(76.9), Grade::CPlus);
        assert_eq!(score_to_grade(73.0), Grade::CPlus);
        assert_eq!(score_to_grade(72.9), Grade::C);
        assert_eq!(score_to_grade(70.0), Grade::C);
        assert_eq!(score_to_grade(69.9), Grade::CMinus);
        assert_eq!(score_to_grade(67.0), Grade::CMinus);
        assert_eq!(score_to_grade(66.9), Grade::DPlus);
        assert_eq!(score_to_grade(63.0), Grade::DPlus);
        assert_eq!(score_to_grade(62.9), Grade::D);
        assert_eq!(score_to_grade(60.0), Grade::D);
        assert_eq!(score_to_grade(59.9), Grade::DMinus);
        assert_eq!(score_to_grade(57.0), Grade::DMinus);
        assert_eq!(score_to_grade(56.9), Grade::F);
        assert_eq!(score_to_grade(40.0), Grade::F);
        assert_eq!(score_to_grade(39.9), Grade::FMinusMinus);
        assert_eq!(score_to_grade(0.0), Grade::FMinusMinus);
    }

    // --- compute_project_score ---

    #[test]
    fn test_compute_project_score() {
        let dims = vec![
            DimensionScore {
                name: "A",
                weight: 0.60,
                score: 80.0,
                grade: Grade::B,
            },
            DimensionScore {
                name: "B",
                weight: 0.40,
                score: 60.0,
                grade: Grade::D,
            },
        ];
        let s = compute_project_score(&dims);
        // 0.6*80 + 0.4*60 = 48 + 24 = 72
        assert!((s - 72.0).abs() < 0.01, "expected 72, got {s}");
    }

    #[test]
    fn test_compute_project_score_empty() {
        assert!((compute_project_score(&[]) - 0.0).abs() < 0.01);
    }
}
