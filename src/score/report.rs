//! Report formatters for the overall code health score.
//!
//! Provides two output modes: a formatted table showing per-dimension
//! breakdown and worst-scoring files, and a JSON format for machine
//! consumption. The table includes thousand-separated LOC counts and
//! truncated file paths for readability.

use serde::Serialize;

use super::analyzer::{DimensionScore, Grade, Graded, ProjectScore, grade_text, score_text};
use crate::report_helpers;

/// Print the project health score as a formatted table.
///
/// Shows the overall grade, per-dimension breakdown (name, weight, score,
/// grade), and a "needs attention" section listing the worst-scoring files
/// with their issues. File paths are truncated with `...` prefix when they
/// exceed the column width.
pub fn print_report(score: &ProjectScore, bottom: usize, target: Option<&str>) {
    let separator = report_helpers::separator(66);

    let header = match target {
        Some(t) if score.files_analyzed == 1 => format!("File Score: {t}"),
        Some(t) => format!("Code Health Score: {t}"),
        None => "Code Health Score".to_string(),
    };
    println!("{header}");
    println!("{separator}");
    println!("{}", project_line(score));
    println!(" Files Analyzed: {}", score.files_analyzed);
    println!(" Total LOC:      {}", format_thousands(score.total_loc));
    println!("{separator}");
    println!(
        " {:<25} {:>6}   {:>5}   {:<5}",
        "Dimension", "Weight", "Score", "Grade"
    );
    println!("{separator}");

    for d in &score.dimensions {
        println!("{}", dimension_row(d));
    }

    println!("{separator}");
    print!("{}", dimension_note(score));

    if score.needs_attention.is_empty() {
        return;
    }

    let show = bottom.min(score.needs_attention.len());
    println!();
    println!(" Files Needing Attention (worst per-file scores, excl. duplication)");
    println!("{separator}");

    let max_path = score.needs_attention[..show]
        .iter()
        .map(|f| crate::report_helpers::shown(&f.path).len())
        .max()
        .unwrap() // safe: show > 0 because needs_attention is non-empty
        .clamp(4, 40);

    println!(
        " {:>5}  {:<5}  {:<width$}   Issues",
        "Score",
        "Grade",
        "File",
        width = max_path
    );
    println!("{separator}");

    for f in &score.needs_attention[..show] {
        let path_str = crate::report_helpers::shown(&f.path);
        let truncated = if path_str.len() > max_path {
            format!("...{}", &path_str[path_str.len() - max_path + 3..])
        } else {
            path_str
        };
        println!(
            " {:>5.1}  {:<5}  {:<width$}   {}",
            f.score,
            f.grade.as_str(),
            truncated,
            f.issues.join(", "),
            width = max_path
        );
    }

    println!("{separator}");
}

/// The project score line of the table: grade and score, or that there
/// is nothing to grade.
fn project_line(score: &ProjectScore) -> String {
    match score.graded {
        Some(g) => format!(" Project Score:  {} ({:.1})", g.grade, g.score),
        None => " Project Score:  - (nothing to grade)".to_string(),
    }
}

/// The line under the table that says why a project with a score shows a
/// dimension without one, or nothing when every dimension was measured.
fn dimension_note(score: &ProjectScore) -> String {
    let unmeasured = score.dimensions.iter().filter(|d| d.graded.is_none());
    let note = score
        .graded
        .and(unmeasured_note(unmeasured.map(|d| d.name)));
    note.map_or_else(String::new, |note| format!(" {note}\n"))
}

/// The note for the dimensions named, as `Halstead Effort not measured; its
/// weight is spread over the other dimensions.`, or `None` for none.
pub(super) fn unmeasured_note<'a>(names: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let names: Vec<&str> = names.into_iter().collect();
    let (last, rest) = names.split_last()?;
    let (listed, its) = match rest {
        [] => (last.to_string(), "its"),
        _ => (format!("{} and {last}", rest.join(", ")), "their"),
    };
    Some(format!(
        "{listed} not measured; {its} weight is spread over the other dimensions."
    ))
}

/// A dimension row of the table.
fn dimension_row(d: &DimensionScore) -> String {
    format!(
        " {:<25} {:>5.0}%   {:>5}   {:<5}",
        d.name,
        d.weight * 100.0,
        score_text(d.graded, 1),
        grade_text(d.graded),
    )
}

