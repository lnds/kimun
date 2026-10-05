//! Report formatters for impact analysis.
//!
//! The table is built as a `String` so tests assert on the text.

use serde::Serialize;

use super::Impact;
use super::analyzer::{MissingCoChange, Trigger};
use super::projects::{self, JsonProjects};
use super::structural::{self, JsonStructural};
use crate::report_helpers;

const MIN_WIDTH: usize = 78;
/// Beyond this the table of missing files gives way to stacked rows.
const MAX_WIDTH: usize = 100;
const COL_CONFIDENCE: usize = 10;
const COL_SHARED: usize = 7;

/// Print the impact report as a table.
pub fn print_report(impact: &Impact, top: usize) {
    print!("{}", render_report(impact, top));
}

/// Render the project block, the diffusion block, the logical radius table
/// and the notes.
fn render_report(impact: &Impact, top: usize) -> String {
    if impact.diffusion.files == 0 {
        let mut lines = vec![format!("No changes: {}.", impact.source)];
        lines.extend(generated_note(impact));
        return lines.join("\n") + "\n";
    }

    let shown = &impact.missing[..impact.missing.len().min(top)];
    let (width, rows) = match shown {
        [] => (
            MIN_WIDTH,
            vec![format!(
                " No missing co-changes (confidence >= {:.2}, shared commits >= {}).",
                impact.thresholds.min_confidence, impact.thresholds.min_shared
            )],
        ),
        _ => missing_rows(shown),
    };
    let sep = report_helpers::separator(width);

    let mut lines = vec![format!("Change Impact — {}", impact.source), String::new()];
    lines.extend(projects::render(&impact.projects));
    lines.push(String::new());
    lines.extend(structural::render(&impact.structural, top));
    lines.push(String::new());
    lines.extend(diffusion_lines(impact));
    lines.push(String::new());
    lines.push(
        "Logical radius — files that usually change with this diff and are not in it".to_string(),
    );
    lines.push(sep.clone());
    lines.extend(rows);
    lines.push(sep);
    if impact.missing.len() > shown.len() {
        lines.push(format!(
            "{} missing files ({} shown).",
            impact.missing.len(),
            shown.len()
        ));
    }
    lines.extend(history_note(impact));
    lines.extend(skipped_note(impact));
    lines.extend(generated_note(impact));
    lines.join("\n") + "\n"
}

/// The rows that list the missing files, and the width they span: a table
/// when it fits in `MAX_WIDTH`, and otherwise each file on a line of its own
/// with its evidence below, so that long paths are not cut.
fn missing_rows(shown: &[MissingCoChange]) -> (usize, Vec<String>) {
    let path_width = report_helpers::max_path_width(shown.iter().map(|m| m.path.as_path()), 12);
    let with_width = shown
        .iter()
        .map(|m| report_helpers::display_width(&changes_with(m)))
        .max()
        .unwrap_or(0)
        .max(12);
    let width = (path_width + COL_CONFIDENCE + COL_SHARED + with_width + 7).max(MIN_WIDTH);
    let evidence = |m: &MissingCoChange| {
        let best = m.best();
        (
            format!("{:.2}", best.confidence),
            format!("{}/{}", best.shared_commits, best.commits),
        )
    };

    if width > MAX_WIDTH {
        let rows = shown.iter().flat_map(|m| {
            let (confidence, shared) = evidence(m);
            [
                format!(" {}", m.path.display()),
                format!(
                    "     confidence {confidence}, shared {shared} with {}",
                    changes_with(m)
                ),
            ]
        });
        return (MIN_WIDTH, rows.collect());
    }

    let mut rows = vec![
        row(
            path_width,
            [&"Missing file", &"Confidence", &"Shared", &"Changes with"],
        ),
        report_helpers::separator(width),
    ];
    rows.extend(shown.iter().map(|m| {
        let (confidence, shared) = evidence(m);
        row(
            path_width,
            [&m.path.display(), &confidence, &shared, &changes_with(m)],
        )
    }));
    (width, rows)
}

/// One row of the logical radius table.
fn row(path_width: usize, cells: [&dyn std::fmt::Display; 4]) -> String {
    let [path, confidence, shared, with] = cells;
    format!(" {path:<path_width$}  {confidence:>COL_CONFIDENCE$}  {shared:>COL_SHARED$}  {with}")
}

/// The strongest trigger, and how many more predict the same file.
fn changes_with(missing: &MissingCoChange) -> String {
    let best = missing.best().path.display();
    match missing.triggers.len() - 1 {
        0 => best.to_string(),
        more => format!("{best} (+{more} more)"),
    }
}

fn diffusion_lines(impact: &Impact) -> Vec<String> {
    let d = &impact.diffusion;
    let count = |label: &str, value: usize| format!("  {label:<16} {value:>8}");
    vec![
        "Diffusion".to_string(),
        count("Files changed", d.files),
        count("Directories", d.directories),
        count("Subsystems", d.subsystems),
        count("Lines added", d.lines_added),
        count("Lines deleted", d.lines_deleted),
        format!(
            "  {:<16} {:>8.2}  (0 = one file holds the change, 1 = evenly spread)",
            "Entropy", d.entropy
        ),
    ]
}

