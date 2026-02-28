//! Diff data types and computation for comparing two `ProjectScore` snapshots.
//!
//! Produces a `ScoreDiff` that captures the before/after state of each
//! dimension plus the overall project score, with signed deltas.

use serde::Serialize;

use super::analyzer::{Grade, ProjectScore};

/// Numeric delta: before, after, and signed difference.
#[derive(Debug, Clone, Serialize)]
pub struct ScoreDelta {
    pub before: f64,
    pub after: f64,
    pub delta: f64,
}

/// Per-dimension delta with name, weight, and before/after grades.
#[derive(Debug, Clone, Serialize)]
pub struct DimensionDelta {
    pub name: String,
    pub weight: f64,
    pub before_score: f64,
    pub before_grade: Grade,
    pub after_score: f64,
    pub after_grade: Grade,
    pub delta: f64,
}

/// Full diff result comparing two project score snapshots.
#[derive(Debug, Clone, Serialize)]
pub struct ScoreDiff {
    pub git_ref: String,
    pub overall: ScoreDelta,
    pub before_grade: Grade,
    pub after_grade: Grade,
    pub files_before: usize,
    pub files_after: usize,
    pub loc_before: usize,
    pub loc_after: usize,
    pub dimensions: Vec<DimensionDelta>,
}

/// Compare two `ProjectScore` snapshots and produce a `ScoreDiff`.
pub fn compute_diff(git_ref: &str, before: &ProjectScore, after: &ProjectScore) -> ScoreDiff {
    let dimensions: Vec<DimensionDelta> = before
        .dimensions
        .iter()
        .zip(after.dimensions.iter())
        .map(|(b, a)| DimensionDelta {
            name: b.name.to_string(),
            weight: b.weight,
            before_score: b.score,
            before_grade: b.grade,
            after_score: a.score,
            after_grade: a.grade,
            delta: a.score - b.score,
        })
        .collect();

    ScoreDiff {
        git_ref: git_ref.to_string(),
        overall: ScoreDelta {
            before: before.score,
            after: after.score,
            delta: after.score - before.score,
        },
        before_grade: before.grade,
        after_grade: after.grade,
        files_before: before.files_analyzed,
        files_after: after.files_analyzed,
        loc_before: before.total_loc,
        loc_after: after.total_loc,
        dimensions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::analyzer::{DimensionScore, Grade, ProjectScore};

    fn make_score(score: f64, files: usize, loc: usize, dim_score: f64) -> ProjectScore {
        let grade = crate::score::analyzer::score_to_grade(score);
        let dim_grade = crate::score::analyzer::score_to_grade(dim_score);
        ProjectScore {
            score,
            grade,
            files_analyzed: files,
            total_loc: loc,
            dimensions: vec![
                DimensionScore {
                    name: "Cognitive Complexity",
                    weight: 0.30,
                    score: dim_score,
                    grade: dim_grade,
                },
                DimensionScore {
                    name: "Duplication",
                    weight: 0.20,
                    score: dim_score,
                    grade: dim_grade,
                },
            ],
            needs_attention: vec![],
        }
    }

    #[test]
    fn compute_diff_positive_delta() {
        let before = make_score(70.0, 10, 1000, 70.0);
        let after = make_score(80.0, 12, 1200, 80.0);
        let diff = compute_diff("HEAD", &before, &after);

        assert_eq!(diff.git_ref, "HEAD");
        assert!((diff.overall.delta - 10.0).abs() < 0.01);
        assert_eq!(diff.before_grade, Grade::C);
        assert_eq!(diff.after_grade, Grade::B);
        assert_eq!(diff.files_before, 10);
        assert_eq!(diff.files_after, 12);
        assert_eq!(diff.dimensions.len(), 2);
        assert!((diff.dimensions[0].delta - 10.0).abs() < 0.01);
    }

    #[test]
    fn compute_diff_negative_delta() {
        let before = make_score(80.0, 10, 1000, 80.0);
        let after = make_score(70.0, 10, 1000, 70.0);
        let diff = compute_diff("main", &before, &after);

        assert!((diff.overall.delta - (-10.0)).abs() < 0.01);
    }

    #[test]
    fn compute_diff_no_change() {
        let before = make_score(85.0, 10, 1000, 85.0);
        let after = make_score(85.0, 10, 1000, 85.0);
        let diff = compute_diff("HEAD", &before, &after);

        assert!((diff.overall.delta).abs() < 0.01);
        assert_eq!(diff.before_grade, diff.after_grade);
    }
}