/// Format an integer with thousand separators (e.g. 1234567 → "1,234,567").
fn format_thousands(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

/// Print score as a single compact line.
pub fn print_short(score: &ProjectScore) {
    println!("{}", short(score));
}

/// The compact line: `-` for what has no score.
fn short(score: &ProjectScore) -> String {
    let dim_scores: Vec<String> = score
        .dimensions
        .iter()
        .map(|d| {
            let key = d
                .name
                .split_whitespace()
                .next()
                .unwrap_or(d.name)
                .to_lowercase();
            format!("{key}:{}", score_text(d.graded, 1))
        })
        .collect();
    format!(
        "score s:{} g:{} files:{} loc:{} {}",
        score_text(score.graded, 1),
        grade_text(score.graded),
        score.files_analyzed,
        score.total_loc,
        dim_scores.join(" "),
    )
}

/// Print only the overall score value, `-` when there is nothing to grade.
pub fn print_terse(score: &ProjectScore) {
    println!("{}", score_text(score.graded, 1));
}

/// A score and its grade in JSON: both null when there is no score (nothing
/// to grade, or a dimension nothing measured), so every key is always there.
#[derive(Serialize)]
pub(super) struct JsonGraded {
    score: Option<f64>,
    grade: Option<Grade>,
}

impl From<Option<Graded>> for JsonGraded {
    fn from(graded: Option<Graded>) -> Self {
        Self {
            score: graded.map(|g| g.score),
            grade: graded.map(|g| g.grade),
        }
    }
}

/// JSON-serializable representation of a single score dimension.
#[derive(Serialize)]
struct JsonDimension {
    /// Human-readable dimension name (e.g. "Maintainability Index").
    name: String,
    /// Relative weight in the final score (0.0–1.0, all sum to 1.0). The
    /// weight of a dimension whose score is null goes to the others.
    weight: f64,
    /// Normalized score (0–100 scale) and its letter grade, both null when
    /// no file of the target has the metric.
    #[serde(flatten)]
    graded: JsonGraded,
}

/// JSON-serializable representation of a per-file score with issues.
#[derive(Serialize)]
struct JsonFileScore {
    /// Relative file path from the analysis root.
    path: String,
    /// Weighted per-file score (0–100 scale).
    score: f64,
    /// Letter grade for this file's health.
    grade: Grade,
    /// Human-readable issue descriptions (e.g. "low MI: 45.2").
    issues: Vec<String>,
}

/// JSON-serializable representation of the full project score output.
#[derive(Serialize)]
struct JsonProjectScore {
    /// Target path or file name (omitted when analyzing "." default).
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    /// Overall weighted project score (0–100 scale) and grade (A++ to F--).
    #[serde(flatten)]
    graded: JsonGraded,
    /// Number of source files included in the analysis.
    files_analyzed: usize,
    /// Total lines of code across all analyzed files.
    total_loc: usize,
    /// Per-dimension breakdown of the score.
    dimensions: Vec<JsonDimension>,
    /// Worst-scoring files that need attention.
    needs_attention: Vec<JsonFileScore>,
    /// The score of the default model, when this one is experimental.
    #[serde(skip_serializing_if = "Option::is_none")]
    baseline: Option<JsonBaseline>,
}

/// The score of the default model, next to an experimental one.
#[derive(Serialize)]
struct JsonBaseline {
    #[serde(flatten)]
    graded: JsonGraded,
}

/// Serialize the project score to pretty-printed JSON and print to stdout.
pub fn print_json(
    score: &ProjectScore,
    target: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    print_json_against(score, target, None)
}

/// The same, with the score of the default model it is compared to.
pub fn print_json_against(
    score: &ProjectScore,
    target: Option<&str>,
    baseline: Option<&ProjectScore>,
) -> Result<(), Box<dyn std::error::Error>> {
    report_helpers::print_json_stdout(&to_json(score, target, baseline))
}

/// The JSON of the score. Every key is present; what there is nothing to
/// grade is null.
fn to_json(
    score: &ProjectScore,
    target: Option<&str>,
    baseline: Option<&ProjectScore>,
) -> JsonProjectScore {
    JsonProjectScore {
        target: target.map(String::from),
        graded: score.graded.into(),
        files_analyzed: score.files_analyzed,
        total_loc: score.total_loc,
        dimensions: score
            .dimensions
            .iter()
            .map(|d| JsonDimension {
                name: d.name.to_string(),
                weight: d.weight,
                graded: d.graded.into(),
            })
            .collect(),
        needs_attention: score
            .needs_attention
            .iter()
            .map(|f| JsonFileScore {
                path: crate::report_helpers::shown(&f.path),
                score: f.score,
                grade: f.grade,
                issues: f.issues.clone(),
            })
            .collect(),
        baseline: baseline.map(|b| JsonBaseline {
            graded: b.graded.into(),
        }),
    }
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
