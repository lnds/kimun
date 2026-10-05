//! The structural radius as text and as JSON.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::structural::{Dependent, Exposure, Narrowing, Structural};

const TITLE: &str = "Structural radius — source files that use what changed";

fn names(paths: impl Iterator<Item = impl AsRef<Path>>, limit: usize) -> String {
    let all: Vec<String> = paths.map(|p| p.as_ref().display().to_string()).collect();
    let shown = all[..all.len().min(limit)].join(", ");
    match all.len().saturating_sub(limit) {
        0 => shown,
        more => format!("{shown} and {more} more"),
    }
}

/// The exposed dependents, most exposed first, each with what exposes it.
fn dependent_lines(radius: &Structural, top: usize) -> Vec<String> {
    let exposed: Vec<&Dependent> = radius.exposed().collect();
    let mut lines = vec![" Tests  Dependent".to_string()];
    for dependent in exposed.iter().take(top) {
        let tests = match dependent.tests {
            0 => "none".to_string(),
            n => n.to_string(),
        };
        lines.push(format!(" {tests:>5}  {}", dependent.file.display()));
        if dependent.exposure == Exposure::Refers {
            lines.push("            refers to the module without calling it".to_string());
        } else if !dependent.calls.is_empty() {
            lines.push(format!(
                "            calls {}",
                names(dependent.calls.iter(), 3)
            ));
        }
    }
    if exposed.len() > top {
        lines.push(format!(" {} files ({top} shown).", exposed.len()));
    }
    lines
}

/// How far the change reaches when every use of a changed file counts.
pub(super) fn reach_line(radius: &Structural) -> String {
    let near: Vec<String> = radius
        .by_distance
        .iter()
        .take(3)
        .enumerate()
        .map(|(i, files)| format!("{} at distance {}", files, i + 1))
        .collect();
    let further: usize = radius.by_distance.iter().skip(3).sum();
    match further {
        0 => near.join(", "),
        _ => format!(
            "{}, {further} further (up to distance {})",
            near.join(", "),
            radius.by_distance.len()
        ),
    }
}

/// The origins that could not be narrowed to functions, each with why.
fn unnarrowed(radius: &Structural) -> String {
    let why: Vec<String> = radius
        .narrowing
        .iter()
        .filter_map(|n| Some(format!("{} ({})", n.file.display(), n.reason.as_ref()?)))
        .collect();
    names(why.iter(), 3)
}

/// The lines that sum the radius up, above the list of dependents.
pub(super) fn summary(radius: &Structural) -> Vec<String> {
    let total = radius.source_files.max(1) as f64;
    // A share too small to round to 1% is still not zero.
    let share = |files: usize| match 100.0 * files as f64 / total {
        pct if pct > 0.0 && pct < 0.5 => "<1%".to_string(),
        pct => format!("{pct:.0}%"),
    };
    let reached = radius.reached();
    let changed = format!(" Changed: {}", names(radius.origins.iter(), 5));
    let unprotected = radius.unprotected().count();
    let Some(functions) = &radius.functions else {
        return vec![
            format!(
                " {reached} of {} source files reached ({}), {} direct, {unprotected} of them with no test",
                radius.source_files,
                share(reached),
                radius.direct.len()
            ),
            changed,
            format!(" Every use counts: {}", unnarrowed(radius)),
            format!(" Reach: {}", reach_line(radius)),
        ];
    };
    let calling = radius
        .direct
        .iter()
        .filter(|d| d.exposure == Exposure::Calls)
        .count();
    let radius_line = match reached {
        0 => format!(" Radius: 0 of {} source files", radius.source_files),
        _ => format!(
            " Radius: {reached} of {} source files ({}): {}",
            radius.source_files,
            share(reached),
            reach_line(radius)
        ),
    };
    vec![
        format!(
            " {}, {unprotected} of them with no test",
            match calling {
                1 => "1 file calls what changed".to_string(),
                n => format!("{n} files call what changed"),
            }
        ),
        changed,
        format!(" Functions: {}", names(functions.iter(), 8)),
    ]
    .into_iter()
    .chain(
        radius
            .narrowing
            .iter()
            .any(|n| n.functions.is_none())
            .then(|| format!(" Every use counts for: {}", unnarrowed(radius))),
    )
    .chain([
        radius_line,
        format!(
            " Upper bound, whatever the function: {} files ({})",
            radius.upper_bound,
            share(radius.upper_bound)
        ),
    ])
    .collect()
}

/// What the radius comes to.
fn body(radius: &Structural, top: usize) -> Vec<String> {
    if radius.origins.is_empty() {
        return vec![
            " No changed source file in a language with a reliable graph".to_string(),
            " (Elixir, JavaScript/TypeScript, Kaikai).".to_string(),
        ];
    }
    if radius.direct.is_empty() {
        return vec![
            format!(" 0 of {} source files reached.", radius.source_files),
            format!(" Changed: {}", names(radius.origins.iter(), 5)),
            " No source file measured uses what changed.".to_string(),
        ];
    }
    let mut lines = summary(radius);
    if radius.exposed().next().is_none() {
        lines.push(" No other file calls the functions that changed.".to_string());
        return lines;
    }
    lines.push(String::new());
    lines.extend(dependent_lines(radius, top));
    lines
}

