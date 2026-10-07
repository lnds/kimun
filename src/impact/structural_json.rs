//! The structural radius as JSON.

use std::path::PathBuf;

use serde::Serialize;

use super::protection::Protection;
use super::structural::{Dependent, Exposure, Narrowing, Structural};

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
    /// Command-line tasks and scripts that call what changed and have no
    /// test. Nothing uses them, so they are not counted as unprotected.
    untested_entry_points: Vec<String>,
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
            untested_entry_points: radius
                .untested_entry_points()
                .map(|d| text(&d.file))
                .collect(),
            unknown_without_tests: radius
                .direct
                .iter()
                .filter(|d| d.exposure == Exposure::Refers && d.protection == Protection::None)
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
