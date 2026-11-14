//! Diff data types and computation for comparing two `ProjectScore` snapshots.
//!
//! Produces a `ScoreDiff` that captures the before/after state of each
//! dimension plus the overall project score, with signed deltas.

use serde::Serialize;

use super::analyzer::{Graded, ProjectScore};
use super::changed::ChangedScope;

/// Per-dimension delta with name, weight, and the before/after scores.
/// A side with nothing to grade is `None`, and so is the delta.
#[derive(Debug, Clone, Serialize)]
pub struct DimensionDelta {
    pub name: String,
    pub weight: f64,
    pub before: Option<Graded>,
    pub after: Option<Graded>,
    pub delta: Option<f64>,
}

/// Full diff result comparing two project score snapshots. A side with
/// nothing to grade (no code files) is `None`, and so is the delta.
#[derive(Debug, Clone, Serialize)]
pub struct ScoreDiff {
    pub git_ref: String,
    pub before: Option<Graded>,
    pub after: Option<Graded>,
    pub delta: Option<f64>,
    pub files_before: usize,
    pub files_after: usize,
    pub loc_before: usize,
    pub loc_after: usize,
    pub dimensions: Vec<DimensionDelta>,
    /// Per-file breakdown, present only under `--gate-scope changed`.
    pub changed: Option<ChangedScope>,
}

/// After minus before, when both sides have a score.
fn change(before: Option<Graded>, after: Option<Graded>) -> Option<f64> {
    Some(after?.score - before?.score)
}

/// Compare two `ProjectScore` snapshots and produce a `ScoreDiff`.
///
/// # Panics
///
/// Panics if the two snapshots were computed with different scoring models
/// (different dimension names or counts). Both must use the same `--model`.
pub fn compute_diff(git_ref: &str, before: &ProjectScore, after: &ProjectScore) -> ScoreDiff {
    assert_eq!(
        before.dimensions.len(),
        after.dimensions.len(),
        "cannot diff scores with different models: before has {} dimensions, after has {}",
        before.dimensions.len(),
        after.dimensions.len(),
    );
    for (b, a) in before.dimensions.iter().zip(after.dimensions.iter()) {
        assert_eq!(
            b.name, a.name,
            "dimension mismatch: before has '{}', after has '{}' — use the same --model for both",
            b.name, a.name,
        );
    }

    let dimensions: Vec<DimensionDelta> = before
        .dimensions
        .iter()
        .zip(after.dimensions.iter())
        .map(|(b, a)| DimensionDelta {
            name: b.name.to_string(),
            weight: b.weight,
            before: b.graded,
            after: a.graded,
            delta: change(b.graded, a.graded),
        })
        .collect();

    ScoreDiff {
        git_ref: git_ref.to_string(),
        before: before.graded,
        after: after.graded,
        delta: change(before.graded, after.graded),
        files_before: before.files_analyzed,
        files_after: after.files_analyzed,
        loc_before: before.total_loc,
        loc_after: after.total_loc,
        dimensions,
        changed: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::analyzer::{DimensionScore, Grade, ProjectScore};

    /// A two-dimension score, `None` for nothing to grade.
    fn make_score(score: Option<f64>, files: usize, loc: usize) -> ProjectScore {
        let graded = score.map(Graded::of);
        let dimension = |name| DimensionScore {
            name,
            weight: 0.5,
            graded,
        };
        ProjectScore {
            graded,
            files_analyzed: files,
            total_loc: loc,
            dimensions: vec![dimension("Cognitive Complexity"), dimension("Duplication")],
            needs_attention: vec![],
        }
    }

    #[test]
    fn compute_diff_positive_delta() {
        let before = make_score(Some(70.0), 10, 1000);
        let after = make_score(Some(80.0), 12, 1200);
        let diff = compute_diff("HEAD", &before, &after);

        assert_eq!(diff.git_ref, "HEAD");
        assert_eq!(diff.delta, Some(10.0));
        assert_eq!(diff.before.map(|g| g.grade), Some(Grade::C));
        assert_eq!(diff.after.map(|g| g.grade), Some(Grade::B));
        assert_eq!(diff.files_before, 10);
        assert_eq!(diff.files_after, 12);
        assert_eq!(diff.dimensions.len(), 2);
        assert_eq!(diff.dimensions[0].delta, Some(10.0));
    }

    #[test]
    fn compute_diff_negative_delta() {
        let before = make_score(Some(80.0), 10, 1000);
        let after = make_score(Some(70.0), 10, 1000);
        let diff = compute_diff("main", &before, &after);

        assert_eq!(diff.delta, Some(-10.0));
    }

    #[test]
    fn compute_diff_no_change() {
        let before = make_score(Some(85.0), 10, 1000);
        let after = make_score(Some(85.0), 10, 1000);
        let diff = compute_diff("HEAD", &before, &after);

        assert_eq!(diff.delta, Some(0.0));
        assert_eq!(diff.before, diff.after);
    }

    #[test]
    fn a_side_with_nothing_to_grade_has_no_delta() {
        let graded = make_score(Some(85.0), 3, 120);
        let ungraded = make_score(None, 0, 0);

        for (before, after) in [(&graded, &ungraded), (&ungraded, &graded)] {
            let diff = compute_diff("HEAD", before, after);
            assert_eq!(diff.delta, None);
            assert!(diff.dimensions.iter().all(|d| d.delta.is_none()));
        }
        let diff = compute_diff("HEAD", &graded, &ungraded);
        assert_eq!(diff.before, Some(Graded::of(85.0)));
        assert_eq!(diff.after, None);
        assert_eq!(diff.dimensions[0].before, Some(Graded::of(85.0)));
        assert_eq!(diff.dimensions[0].after, None);
    }
}
