/// Code age classifier.
///
/// Classifies files by how long ago they were last modified in git:
/// - Active:  < `active_days` days ago (default 90)
/// - Stale:   between `active_days` and `frozen_days` (default 365)
/// - Frozen:  > `frozen_days` days ago
use std::path::PathBuf;

/// Age classification for a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgeStatus {
    /// Modified within `active_days` days.
    Active,
    /// Not modified for `active_days`–`frozen_days` days.
    Stale,
    /// Not modified for more than `frozen_days` days.
    Frozen,
}

impl AgeStatus {
    pub fn label(self) -> &'static str {
        match self {
            AgeStatus::Active => "ACTIVE",
            AgeStatus::Stale => "STALE",
            AgeStatus::Frozen => "FROZEN",
        }
    }
}

/// Age analysis result for a single file.
pub struct FileAge {
    /// Repository-relative file path.
    pub path: PathBuf,
    pub language: String,
    /// Unix timestamp of the last commit touching this file.
    pub last_modified: i64,
    /// Number of days since the last commit.
    pub age_days: u64,
    pub status: AgeStatus,
}

/// Thresholds for age classification (in days).
pub struct AgeThresholds {
    /// Files modified within this many days are Active.
    pub active_days: u64,
    /// Files not modified for more than this many days are Frozen.
    pub frozen_days: u64,
}

impl Default for AgeThresholds {
    fn default() -> Self {
        Self {
            active_days: 90,
            frozen_days: 365,
        }
    }
}

/// Classify a file by its last modification timestamp relative to `now`.
pub fn classify(
    path: PathBuf,
    language: &str,
    last_modified: i64,
    now: i64,
    thresholds: &AgeThresholds,
) -> FileAge {
    let age_days = ((now - last_modified).max(0) as u64) / 86_400;
    let status = if age_days < thresholds.active_days {
        AgeStatus::Active
    } else if age_days < thresholds.frozen_days {
        AgeStatus::Stale
    } else {
        AgeStatus::Frozen
    };
    FileAge {
        path,
        language: language.to_string(),
        last_modified,
        age_days,
        status,
    }
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
