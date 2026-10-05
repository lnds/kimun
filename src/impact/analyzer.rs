//! Impact measures of a change set.
//!
//! Diffusion describes how spread the change is (Kamei et al., "A Large-Scale
//! Empirical Study of Just-in-Time Quality Assurance", 2013). The logical
//! radius lists the files that history says should have changed with it
//! (Zimmermann et al., "Mining Version Histories to Guide Software Changes",
//! 2005).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::git::{CoChangeHistory, FileDiffStat};

/// How spread a change is across the project.
#[derive(Debug, Clone, PartialEq)]
pub struct Diffusion {
    pub files: usize,
    /// Distinct directories holding a changed file.
    pub directories: usize,
    /// Distinct top-level directories; files at the root form one more.
    pub subsystems: usize,
    pub lines_added: usize,
    pub lines_deleted: usize,
    /// Shannon entropy of the modified lines over the files, divided by its
    /// maximum: 0 when one file holds every modified line, 1 when all the
    /// files hold the same amount.
    pub entropy: f64,
}

/// The top-level directory of a path, or `.` for a file at the root.
fn subsystem(path: &Path) -> &Path {
    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(first), Some(_)) => Path::new(first.as_os_str()),
        _ => Path::new("."),
    }
}

/// Normalized Shannon entropy of a distribution given as raw weights.
/// Zero weights are ignored; fewer than two weights have no spread.
fn normalized_entropy(weights: &[usize]) -> f64 {
    let weights: Vec<f64> = weights
        .iter()
        .filter(|&&w| w > 0)
        .map(|&w| w as f64)
        .collect();
    if weights.len() < 2 {
        return 0.0;
    }
    let total: f64 = weights.iter().sum();
    let entropy: f64 = weights
        .iter()
        .map(|w| {
            let p = w / total;
            -p * p.log2()
        })
        .sum();
    entropy / (weights.len() as f64).log2()
}

/// Measure how spread a change set is.
pub fn compute_diffusion(changes: &[FileDiffStat]) -> Diffusion {
    let directories: HashSet<&Path> = changes
        .iter()
        .map(|c| c.path.parent().unwrap_or(Path::new("")))
        .collect();
    let subsystems: HashSet<&Path> = changes.iter().map(|c| subsystem(&c.path)).collect();
    let modified: Vec<usize> = changes.iter().map(|c| c.added + c.deleted).collect();

    Diffusion {
        files: changes.len(),
        directories: directories.len(),
        subsystems: subsystems.len(),
        lines_added: changes.iter().map(|c| c.added).sum(),
        lines_deleted: changes.iter().map(|c| c.deleted).sum(),
        entropy: normalized_entropy(&modified),
    }
}

/// A changed file and the path its history is recorded under, which differs
/// from the current one after a rename.
pub struct Target {
    pub path: PathBuf,
    pub history_path: PathBuf,
}

/// A changed file that predicts a change to another file.
#[derive(Debug, Clone, PartialEq)]
pub struct Trigger {
    pub path: PathBuf,
    /// Commits that changed both files.
    pub shared_commits: usize,
    /// Commits that changed the trigger.
    pub commits: usize,
    /// Share of the trigger's commits that also changed the other file.
    pub confidence: f64,
}

/// A file left out of the change that usually changes with it.
#[derive(Debug, Clone, PartialEq)]
pub struct MissingCoChange {
    pub path: PathBuf,
    /// The changed files that predict it, strongest first. Never empty.
    pub triggers: Vec<Trigger>,
}

impl MissingCoChange {
    /// The strongest trigger.
    pub fn best(&self) -> &Trigger {
        &self.triggers[0]
    }
}

/// Minimum evidence for reporting a missing co-change.
#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    pub min_confidence: f64,
    pub min_shared: usize,
}

/// Strongest first: confidence, then shared commits, then path.
fn by_strength(a: &Trigger, b: &Trigger) -> std::cmp::Ordering {
    b.confidence
        .total_cmp(&a.confidence)
        .then_with(|| b.shared_commits.cmp(&a.shared_commits))
        .then_with(|| a.path.cmp(&b.path))
}

/// Find the files that usually change with `targets` and pass `is_candidate`.
///
/// Confidence is directional: the share of a target's commits that also
/// changed the candidate. A symmetric strength would report a file that
/// rarely needs to follow a frequently changed one.
pub fn missing_co_changes(
    history: &CoChangeHistory,
    targets: &[Target],
    is_candidate: impl Fn(&Path) -> bool,
    thresholds: Thresholds,
) -> Vec<MissingCoChange> {
    let mut by_path: HashMap<&Path, Vec<Trigger>> = HashMap::new();

    for target in targets {
        let commits = history
            .commits
            .get(&target.history_path)
            .copied()
            .unwrap_or(0);
        let Some(shared) = history.shared.get(&target.history_path) else {
            continue;
        };
        for (other, &shared_commits) in shared {
            let confidence = shared_commits as f64 / commits as f64;
            if shared_commits < thresholds.min_shared
                || confidence < thresholds.min_confidence
                || !is_candidate(other)
            {
                continue;
            }
            by_path.entry(other).or_default().push(Trigger {
                path: target.path.clone(),
                shared_commits,
                commits,
                confidence,
            });
        }
    }

    let mut missing: Vec<MissingCoChange> = by_path
        .into_iter()
        .map(|(path, mut triggers)| {
            triggers.sort_by(by_strength);
            MissingCoChange {
                path: path.to_path_buf(),
                triggers,
            }
        })
        .collect();
    missing.sort_by(|a, b| by_strength(a.best(), b.best()).then_with(|| a.path.cmp(&b.path)));
    missing
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
