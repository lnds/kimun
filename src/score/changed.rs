use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::gate_score_worsened;
use crate::git::FileChange;

/// A changed file's score at the ref and now. `before` is `None` for new files.
#[derive(Debug, Clone, Serialize)]
pub struct ChangedFile {
    /// Repo-relative, as it appears in the PR diff.
    pub path: String,
    pub before: Option<f64>,
    pub after: f64,
    pub delta: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangedScope {
    pub files: Vec<ChangedFile>,
    pub duplicated_lines_before: usize,
    pub duplicated_lines_after: usize,
}

/// Pairs each change with its scores, worst drop first and new files last.
///
/// `before` and `after` are keyed by path relative to the analyzed directory,
/// which sits at `prefix` inside the repo. Changes outside it, and files that
/// were not scored (tests, unsupported languages), are skipped.
pub fn changed_files(
    changes: &[FileChange],
    prefix: &Path,
    before: &HashMap<PathBuf, f64>,
    after: &HashMap<PathBuf, f64>,
) -> Vec<ChangedFile> {
    let mut files: Vec<ChangedFile> = changes
        .iter()
        .filter_map(|c| {
            let after = *after.get(c.path.strip_prefix(prefix).ok()?)?;
            let before = c
                .old_path
                .as_deref()
                .and_then(|p| p.strip_prefix(prefix).ok())
                .and_then(|p| before.get(p).copied());
            Some(ChangedFile {
                path: c.path.display().to_string(),
                before,
                after,
                delta: before.map(|b| after - b),
            })
        })
        .collect();
    files.sort_by(|a, b| {
        let by_delta = match (a.delta, b.delta) {
            (Some(x), Some(y)) => x.total_cmp(&y),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        };
        by_delta.then_with(|| a.path.cmp(&b.path))
    });
    files
}

/// A file fails only if it ends below `baseline`, the project score at the
/// ref, after dropping more than `tolerance`: files above it may grow and
/// lose points freely. New files never fail, having no earlier score.
pub fn gate_failure(scope: &ChangedScope, tolerance: f64, baseline: f64) -> Option<String> {
    let mut reasons: Vec<String> = scope
        .files
        .iter()
        .filter_map(|f| {
            let before = f.before?;
            (f.after < baseline && gate_score_worsened(before, f.after, tolerance)).then(|| {
                format!(
                    "{} dropped {before:.2} → {:.2} ({:+.4}), below the project score \
                     {baseline:.2} and more than the {tolerance} tolerance",
                    f.path,
                    f.after,
                    f.after - before
                )
            })
        })
        .collect();
    let (dup_before, dup_after) = (scope.duplicated_lines_before, scope.duplicated_lines_after);
    if dup_after > dup_before {
        reasons.push(format!(
            "duplicated lines grew {dup_before} → {dup_after} (+{})",
            dup_after - dup_before
        ));
    }
    (!reasons.is_empty()).then(|| format!("quality gate failed:\n  {}", reasons.join("\n  ")))
}

#[cfg(test)]
#[path = "changed_test.rs"]
mod tests;
