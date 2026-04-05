//! Report formatters for code smell analysis.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

use super::analyzer::{FileSmells, SmellKind};
use crate::report_helpers;

/// Per-file smell metrics for reporting.
pub struct FileSmellMetrics {
    pub path: PathBuf,
    pub language: String,
    pub smells: FileSmells,
    pub total: usize,
}

/// Print a table of per-file code smell counts.
pub fn print_report(files: &[FileSmellMetrics]) {
    if files.is_empty() {
        println!("No code smells found.");
        return;
    }

    let max_path_len = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);
    let header_width = max_path_len + 40;
    let separator = report_helpers::separator(header_width.max(55));

    println!("Code Smells");
    println!("{separator}");
    println!(
        " {:<width$}  {:>7}  Top Smell",
        "File",
        "Smells",
        width = max_path_len
    );
    println!("{separator}");

    for f in files {
        let top = top_smell(&f.smells);
        println!(
            " {:<width$}  {:>7}  {}",
            f.path.display(),
            f.total,
            top,
            width = max_path_len
        );
    }

    println!("{separator}");

    let total_smells: usize = files.iter().map(|f| f.total).sum();
    let total_label = format!(" Total ({} files)", files.len());
    println!(
        "{:<width$}  {:>7}",
        total_label,
        total_smells,
        width = max_path_len + 1
    );
}

/// Find the most common smell kind in a file and format it as "kind (count)".
fn top_smell(smells: &FileSmells) -> String {
    let mut counts: HashMap<SmellKind, usize> = HashMap::new();
    for s in &smells.smells {
        *counts.entry(s.kind).or_default() += 1;
    }

    counts
        .into_iter()
        .max_by_key(|&(_, c)| c)
        .map(|(kind, count)| format!("{} ({count})", kind.as_str()))
        .unwrap_or_default()
}

/// JSON-serializable smell instance.
#[derive(Serialize)]
struct JsonSmell {
    kind: SmellKind,
    line: usize,
    detail: String,
}

/// JSON-serializable file entry.
#[derive(Serialize)]
struct JsonFileEntry {
    path: String,
    language: String,
    smells: Vec<JsonSmell>,
    total: usize,
}

/// Serialize per-file smell metrics as JSON to stdout.
pub fn print_json(files: &[FileSmellMetrics]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<JsonFileEntry> = files
        .iter()
        .map(|f| JsonFileEntry {
            path: f.path.display().to_string(),
            language: f.language.clone(),
            smells: f
                .smells
                .smells
                .iter()
                .map(|s| JsonSmell {
                    kind: s.kind,
                    line: s.line,
                    detail: s.detail.clone(),
                })
                .collect(),
            total: f.total,
        })
        .collect();

    report_helpers::print_json_stdout(&entries)
}

/// Emit one GitHub Actions warning annotation per smell instance.
/// Each annotation links directly to the file and line in the PR diff.
pub fn print_github(files: &[FileSmellMetrics]) {
    for f in files {
        let path = f.path.display().to_string();
        for s in &f.smells.smells {
            report_helpers::github_annotation(
                "warning",
                &path,
                s.line,
                s.kind.title(),
                &s.detail,
            );
        }
    }
}
