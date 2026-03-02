//! Overall code health score computation.
//!
//! Walks source files once, computes per-file metrics (cognitive complexity
//! or MI+cyclomatic depending on model), indent, Halstead, file size),
//! detects project-level duplication, normalizes each dimension to 0–100
//! via piecewise linear curves, and produces a LOC-weighted aggregate
//! score with letter grade (A++ to F--).
//!
//! Two scoring models are supported:
//! - **Cognitive** (default, v0.14+): 5 dimensions with cognitive complexity.
//! - **Legacy** (`--model legacy`, v0.13): 6 dimensions with MI + cyclomatic.
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

use crate::dups;
use crate::git::GitRepo;
use crate::walk::WalkConfig;

use analyzer::{FileScore, ProjectScore, compute_project_score, score_to_grade};
use collector::{FileMetrics, analyze_single_file};
use report::{print_json, print_report};
use scoring::{build_dimensions, build_empty_dimensions, score_file};

/// Scoring model selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringModel {
    /// Cognitive complexity model (v0.14+, default): 5 dimensions.
    Cognitive,
    /// Legacy model (v0.13): MI + cyclomatic, 6 dimensions.
    Legacy,
}

impl ScoringModel {
    /// Parse a model name from CLI argument (validated by clap's value_parser).
    pub fn from_arg(s: &str) -> Self {
        match s {
            "legacy" => Self::Legacy,
            "cogcom" => Self::Cognitive,
            other => {
                debug_assert!(false, "unexpected scoring model: {other}");
                Self::Cognitive
            }
        }
    }
}

/// Entry point: compute and display the project health score.
/// Outputs either a formatted table or JSON depending on the `json` flag.
pub fn run(
    cfg: &WalkConfig<'_>,
    json: bool,
    bottom: usize,
    min_lines: usize,
    model: &str,
) -> Result<(), Box<dyn Error>> {
    let scoring_model = ScoringModel::from_arg(model);
    let score = compute_score(cfg, bottom, min_lines, &scoring_model)?;

    // Show target in header when user specified an explicit path (not ".")
    let target = cfg
        .path
        .to_str()
        .filter(|s| *s != ".")
        .map(|s| s.to_string());

    if json {
        print_json(&score, target.as_deref())?;
    } else {
        print_report(&score, bottom, target.as_deref());
    }

    Ok(())
}

/// Entry point for `km score diff`: compare current working tree against a git ref.
pub fn run_diff(
    cfg: &WalkConfig<'_>,
    git_ref: &str,
    json: bool,
    bottom: usize,
    min_lines: usize,
    model: &str,
) -> Result<(), Box<dyn Error>> {
    let scoring_model = ScoringModel::from_arg(model);

    // Score the current working tree.
    let after = compute_score(cfg, bottom, min_lines, &scoring_model)?;

    // Open the git repo and extract the ref tree into a temp directory.
    let repo = GitRepo::open(cfg.path)?;
    let tmpdir = tempfile::tempdir()?;
    repo.extract_tree_to_dir(git_ref, tmpdir.path())?;

    // Handle subdirectory case: if the user pointed at a subdir of the repo,
    // analyze the corresponding subdir inside the extracted tree.
    let (_, prefix) = repo.walk_prefix(cfg.path)?;
    let tmp_path = if prefix.as_os_str().is_empty() {
        tmpdir.path().to_path_buf()
    } else {
        tmpdir.path().join(&prefix)
    };

    // Score the ref tree.
    let ref_cfg = WalkConfig::new(&tmp_path, cfg.include_tests, cfg.filter);
    let before = compute_score(&ref_cfg, bottom, min_lines, &scoring_model)?;

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
    cfg: &WalkConfig<'_>,
    bottom: usize,
    min_lines: usize,
    model: &ScoringModel,
) -> Result<ProjectScore, Box<dyn Error>> {
    let exclude_tests = cfg.exclude_tests();
    let mut file_metrics: Vec<FileMetrics> = Vec::new();
    let mut dup_files: Vec<dups::detector::NormalizedFile> = Vec::new();
    let mut total_code_lines: usize = 0;

    for (file_path, spec) in cfg.source_files() {
        if let Some(result) = analyze_single_file(&file_path, spec, exclude_tests, model) {
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
        let dimensions = build_empty_dimensions(model);
        return Ok(ProjectScore {
            score: 0.0,
            grade: score_to_grade(0.0),
            files_analyzed: 0,
            total_loc: 0,
            dimensions,
            needs_attention: vec![],
        });
    }

    let dimensions = build_dimensions(&file_metrics, total_loc, dup_percent, model);
    let project_score = compute_project_score(&dimensions);
    let mut file_scores: Vec<FileScore> =
        file_metrics.iter().map(|f| score_file(f, model)).collect();
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