/// Name the changed files nothing can be expected of, so that their silence
/// is not read as "no impact".
fn history_note(impact: &Impact) -> Option<String> {
    if impact.without_history.is_empty() {
        return None;
    }
    let paths: Vec<String> = impact
        .without_history
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    Some(format!(
        "No history before the diff (new or never committed): {}",
        paths.join(", ")
    ))
}

/// Say how much history was set aside, so a short radius can be told from
/// a history made of sweeping commits.
fn skipped_note(impact: &Impact) -> Option<String> {
    (impact.skipped_commits > 0).then(|| {
        format!(
            "Commits ignored for touching more than {} files: {}",
            impact.max_changeset, impact.skipped_commits
        )
    })
}

fn generated_note(impact: &Impact) -> Option<String> {
    (impact.generated_skipped > 0)
        .then(|| format!("Generated files ignored: {}", impact.generated_skipped))
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[derive(Serialize)]
struct JsonTrigger {
    path: String,
    shared_commits: usize,
    commits: usize,
    confidence: f64,
}

impl From<&Trigger> for JsonTrigger {
    fn from(t: &Trigger) -> Self {
        Self {
            path: t.path.display().to_string(),
            shared_commits: t.shared_commits,
            commits: t.commits,
            confidence: round2(t.confidence),
        }
    }
}

#[derive(Serialize)]
struct JsonMissing {
    path: String,
    confidence: f64,
    shared_commits: usize,
    triggers: Vec<JsonTrigger>,
}

#[derive(Serialize)]
struct JsonDiffusion {
    files: usize,
    directories: usize,
    subsystems: usize,
    lines_added: usize,
    lines_deleted: usize,
    entropy: f64,
}

#[derive(Serialize)]
struct JsonLogicalRadius {
    min_confidence: f64,
    min_shared: usize,
    max_changeset: usize,
    skipped_commits: usize,
    total_missing: usize,
    missing: Vec<JsonMissing>,
    without_history: Vec<String>,
}

#[derive(Serialize)]
struct JsonImpact<'a> {
    source: String,
    projects: JsonProjects,
    structural: JsonStructural<'a>,
    diffusion: JsonDiffusion,
    logical_radius: JsonLogicalRadius,
    generated_skipped: usize,
}

fn to_json(impact: &Impact, top: usize) -> JsonImpact<'_> {
    let d = &impact.diffusion;
    JsonImpact {
        source: impact.source.clone(),
        projects: JsonProjects::from(&impact.projects),
        structural: JsonStructural::from(&impact.structural),
        diffusion: JsonDiffusion {
            files: d.files,
            directories: d.directories,
            subsystems: d.subsystems,
            lines_added: d.lines_added,
            lines_deleted: d.lines_deleted,
            entropy: round2(d.entropy),
        },
        logical_radius: JsonLogicalRadius {
            min_confidence: impact.thresholds.min_confidence,
            min_shared: impact.thresholds.min_shared,
            max_changeset: impact.max_changeset,
            skipped_commits: impact.skipped_commits,
            total_missing: impact.missing.len(),
            missing: impact
                .missing
                .iter()
                .take(top)
                .map(|m| JsonMissing {
                    path: m.path.display().to_string(),
                    confidence: round2(m.best().confidence),
                    shared_commits: m.best().shared_commits,
                    triggers: m.triggers.iter().map(JsonTrigger::from).collect(),
                })
                .collect(),
            without_history: impact
                .without_history
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
        },
        generated_skipped: impact.generated_skipped,
    }
}

/// Serialize the impact report as pretty-printed JSON to stdout.
pub fn print_json(impact: &Impact, top: usize) -> Result<(), Box<dyn std::error::Error>> {
    report_helpers::print_json_stdout(&to_json(impact, top))
}

/// The impact report as a single compact line.
fn render_short(impact: &Impact) -> String {
    let d = &impact.diffusion;
    let max_confidence = impact.missing.first().map_or(0.0, |m| m.best().confidence);
    format!(
        "impact projects_reached:{}/{} structural:{}/{} unprotected:{} files:{} dirs:{} subsystems:{} added:{} deleted:{} entropy:{:.2} missing:{} max_confidence:{:.2}",
        impact.projects.reached.len(),
        impact.projects.total(),
        impact.structural.reached(),
        impact.structural.source_files,
        impact.structural.unprotected().count(),
        d.files,
        d.directories,
        d.subsystems,
        d.lines_added,
        d.lines_deleted,
        d.entropy,
        impact.missing.len(),
        max_confidence,
    )
}

/// Print the impact report as a single compact line.
pub fn print_short(impact: &Impact) {
    println!("{}", render_short(impact));
}

/// Print only the number of missing co-changes.
pub fn print_terse(impact: &Impact) {
    println!("{}", impact.missing.len());
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
