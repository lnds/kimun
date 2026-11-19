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
/// The badge of the score for a README.
mod badge;
/// Per-file comparison behind `--gate-scope changed`.
pub(crate) mod changed;
/// Single-file metric extraction (reads once, computes all dimensions).
mod collector;
/// Diff data types and computation for comparing two ProjectScore snapshots.
pub(crate) mod diff;
/// Table and JSON formatters for score diff output.
mod diff_report;
pub(crate) mod experimental;
/// Piecewise linear normalization curves mapping raw metrics to 0–100.
mod normalize;
/// Table and JSON output formatters for the score report.
mod report;
/// Dimension scoring, per-file scoring, and LOC-weighted aggregation.
mod scoring;

use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::dups;
use crate::git::GitRepo;
use crate::walk::WalkConfig;

use crate::options::{GateScope, OutputMode};
use analyzer::{FileScore, Graded, ProjectScore};
use collector::FileMetrics;
use report::{print_json, print_report, print_short, print_terse};
use scoring::{build_dimensions, build_empty_dimensions, compute_project_score, score_file};

/// Quality gate options for `--trend` mode.
/// Both conditions are independent and checked after the report is printed.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScoreGate {
    /// Fail if the score drops by more than this many points.
    pub max_drop: Option<f64>,
    /// Whether `max_drop` applies to the project aggregate or to each changed file.
    pub scope: GateScope,
    /// Fail if the current grade is below this threshold.
    pub fail_below: Option<analyzer::Grade>,
}

/// Scoring model selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringModel {
    /// Cognitive complexity model (default): 5 dimensions.
    Cognitive,
    /// The cognitive model as weighted from v0.14 to v0.35 (File Size at 15%).
    Cognitive035,
    /// Legacy model (v0.13): MI + cyclomatic, 6 dimensions.
    Legacy,
    /// Experimental: the cognitive model with the Maintainability Index
    /// (Visual Studio variant) in place of Halstead Effort (`--experimental`).
    ExperimentalMi,
}

impl ScoringModel {
    /// Parse a model name from CLI argument (validated by clap's value_parser).
    pub fn from_arg(s: &str) -> Self {
        match s {
            "legacy" => Self::Legacy,
            "cogcom" => Self::Cognitive,
            "cogcom-0.35" => Self::Cognitive035,
            other => {
                debug_assert!(false, "unexpected scoring model: {other}");
                Self::Cognitive
            }
        }
    }

    /// The weights of a cognitive model. The legacy model has its own, so it
    /// gets those of the default, never read.
    pub fn cognitive_weights(&self) -> &'static scoring::CognitiveWeights {
        match self {
            Self::Cognitive035 => &scoring::COGNITIVE_0_35,
            Self::Cognitive | Self::Legacy | Self::ExperimentalMi => &scoring::COGNITIVE,
        }
    }
}

/// Entry point: compute and display the project health score.
pub fn run(
    cfg: &WalkConfig<'_>,
    output: OutputMode,
    bottom: usize,
    min_lines: usize,
    model: &str,
    badge: Option<&Path>,
) -> Result<(), Box<dyn Error>> {
    let scoring_model = ScoringModel::from_arg(model);
    let score = compute_score(cfg, bottom, min_lines, &scoring_model)?;
    if let Some(path) = badge {
        badge::write(path, &score)?;
    }

    // Show target in header when user specified an explicit path (not ".")
    let target = cfg
        .path
        .to_str()
        .filter(|s| *s != ".")
        .map(|s| s.to_string());

    match output {
        OutputMode::Json => print_json(&score, target.as_deref())?,
        OutputMode::Short => print_short(&score),
        OutputMode::Terse => print_terse(&score),
        OutputMode::Github | OutputMode::Codeclimate => {
            return Err(crate::options::ERR_CI_FORMAT_ONLY.into());
        }
        OutputMode::Table => print_report(&score, bottom, target.as_deref()),
    }

    Ok(())
}

/// Compares raw scores: rounding first would fail a sub-tolerance drop
/// whenever it happens to straddle a rounding boundary.
pub(crate) fn gate_score_worsened(before: f64, after: f64, tolerance: f64) -> bool {
    before - after > tolerance
}

/// The delta gets four decimals so a drop just over the tolerance never
/// prints as equal to it.
pub(crate) fn format_gate_error(before: f64, after: f64, delta: f64, tolerance: f64) -> String {
    format!(
        "quality gate failed: score dropped {before:.2} → {after:.2} ({delta:+.4}), \
         more than the {tolerance} tolerance"
    )
}

