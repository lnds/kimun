//! Overall code health score computation.
//!
//! Walks source files once, computes per-file metrics (cognitive complexity,
//! indent, Halstead, file size), detects project-level duplication,
//! normalizes each dimension to 0–100 via piecewise linear curves, and
//! produces a LOC-weighted aggregate score with letter grade (A++ to F--).
//!
//! The scoring pipeline: walk → per-file analysis → project-level
//! duplication → normalize → LOC-weighted mean → grade assignment.

/// Grading system: letter grades, dimension/file/project scores.
pub(crate) mod analyzer;
/// Single-file metric extraction (reads once, computes all dimensions).
mod collector;
/// Diff data types and computation for comparing two ProjectScore snapshots.
mod diff;
/// Table and JSON formatters for score diff output.
mod diff_report;
/// Piecewise linear normalization curves mapping raw metrics to 0–100.
mod normalize;
/// Table and JSON output formatters for the score report.
mod report;
/// Dimension scoring, per-file scoring, and LOC-weighted aggregation.
mod scoring;

use std::error::Error;
use std::path::Path;

use crate::dups;
use crate::git::GitRepo;
use crate::walk;

use analyzer::{FileScore, ProjectScore, compute_project_score, score_to_grade};
use collector::{FileMetrics, analyze_single_file};
use report::{print_json, print_report};
use scoring::{build_dimensions, build_empty_dimensions, score_file};

// Re-export scoring internals for tests.
#[cfg(test)]
pub(crate) use scoring::{
    FILE_WEIGHTS, W_COGCOM, W_DUP, W_HAL, W_INDENT, W_SIZE, weighted_mean,
};

/// Entry point: compute and display the project health score.
/// Outputs either a formatted table or JSON depending on the `json` flag.
pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    bottom: usize,
    min_lines: usize,
) -> Result<(), Box<dyn Error>> {
    let score = compute_score(path, include_tests, bottom, min_lines)?;

    // Show target in header when user specified an explicit path (not ".")
    let target = path.to_str().filter(|s| *s != ".").map(|s| s.to_string());

    if json {
        print_json(&score, target.as_deref())?;
    } else {
        print_report(&score, bottom, target.as_deref());
    }

    Ok(())
}

/// Entry point for `km score diff`: compare current working tree against a git ref.
pub fn run_diff(
    path: &Path,
    git_ref: &str,
    json: bool,
    include_tests: bool,
    bottom: usize,
    min_lines: usize,
) -> Result<(), Box<dyn Error>> {
    // Score the current working tree.
    let after = compute_score(path, include_tests, bottom, min_lines)?;

    // Open the git repo and extract the ref tree into a temp directory.
    let repo = GitRepo::open(path)?;
    let tmpdir = tempfile::tempdir()?;
    repo.extract_tree_to_dir(git_ref, tmpdir.path())?;

    // Handle subdirectory case: if the user pointed at a subdir of the repo,
    // analyze the corresponding subdir inside the extracted tree.
    let (_, prefix) = repo.walk_prefix(path)?;
    let tmp_path = if prefix.as_os_str().is_empty() {
        tmpdir.path().to_path_buf()
    } else {
        tmpdir.path().join(&prefix)
    };

    // Score the ref tree.
    let before = compute_score(&tmp_path, include_tests, bottom, min_lines)?;

    let score_diff = diff::compute_diff(git_ref, &before, &after);

    if json {
        diff_report::print_json(&score_diff)?;
    } else {
        diff_report::print_report(&score_diff);
    }

    Ok(())
}

/// Walk all source files, compute per-file and project-level metrics,
/// normalize each dimension, and produce the final `ProjectScore`.
fn compute_score(
    path: &Path,
    include_tests: bool,
    bottom: usize,
    min_lines: usize,
) -> Result<ProjectScore, Box<dyn Error>> {
    let exclude_tests = !include_tests;
    let mut file_metrics: Vec<FileMetrics> = Vec::new();
    let mut dup_files: Vec<dups::detector::NormalizedFile> = Vec::new();
    let mut total_code_lines: usize = 0;

    for (file_path, spec) in walk::source_files(path, exclude_tests) {
        if let Some(result) = analyze_single_file(&file_path, spec, exclude_tests) {
            total_code_lines += result.normalized_count;
            dup_files.push(result.dup_file);
            file_metrics.push(result.metrics);
        }
    }

    // Duplication (project-level)
    let dup_groups = if dup_files.is_empty() {
        Vec::new()
    } else {
        dups::detector::detect_duplicates(&dup_files, min_lines, true)
    };
    let duplicated_lines: usize = dup_groups.iter().map(|g| g.duplicated_lines()).sum();
    let dup_percent = if total_code_lines == 0 {
        0.0
    } else {
        duplicated_lines as f64 / total_code_lines as f64 * 100.0
    };

    let total_loc: usize = file_metrics.iter().map(|f| f.code_lines).sum();
    let files_analyzed = file_metrics.len();

    if files_analyzed == 0 {
        let dimensions = build_empty_dimensions();
        return Ok(ProjectScore {
            score: 0.0,
            grade: score_to_grade(0.0),
            files_analyzed: 0,
            total_loc: 0,
            dimensions,
            needs_attention: vec![],
        });
    }

    let dimensions = build_dimensions(&file_metrics, total_loc, dup_percent);
    let project_score = compute_project_score(&dimensions);
    let mut file_scores: Vec<FileScore> = file_metrics.iter().map(score_file).collect();
    file_scores.sort_by(|a, b| a.score.total_cmp(&b.score));
    file_scores.truncate(bottom);

    Ok(ProjectScore {
        score: project_score,
        grade: score_to_grade(project_score),
        files_analyzed,
        total_loc,
        dimensions,
        needs_attention: file_scores,
    })
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
