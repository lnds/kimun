//! Temporal coupling analysis: detects files that change together in git.
//!
//! Processes co-changing commit data to find implicit dependencies between
//! files. Coupling strength = shared_commits / min(commits_a, commits_b),
//! classified as Strong (≥0.5), Moderate (0.3–0.5), or Weak (<0.3).
//! High coupling between unrelated modules suggests hidden dependencies.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

/// Coupling strength classification based on shared commit frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CouplingLevel {
    Strong,
    Moderate,
    Weak,
}

impl CouplingLevel {
    /// Human-readable uppercase label for display in reports.
    pub fn label(&self) -> &'static str {
        match self {
            CouplingLevel::Strong => "STRONG",
            CouplingLevel::Moderate => "MODERATE",
            CouplingLevel::Weak => "WEAK",
        }
    }
}

/// Classify a coupling strength value into Strong, Moderate, or Weak.
pub fn classify_level(strength: f64) -> CouplingLevel {
    if strength >= 0.5 {
        CouplingLevel::Strong
    } else if strength >= 0.3 {
        CouplingLevel::Moderate
    } else {
        CouplingLevel::Weak
    }
}

/// A pair of files with their temporal coupling metrics.
/// Pairs are ordered lexicographically (file_a < file_b).
pub struct FileCoupling {
    pub file_a: PathBuf,
    pub file_b: PathBuf,
    /// Number of commits that modified both files.
    pub shared_commits: usize,
    /// Total commits that modified file_a.
    pub commits_a: usize,
    /// Total commits that modified file_b.
    pub commits_b: usize,
    /// Coupling strength: shared / min(commits_a, commits_b).
    pub strength: f64,
    pub level: CouplingLevel,
}

/// Compute temporal coupling for all file pairs from co-change data.
///
/// For each commit, generates all pairs of eligible files (those with
/// at least `min_degree` total commits), counts shared commits per pair,
/// and returns results sorted by strength descending.
pub fn compute_coupling(
    co_changes: &[Vec<PathBuf>],
    freq_map: &HashMap<PathBuf, usize>,
    min_degree: usize,
) -> Vec<FileCoupling> {
    let mut pair_counts: HashMap<(PathBuf, PathBuf), usize> = HashMap::new();

    for commit_files in co_changes {
        // Filter to files that pass the min_degree threshold
        let mut eligible: Vec<&PathBuf> = commit_files
            .iter()
            .filter(|p| freq_map.get(*p).is_some_and(|&c| c >= min_degree))
            .collect();
        eligible.sort_unstable();

        // Generate all pairs ordered lexicographically
        for i in 0..eligible.len() {
            for j in (i + 1)..eligible.len() {
                let key = (eligible[i].clone(), eligible[j].clone());
                *pair_counts.entry(key).or_insert(0) += 1;
            }
        }
    }

    let mut results: Vec<FileCoupling> = pair_counts
        .into_iter()
        .map(|((a, b), shared)| {
            let commits_a = freq_map[&a];
            let commits_b = freq_map[&b];
            // min_commits >= min_degree >= 1 (enforced by caller), so division is safe
            let min_commits = commits_a.min(commits_b);
            let strength = shared as f64 / min_commits as f64;
            FileCoupling {
                file_a: a,
                file_b: b,
                shared_commits: shared,
                commits_a,
                commits_b,
                strength,
                level: classify_level(strength),
            }
        })
        .collect();

    results.sort_by(|a, b| {
        b.strength
            .partial_cmp(&a.strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
