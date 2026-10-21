//! The documentation block of the impact report: table, JSON and CI
//! annotations.

use serde::Serialize;

use super::documentation::{Signal, Stale, Why};
use crate::report_helpers::{self, display_width, pad_to};

const TITLE: &str = "Documentation not updated";

/// Why a document describes what changed, in words.
pub fn why(stale: &Stale) -> String {
    match &stale.why {
        Why::Covers(glob) => format!("covers {glob}"),
        Why::Names { line, gone: false } => format!("names it (line {line})"),
        Why::Names { line, gone: true } => format!("names it, now gone (line {line})"),
        Why::ChangedWith {
            shared, commits, ..
        } => format!("changed together {shared} of {commits} times"),
    }
}

fn describes(stale: &Stale) -> String {
    let first = stale
        .describes
        .first()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    match stale.describes.len() {
        0 | 1 => first,
        n => format!("{first} (+{})", n - 1),
    }
}

/// The block as lines; none when no document was left behind.
pub fn render(stale: &[Stale]) -> Vec<String> {
    if stale.is_empty() {
        return Vec::new();
    }
    let rows: Vec<[String; 3]> = stale
        .iter()
        .map(|s| [s.document.display().to_string(), describes(s), why(s)])
        .collect();
    let width = |i: usize, title: &str| {
        rows.iter()
            .map(|r| display_width(&r[i]))
            .chain([display_width(title)])
            .max()
            .unwrap_or(0)
    };
    let (doc, desc, reason) = (width(0, "Document"), width(1, "Describes"), width(2, "Why"));
    let sep = report_helpers::separator(doc + desc + reason + 5);
    let line = |cells: [&str; 3]| {
        format!(
            " {}  {}  {}",
            pad_to(cells[0], doc),
            pad_to(cells[1], desc),
            cells[2]
        )
    };
    let mut lines = vec![
        "Documentation — documents that describe what changed and were not updated".to_string(),
        sep.clone(),
        line(["Document", "Describes", "Why"]),
        sep.clone(),
    ];
    lines.extend(rows.iter().map(|[a, b, c]| line([a, b, c])));
    lines.push(sep);
    lines
}

#[derive(Serialize)]
pub struct JsonStale {
    document: String,
    signal: &'static str,
    describes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    covers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gone: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shared_commits: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commits: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    confidence: Option<f64>,
}

#[derive(Serialize)]
pub struct JsonDocumentation {
    stale: Vec<JsonStale>,
}

impl From<&[Stale]> for JsonDocumentation {
    fn from(stale: &[Stale]) -> Self {
        let stale = stale
            .iter()
            .map(|s| {
                let mut json = JsonStale {
                    document: s.document.display().to_string(),
                    signal: s.signal.name(),
                    describes: s
                        .describes
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect(),
                    covers: None,
                    line: None,
                    gone: None,
                    shared_commits: None,
                    commits: None,
                    confidence: None,
                };
                match &s.why {
                    Why::Covers(glob) => json.covers = Some(glob.clone()),
                    Why::Names { line, gone } => {
                        (json.line, json.gone) = (Some(*line), Some(*gone))
                    }
                    Why::ChangedWith {
                        shared,
                        commits,
                        confidence,
                    } => {
                        json.shared_commits = Some(*shared);
                        json.commits = Some(*commits);
                        json.confidence = Some((confidence * 100.0).round() / 100.0);
                    }
                }
                json
            })
            .collect();
        Self { stale }
    }
}

/// Each changed file a document describes, with the document to update.
fn annotations(stale: &[Stale]) -> impl Iterator<Item = (&Stale, String, String)> {
    stale.iter().flat_map(|s| {
        s.describes.iter().map(move |path| {
            let message = format!("{} describes this file: {}", s.document.display(), why(s));
            (s, path.display().to_string(), message)
        })
    })
}

/// GitHub annotations on the changed files: a warning when the document
/// says it covers or names the file, a notice when only history relates
/// them.
pub fn print_github(stale: &[Stale]) {
    for (s, file, message) in annotations(stale) {
        let level = match s.signal {
            Signal::CoChanged => "notice",
            _ => "warning",
        };
        report_helpers::github_annotation(level, &file, 1, TITLE, &message);
    }
}

pub fn print_codeclimate(stale: &[Stale]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<_> = annotations(stale)
        .map(|(s, file, message)| {
            let title = format!("{TITLE}: {}", s.document.display());
            report_helpers::codeclimate_entry("minor", &file, 1, &title, &message)
        })
        .collect();
    report_helpers::print_json_stdout(&entries)
}
