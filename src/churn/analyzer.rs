/// Churn analyzer — pure change-frequency data per source file.
///
/// A file's churn rate measures how fast it evolves: commits divided by the
/// number of months it has been active (first commit → last commit, minimum
/// one month). High-churn files are "moving targets" — hard to reason about,
/// easy to break, and worth monitoring even when their complexity is low.
use std::path::PathBuf;

/// Churn classification based on commits-per-month rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChurnLevel {
    /// < 1 commit/month — stable.
    Low,
    /// 1–4 commits/month — active development.
    Medium,
    /// > 4 commits/month — moving target.
    High,
}

impl ChurnLevel {
    pub fn label(self) -> &'static str {
        match self {
            ChurnLevel::Low => "LOW",
            ChurnLevel::Medium => "MEDIUM",
            ChurnLevel::High => "HIGH",
        }
    }
}

/// Churn data for a single source file.
pub struct FileChurn {
    /// Repository-relative file path.
    pub path: PathBuf,
    pub language: String,
    /// Total number of non-merge commits that touched this file.
    pub commits: usize,
    /// Commits per month over the file's active period.
    pub rate: f64,
    /// Unix timestamp of the earliest commit.
    pub first_commit: i64,
    /// Unix timestamp of the most recent commit.
    pub last_commit: i64,
    pub level: ChurnLevel,
}

const SECS_PER_MONTH: f64 = 30.0 * 24.0 * 3600.0;

/// Classify a file by its commit frequency.
pub fn classify(
    path: PathBuf,
    language: &str,
    commits: usize,
    first_commit: i64,
    last_commit: i64,
) -> FileChurn {
    let span_secs = (last_commit - first_commit).max(0) as f64;
    let months = (span_secs / SECS_PER_MONTH).max(1.0);
    let rate = commits as f64 / months;

    let level = if rate > 4.0 {
        ChurnLevel::High
    } else if rate >= 1.0 {
        ChurnLevel::Medium
    } else {
        ChurnLevel::Low
    };

    FileChurn {
        path,
        language: language.to_string(),
        commits,
        rate,
        first_commit,
        last_commit,
        level,
    }
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