/// Entry point for `km score --trend`: compare current working tree against a git ref.
/// Quality gates in `gate` are checked after the report is printed so CI logs are complete.
pub fn run_diff(
    cfg: &WalkConfig<'_>,
    git_ref: &str,
    output: OutputMode,
    bottom: usize,
    min_lines: usize,
    model: &str,
    gate: ScoreGate,
) -> Result<(), Box<dyn Error>> {
    let scoring_model = ScoringModel::from_arg(model);

    // Score the current working tree.
    let after = compute_snapshot(cfg, bottom, min_lines, &scoring_model)?;

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
    let before = compute_snapshot(&ref_cfg, bottom, min_lines, &scoring_model)?;

    let mut score_diff = diff::compute_diff(git_ref, &before.score, &after.score);
    if gate.scope == GateScope::Changed {
        let changes = repo.changes_since_in_workdir(git_ref)?;
        score_diff.changed = Some(changed::ChangedScope {
            files: changed::changed_files(
                &changes,
                &prefix,
                &before.scores_by_path(&tmp_path),
                &after.scores_by_path(cfg.path),
            ),
            duplicated_lines_before: before.duplicated_lines,
            duplicated_lines_after: after.duplicated_lines,
        });
    }

    // Always print first so CI logs show the full report before any gate error.
    match output {
        OutputMode::Json => diff_report::print_json(&score_diff)?,
        OutputMode::Short => diff_report::print_short(&score_diff),
        OutputMode::Terse => diff_report::print_terse(&score_diff),
        OutputMode::Github | OutputMode::Codeclimate => {
            return Err(crate::options::ERR_CI_FORMAT_ONLY.into());
        }
        OutputMode::Table => diff_report::print_report(&score_diff),
    }

    // Quality gates: evaluated after output so the log is always complete.
    if let Some(note) = skipped_gates(&score_diff, gate, &cfg.path.display().to_string()) {
        eprintln!("{note}");
    }
    match gate_failure(&score_diff, gate) {
        Some(msg) => Err(msg.into()),
        None => Ok(()),
    }
}

/// The gate that fails, if any. A side with nothing to grade leaves no
/// score to compare: the project drop gate needs both, `fail_below` the
/// current. `--gate-scope changed` needs only the current: with nothing
/// graded at the ref it still fails when duplicated lines grow.
fn gate_failure(score_diff: &diff::ScoreDiff, gate: ScoreGate) -> Option<String> {
    let after = score_diff.after?;
    let drop_failure = gate
        .max_drop
        .and_then(|tolerance| match &score_diff.changed {
            Some(scope) => {
                changed::gate_failure(scope, tolerance, score_diff.before.map(|b| b.score))
            }
            None => {
                let (before, delta) = (score_diff.before?.score, score_diff.delta?);
                gate_score_worsened(before, after.score, tolerance)
                    .then(|| format_gate_error(before, after.score, delta, tolerance))
            }
        });
    drop_failure.or_else(|| {
        let (grade, threshold) = (after.grade, gate.fail_below?);
        (grade.numeric_rank() < threshold.numeric_rank()).then(|| {
            format!("quality gate failed: score {grade} is below minimum threshold {threshold}")
        })
    })
}

/// A note on the gates that [`gate_failure`] skipped for lack of anything
/// to grade in `target`, now or at the ref, so a gate that passes for that
/// reason says so.
fn skipped_gates(score_diff: &diff::ScoreDiff, gate: ScoreGate, target: &str) -> Option<String> {
    let compared = match score_diff.changed {
        Some(_) => score_diff.after.is_some(),
        None => score_diff.delta.is_some(),
    };
    let drop = gate.max_drop.is_some() && !compared;
    let below = gate.fail_below.is_some() && score_diff.after.is_none();
    let flags = match (drop, below) {
        (true, true) => "--fail-if-worse and --fail-below",
        (true, false) => "--fail-if-worse",
        (false, true) => "--fail-below",
        (false, false) => return None,
    };
    let place = match score_diff.after {
        None => target.to_string(),
        Some(_) => format!("{target} at {}", score_diff.git_ref),
    };
    Some(format!(
        "note: nothing to grade in {place} (no code files measured); {flags} skipped"
    ))
}

