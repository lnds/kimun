/// Report formatters for lines-of-code results.
///
/// Provides a cloc-style table (sorted by code lines descending),
/// a by-author table (sorted by code lines descending), and
/// a JSON output with per-language and total breakdowns.
use std::time::Duration;

use serde::Serialize;
use unicode_width::UnicodeWidthStr;

use crate::report_helpers;

/// Per-language line count summary.
#[derive(Debug, Serialize)]
pub struct LanguageReport {
    pub name: String,
    pub files: usize,
    pub blank: usize,
    pub comment: usize,
    pub code: usize,
}

/// Timing and file-count statistics shown in verbose mode.
pub struct VerboseStats {
    pub total_files: usize,
    pub unique_files: usize,
    pub duplicate_files: usize,
    pub binary_files: usize,
    pub elapsed: Duration,
}

/// Print a cloc-style table with per-language line counts and totals.
/// When `verbose` is provided, prints file counts and throughput first.
pub fn print_report(mut reports: Vec<LanguageReport>, verbose: Option<VerboseStats>) {
    reports.sort_by(|a, b| b.code.cmp(&a.code));

    if let Some(stats) = &verbose {
        let secs = stats.elapsed.as_secs_f64();
        let files_per_sec = if secs > 0.0 {
            stats.unique_files as f64 / secs
        } else {
            0.0
        };
        let total_lines: usize = reports.iter().map(|r| r.blank + r.comment + r.code).sum();
        let lines_per_sec = if secs > 0.0 {
            total_lines as f64 / secs
        } else {
            0.0
        };
        println!("{:>8} text files.", stats.total_files);
        println!("{:>8} unique files.", stats.unique_files);
        println!("{:>8} files ignored (duplicates).", stats.duplicate_files);
        println!("{:>8} files ignored (binary).", stats.binary_files);
        println!(
            "T={:.2} s ({:.1} files/s, {:.1} lines/s)",
            secs, files_per_sec, lines_per_sec
        );
        println!();
    }

    let separator = report_helpers::separator(68);

    println!("{separator}");
    println!(
        " {:<20} {:>8} {:>12} {:>12} {:>12}",
        "Language", "Files", "Blank", "Comment", "Code"
    );
    println!("{separator}");

    let mut total_files = 0usize;
    let mut total_blank = 0usize;
    let mut total_comment = 0usize;
    let mut total_code = 0usize;

    for r in &reports {
        println!(
            " {:<20} {:>8} {:>12} {:>12} {:>12}",
            r.name, r.files, r.blank, r.comment, r.code
        );
        total_files += r.files;
        total_blank += r.blank;
        total_comment += r.comment;
        total_code += r.code;
    }

    println!("{separator}");
    println!(
        " {:<20} {:>8} {:>12} {:>12} {:>12}",
        "SUM:", total_files, total_blank, total_comment, total_code
    );
    println!("{separator}");
}

/// JSON envelope with per-language details and aggregated totals.
#[derive(Serialize)]
struct JsonOutput {
    languages: Vec<LanguageReport>,
    totals: JsonTotals,
}

/// Aggregated totals across all languages.
#[derive(Serialize)]
struct JsonTotals {
    files: usize,
    blank: usize,
    comment: usize,
    code: usize,
}

/// Serialize line counts as pretty-printed JSON to stdout.
pub fn print_json(mut reports: Vec<LanguageReport>) {
    reports.sort_by(|a, b| b.code.cmp(&a.code));

    let totals = JsonTotals {
        files: reports.iter().map(|r| r.files).sum(),
        blank: reports.iter().map(|r| r.blank).sum(),
        comment: reports.iter().map(|r| r.comment).sum(),
        code: reports.iter().map(|r| r.code).sum(),
    };

    let output = JsonOutput {
        languages: reports,
        totals,
    };

    report_helpers::print_json_stdout(&output).unwrap();
}

/// Per-author line count summary across all files.
#[derive(Debug, Serialize)]
pub struct AuthorReport {
    pub name: String,
    pub email: String,
    pub files: usize,
    pub blank: usize,
    pub comment: usize,
    pub code: usize,
}

/// Left-pad `s` to `width` terminal display columns using `unicode-width`.
/// Rust's built-in `{:<N}` counts codepoints, not display columns — for
/// names with combining characters or CJK the two can diverge.
fn pad_to(s: &str, width: usize) -> String {
    let display_w = UnicodeWidthStr::width(s);
    let padding = width.saturating_sub(display_w);
    format!("{s}{}", " ".repeat(padding))
}

/// Print a cloc-style table with per-author line counts and totals.
/// The author column width adapts to the longest name in the dataset,
/// measured in terminal display columns rather than codepoints.
pub fn print_author_report(mut reports: Vec<AuthorReport>) {
    reports.sort_by(|a, b| b.code.cmp(&a.code));

    let col_author = reports
        .iter()
        .map(|r| UnicodeWidthStr::width(r.name.as_str()))
        .max()
        .unwrap_or(0)
        .max(UnicodeWidthStr::width("Author"));

    let sep_width = 1 + col_author + 1 + COL_FILES + 1 + COL_NUM + 1 + COL_NUM + 1 + COL_NUM;
    let separator = report_helpers::separator(sep_width);

    println!("{separator}");
    println!(
        " {} {:>COL_FILES$} {:>COL_NUM$} {:>COL_NUM$} {:>COL_NUM$}",
        pad_to("Author", col_author),
        "Files",
        "Blank",
        "Comment",
        "Code"
    );
    println!("{separator}");

    let mut total_files = 0usize;
    let mut total_blank = 0usize;
    let mut total_comment = 0usize;
    let mut total_code = 0usize;

    for r in &reports {
        println!(
            " {} {:>COL_FILES$} {:>COL_NUM$} {:>COL_NUM$} {:>COL_NUM$}",
            pad_to(&r.name, col_author),
            r.files,
            r.blank,
            r.comment,
            r.code
        );
        total_files += r.files;
        total_blank += r.blank;
        total_comment += r.comment;
        total_code += r.code;
    }

    println!("{separator}");
    println!(
        " {} {:>COL_FILES$} {:>COL_NUM$} {:>COL_NUM$} {:>COL_NUM$}",
        pad_to("SUM:", col_author),
        total_files,
        total_blank,
        total_comment,
        total_code
    );
    println!("{separator}");
}

/// Serialize by-author line counts as pretty-printed JSON to stdout.
pub fn print_author_json(mut reports: Vec<AuthorReport>) {
    reports.sort_by(|a, b| b.code.cmp(&a.code));

    #[derive(Serialize)]
    struct JsonOutput {
        authors: Vec<AuthorReport>,
        totals: JsonTotals,
    }

    let totals = JsonTotals {
        files: reports.iter().map(|r| r.files).sum(),
        blank: reports.iter().map(|r| r.blank).sum(),
        comment: reports.iter().map(|r| r.comment).sum(),
        code: reports.iter().map(|r| r.code).sum(),
    };

    report_helpers::print_json_stdout(&JsonOutput {
        authors: reports,
        totals,
    })
    .unwrap();
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