/// The warnings the radius is measured for.
fn warnings(radius: &Structural) -> Vec<String> {
    let mut lines = Vec::new();
    let unprotected: Vec<&Path> = radius.unprotected().map(|d| d.file.as_path()).collect();
    if !unprotected.is_empty() {
        lines.push(format!(
            "No test refers to {} of the files that call what changed; an integration test is probably missing:",
            unprotected.len()
        ));
        lines.extend(
            unprotected
                .iter()
                .take(5)
                .map(|p| format!("  {}", p.display())),
        );
        if unprotected.len() > 5 {
            lines.push(format!("  and {} more", unprotected.len() - 5));
        }
    }
    let exercised = radius.exposed().any(|d| d.tests_in_diff > 0);
    if radius.exposed().next().is_some() && !exercised {
        lines.push("No test in the change exercises a file that uses what changed.".to_string());
    }
    let unknown = radius
        .direct
        .iter()
        .filter(|d| d.exposure == Exposure::Refers && d.tests == 0)
        .count();
    if unknown > 0 {
        lines.push(format!(
            "{unknown} more with no test use the module without a call that tells whether the change concerns them."
        ));
    }
    lines.extend(radius.unavailable.iter().map(|(language, files)| {
        format!(
            "Not measured for {language} ({files} changed): its graph does not reflect usage yet."
        )
    }));
    if radius.ambiguous > 0 {
        lines.push(format!(
            "Module references left out for naming more than one file: {}",
            radius.ambiguous
        ));
    }
    lines
}

/// The structural block of the report.
pub fn render(radius: &Structural, top: usize) -> Vec<String> {
    let sep = crate::report_helpers::separator(78);
    let mut lines = vec![TITLE.to_string(), sep.clone()];
    lines.extend(body(radius, top));
    lines.push(sep);
    lines.extend(warnings(radius));
    lines
}

#[derive(Serialize)]
struct JsonUnavailable<'a> {
    language: &'a str,
    files: usize,
}

/// The radius as a number: the files the change concerns, over those measured.
#[derive(Serialize)]
struct JsonRadius {
    files: usize,
    source_files: usize,
    /// `files / source_files`, from 0 to 1.
    share: f64,
}

#[derive(Serialize)]
struct JsonReach<'a> {
    distance: usize,
    files: &'a [PathBuf],
}

/// The structural block of the JSON report.
#[derive(Serialize)]
pub struct JsonStructural<'a> {
    source_files: usize,
    origins: Vec<String>,
    /// Public functions the change affects; null when it cannot be narrowed.
    functions: Option<&'a [String]>,
    /// Per changed file: the functions affected, or why they are not told.
    narrowing: &'a [Narrowing],
    radius: JsonRadius,
    reached: usize,
    by_distance: &'a [usize],
    /// The files of the radius, distance by distance.
    reach: Vec<JsonReach<'a>>,
    /// Files reached if every use of a changed file counted.
    upper_bound: usize,
    direct: &'a [Dependent],
    /// Files that call what changed and that no test protects.
    unprotected: Vec<String>,
    /// Files that use the changed module without a call that tells whether
    /// the change concerns them, and that no test protects.
    unknown_without_tests: Vec<String>,
    /// Whether a test that is part of the change protects a file that uses
    /// what changed.
    change_tests_a_dependent: bool,
    unavailable: Vec<JsonUnavailable<'a>>,
    ambiguous: usize,
}

impl<'a> From<&'a Structural> for JsonStructural<'a> {
    fn from(radius: &'a Structural) -> Self {
        let text = |p: &PathBuf| p.display().to_string();
        let reached = radius.reached();
        let share = reached as f64 / radius.source_files.max(1) as f64;
        Self {
            source_files: radius.source_files,
            origins: radius.origins.iter().map(text).collect(),
            functions: radius.functions.as_deref(),
            narrowing: &radius.narrowing,
            radius: JsonRadius {
                files: reached,
                source_files: radius.source_files,
                share: (share * 10_000.0).round() / 10_000.0,
            },
            reached,
            by_distance: &radius.by_distance,
            reach: radius
                .reached_files
                .iter()
                .enumerate()
                .map(|(i, files)| JsonReach {
                    distance: i + 1,
                    files,
                })
                .collect(),
            upper_bound: radius.upper_bound,
            direct: &radius.direct,
            unprotected: radius.unprotected().map(|d| text(&d.file)).collect(),
            unknown_without_tests: radius
                .direct
                .iter()
                .filter(|d| d.exposure == Exposure::Refers && d.tests == 0)
                .map(|d| text(&d.file))
                .collect(),
            change_tests_a_dependent: radius.exposed().any(|d| d.tests_in_diff > 0),
            unavailable: radius
                .unavailable
                .iter()
                .map(|(language, files)| JsonUnavailable {
                    language,
                    files: *files,
                })
                .collect(),
            ambiguous: radius.ambiguous,
        }
    }
}
