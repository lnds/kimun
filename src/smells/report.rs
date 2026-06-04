//! Report formatters for code smell analysis.

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

/// Count how many smells of each kind a file has, in canonical column order.
fn kind_counts(smells: &FileSmells) -> [usize; 5] {
    let mut counts = [0usize; 5];
    for s in &smells.smells {
        for (i, kind) in SmellKind::all().iter().enumerate() {
            if *kind == s.kind {
                counts[i] += 1;
                break;
            }
        }
    }
    counts
}

/// Print a table of per-file code smell counts with one column per smell kind.
pub fn print_report(files: &[FileSmellMetrics]) {
    if files.is_empty() {
        println!("No code smells found.");
        return;
    }

    let kinds = SmellKind::all();
    let max_path_len = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);

    // Per-file kind counts and column-wide totals (for the footer and column widths).
    let per_file: Vec<[usize; 5]> = files.iter().map(|f| kind_counts(&f.smells)).collect();
    let mut totals = [0usize; 5];
    for counts in &per_file {
        for (t, c) in totals.iter_mut().zip(counts) {
            *t += c;
        }
    }
    let total_smells: usize = files.iter().map(|f| f.total).sum();

    // Each kind column is as wide as the widest of its header or any count it must hold.
    let kind_widths: [usize; 5] = std::array::from_fn(|i| {
        kinds[i]
            .short_label()
            .len()
            .max(totals[i].to_string().len())
    });
    let total_col = "Total".len().max(total_smells.to_string().len());

    // Build the header row to derive the separator width.
    let mut header = format!(
        " {:<width$}  {:>tw$}",
        "File",
        "Total",
        width = max_path_len,
        tw = total_col
    );
    for (i, kind) in kinds.iter().enumerate() {
        header.push_str(&format!("  {:>w$}", kind.short_label(), w = kind_widths[i]));
    }
    let separator = report_helpers::separator(header.len().max(55));

    println!("Code Smells");
    println!("{separator}");
    println!("{header}");
    println!("{separator}");

    for (f, counts) in files.iter().zip(&per_file) {
        let mut row = format!(
            " {:<width$}  {:>tw$}",
            f.path.display(),
            f.total,
            width = max_path_len,
            tw = total_col
        );
        for (i, c) in counts.iter().enumerate() {
            row.push_str(&format!("  {:>w$}", c, w = kind_widths[i]));
        }
        println!("{row}");
    }

    println!("{separator}");

    let total_label = format!(" Total ({} files)", files.len());
    let mut footer = format!(
        "{:<width$}  {:>tw$}",
        total_label,
        total_smells,
        width = max_path_len + 1,
        tw = total_col
    );
    for (i, t) in totals.iter().enumerate() {
        footer.push_str(&format!("  {:>w$}", t, w = kind_widths[i]));
    }
    println!("{footer}");
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

/// Print smells as a single compact line.
pub fn print_short(files: &[FileSmellMetrics]) {
    let total_smells: usize = files.iter().map(|f| f.total).sum();
    let file_count = files.len();
    println!("smells files:{file_count} total:{total_smells}");
}

/// Print only the total smell count.
pub fn print_terse(files: &[FileSmellMetrics]) {
    let total_smells: usize = files.iter().map(|f| f.total).sum();
    println!("{total_smells}");
}

/// Emit a CodeClimate JSON array (GitLab Code Quality format) for all smell instances.
pub fn print_codeclimate(files: &[FileSmellMetrics]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<_> = files
        .iter()
        .flat_map(|f| {
            let path = f.path.display().to_string();
            f.smells.smells.iter().map(move |s| {
                report_helpers::codeclimate_entry("minor", &path, s.line, s.kind.title(), &s.detail)
            })
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
    fn kind_counts_tallies_each_kind_in_canonical_order() {
        // Canonical column order: [magic, long, param, todo, comm]
        let smells = vec![
            make_smell(SmellKind::TodoDebt, 1, "TODO"),
            make_smell(SmellKind::TodoDebt, 2, "FIXME"),
            make_smell(SmellKind::LongFunction, 3, "big_func"),
            make_smell(SmellKind::MagicNumber, 4, "42"),
            make_smell(SmellKind::MagicNumber, 5, "99"),
            make_smell(SmellKind::MagicNumber, 6, "7"),
        ];
        let counts = kind_counts(&FileSmells { smells });
        assert_eq!(counts, [3, 1, 0, 2, 0]);
    }

    #[test]
    fn kind_counts_empty_smells_is_all_zero() {
        let counts = kind_counts(&FileSmells { smells: vec![] });
        assert_eq!(counts, [0; 5]);
    }
}
