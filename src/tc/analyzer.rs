use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CouplingLevel {
    Strong,
    Moderate,
    Weak,
}

impl CouplingLevel {
    pub fn label(&self) -> &'static str {
        match self {
            CouplingLevel::Strong => "STRONG",
            CouplingLevel::Moderate => "MODERATE",
            CouplingLevel::Weak => "WEAK",
        }
    }
}

pub fn classify_level(strength: f64) -> CouplingLevel {
    if strength >= 0.5 {
        CouplingLevel::Strong
    } else if strength >= 0.3 {
        CouplingLevel::Moderate
    } else {
        CouplingLevel::Weak
    }
}

pub struct FileCoupling {
    pub file_a: PathBuf,
    pub file_b: PathBuf,
    pub shared_commits: usize,
    pub commits_a: usize,
    pub commits_b: usize,
    pub strength: f64,
    pub level: CouplingLevel,
}

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