/// Walk all source files, compute per-file and project-level metrics,
/// normalize each dimension, and produce the final `ProjectScore`.
pub(crate) fn compute_score(
    cfg: &WalkConfig<'_>,
    bottom: usize,
    min_lines: usize,
    model: &ScoringModel,
) -> Result<ProjectScore, Box<dyn Error>> {
    compute_snapshot(cfg, bottom, min_lines, model).map(|s| s.score)
}

/// A `ProjectScore` plus the per-file and duplication data the gates compare.
pub(crate) struct Snapshot {
    pub(crate) score: ProjectScore,
    files: Vec<FileScore>,
    duplicated_lines: usize,
}

impl Snapshot {
    /// Every scored file, worst first.
    pub(crate) fn files(&self) -> &[FileScore] {
        &self.files
    }

    /// File scores keyed by path relative to `root`, the directory that was walked.
    fn scores_by_path(&self, root: &Path) -> HashMap<PathBuf, f64> {
        self.files
            .iter()
            .map(|f| {
                let rel = f.path.strip_prefix(root).unwrap_or(&f.path);
                (rel.to_path_buf(), f.score)
            })
            .collect()
    }
}

/// The metrics of every scored file, and the duplication among them.
pub(crate) struct Measured {
    file_metrics: Vec<FileMetrics>,
    dup_percent: f64,
    duplicated_lines: usize,
    /// Code files that could not be read.
    unreadable: usize,
}

impl Measured {
    /// The measurement, unless there were code files and none could be
    /// read: that is an error, not a target with nothing to grade, so a
    /// gate cannot pass on it.
    pub(crate) fn readable(self) -> Result<Self, String> {
        if self.file_metrics.is_empty() && self.unreadable > 0 {
            return Err(format!(
                "cannot grade: the code could not be read (unreadable files: {})",
                self.unreadable
            ));
        }
        Ok(self)
    }
}

/// Walk the code files and measure what `model` scores, duplication included.
/// A caller that grades takes the result through [`Measured::readable`].
pub(crate) fn measure(cfg: &WalkConfig<'_>, min_lines: usize, model: &ScoringModel) -> Measured {
    let mut file_metrics: Vec<FileMetrics> = Vec::new();
    let mut dup_files: Vec<dups::detector::NormalizedFile> = Vec::new();
    let mut total_code_lines: usize = 0;

    let (analyzed, unreadable) = collector::analyze_files(cfg, model);
    for result in analyzed {
        total_code_lines += result.normalized_count;
        dup_files.push(result.dup_file);
        file_metrics.push(result.metrics);
    }

    let dup_groups = if dup_files.is_empty() {
        Vec::new()
    } else {
        dups::detector::detect_duplicates(&dup_files, min_lines, true)
    };
    let duplicated_lines: usize = crate::dups::detector::duplicated_line_count(&dup_groups);
    let dup_percent = if total_code_lines == 0 {
        0.0
    } else {
        duplicated_lines as f64 / total_code_lines as f64 * 100.0
    };
    Measured {
        file_metrics,
        dup_percent,
        duplicated_lines,
        unreadable,
    }
}

/// Score what was measured with `model`, keeping the `bottom` worst files.
pub(crate) fn snapshot_of(measured: &Measured, bottom: usize, model: &ScoringModel) -> Snapshot {
    let file_metrics = &measured.file_metrics;
    let total_loc: usize = file_metrics.iter().map(|f| f.code_lines).sum();
    let dimensions = if file_metrics.is_empty() {
        build_empty_dimensions(model)
    } else {
        build_dimensions(file_metrics, total_loc, measured.dup_percent, model)
    };
    let graded = compute_project_score(&dimensions).map(Graded::of);
    let mut file_scores: Vec<FileScore> =
        file_metrics.iter().map(|f| score_file(f, model)).collect();
    file_scores.sort_by(|a, b| a.score.total_cmp(&b.score));
    let needs_attention = file_scores.iter().take(bottom).cloned().collect();

    Snapshot {
        score: ProjectScore {
            graded,
            files_analyzed: file_metrics.len(),
            total_loc,
            dimensions,
            needs_attention,
        },
        files: file_scores,
        duplicated_lines: measured.duplicated_lines,
    }
}

fn compute_snapshot(
    cfg: &WalkConfig<'_>,
    bottom: usize,
    min_lines: usize,
    model: &ScoringModel,
) -> Result<Snapshot, Box<dyn Error>> {
    let measured = measure(cfg, min_lines, model).readable()?;
    Ok(snapshot_of(&measured, bottom, model))
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
