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

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
