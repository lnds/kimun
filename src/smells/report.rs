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

/// Print smell counts as a single compact line.
pub fn print_short(files: &[FileSmellMetrics]) {
    let count = files.len();
    let total_smells: usize = files.iter().map(|f| f.total).sum();
    println!("smells files:{count} total:{total_smells}");
}

/// Print only the total smell count.
pub fn print_terse(files: &[FileSmellMetrics]) {
    let total_smells: usize = files.iter().map(|f| f.total).sum();
    println!("{total_smells}");
}

/// Emit one GitHub Actions warning annotation per smell instance.
/// Each annotation links directly to the file and line in the PR diff.
pub fn print_github(files: &[FileSmellMetrics]) {
    for f in files {
        let path = f.path.display().to_string();
        for s in &f.smells.smells {
            report_helpers::github_annotation("warning", &path, s.line, s.kind.title(), &s.detail);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_smell(
        kind: SmellKind,
        line: usize,
        detail: &str,
    ) -> super::super::analyzer::SmellInstance {
        super::super::analyzer::SmellInstance {
            kind,
            line,
            detail: detail.to_string(),
        }
    }

    fn make_file_metrics(
        path: &str,
        lang: &str,
        smells: Vec<super::super::analyzer::SmellInstance>,
    ) -> FileSmellMetrics {
        let total = smells.len();
        FileSmellMetrics {
            path: PathBuf::from(path),
            language: lang.to_string(),
            smells: FileSmells { smells },
            total,
        }
    }

    #[test]
    fn print_report_empty_does_not_panic() {
        print_report(&[]);
    }

    #[test]
    fn print_report_with_smells_does_not_panic() {
        let smells = vec![
            make_smell(SmellKind::LongFunction, 1, "long_func (60 lines)"),
            make_smell(SmellKind::TodoDebt, 10, "TODO: fix this"),
        ];
        let files = vec![make_file_metrics("src/main.rs", "Rust", smells)];
        print_report(&files);
    }

    #[test]
    fn print_report_multiple_files() {
        let s1 = vec![make_smell(SmellKind::MagicNumber, 5, "42")];
        let s2 = vec![
            make_smell(SmellKind::LongParameterList, 3, "6 params"),
            make_smell(SmellKind::CommentedOutCode, 7, "let x = 1;"),
        ];
        let files = vec![
            make_file_metrics("src/a.rs", "Rust", s1),
            make_file_metrics("src/b.rs", "Rust", s2),
        ];
        print_report(&files);
    }

    #[test]
    fn print_json_empty_does_not_panic() {
        print_json(&[]).unwrap();
    }

    #[test]
    fn print_json_with_smells_does_not_panic() {
        let smells = vec![
            make_smell(SmellKind::LongFunction, 1, "big_func (55 lines)"),
            make_smell(SmellKind::TodoDebt, 20, "FIXME: broken"),
        ];
        let files = vec![make_file_metrics("src/lib.rs", "Rust", smells)];
        print_json(&files).unwrap();
    }

    #[test]
    fn top_smell_returns_most_common() {
        // Two TodoDebt, one LongFunction => TodoDebt wins
        let smells = vec![
            make_smell(SmellKind::TodoDebt, 1, "TODO"),
            make_smell(SmellKind::TodoDebt, 2, "FIXME"),
            make_smell(SmellKind::LongFunction, 3, "big_func"),
        ];
        let top = top_smell(&FileSmells { smells });
        assert!(top.contains("todo_debt"), "expected todo_debt, got: {top}");
        assert!(top.contains("(2)"), "expected count 2, got: {top}");
    }

    #[test]
    fn top_smell_empty_smells_returns_empty_string() {
        let top = top_smell(&FileSmells { smells: vec![] });
        assert!(top.is_empty(), "expected empty string, got: {top}");
    }
}
