/// Report formatters for author summary analysis.
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::analyzer::AuthorSummary;
use crate::report_helpers;

const COL_OWNED: usize = 7;
const COL_LINES: usize = 10;
const COL_DATE: usize = 11;

/// Format a Unix timestamp as `YYYY-MM-DD`.
fn format_date(ts: i64) -> String {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Print a table of per-author summaries sorted by lines descending.
pub fn print_report(authors: &[AuthorSummary]) {
    if authors.is_empty() {
        println!("No authors found.");
        return;
    }

    let col_author = authors
        .iter()
        .map(|a| report_helpers::display_width(a.name.as_str()))
        .max()
        .unwrap_or(0)
        .max(report_helpers::display_width("Author"));

    let col_langs = authors
        .iter()
        .map(|a| a.languages.join(", ").len())
        .max()
        .unwrap_or(0)
        .max("Languages".len());

    let sep_width = 1 + col_author + 1 + COL_OWNED + 1 + COL_LINES + 2 + col_langs + 1 + COL_DATE;
    let separator = report_helpers::separator(sep_width);

    println!("{separator}");
    println!(
        " {} {:>COL_OWNED$} {:>COL_LINES$}  {:<col_langs$} {:>COL_DATE$}",
        report_helpers::pad_to("Author", col_author),
        "Owned",
        "Lines",
        "Languages",
        "Last Active",
    );
    println!("{separator}");

    for a in authors {
        let langs = a.languages.join(", ");
        println!(
            " {} {:>COL_OWNED$} {:>COL_LINES$}  {:<col_langs$} {:>COL_DATE$}",
            report_helpers::pad_to(&a.name, col_author),
            a.owned_files,
            a.lines,
            langs,
            format_date(a.last_active),
        );
    }

    println!("{separator}");
}

/// JSON-serializable representation of a single author summary.
#[derive(Serialize)]
struct JsonEntry<'a> {
    name: &'a str,
    email: &'a str,
    owned_files: usize,
    lines: usize,
    languages: &'a [String],
    last_active: String,
}

/// Serialize author summaries as pretty-printed JSON to stdout.
pub fn print_json(authors: &[AuthorSummary]) {
    let entries: Vec<JsonEntry<'_>> = authors
        .iter()
        .map(|a| JsonEntry {
            name: &a.name,
            email: &a.email,
            owned_files: a.owned_files,
            lines: a.lines,
            languages: &a.languages,
            last_active: format_date(a.last_active),
        })
        .collect();

    report_helpers::print_json_stdout(&entries).unwrap();
}

/// Print authors as a single compact line.
pub fn print_short(authors: &[AuthorSummary]) {
    let count = authors.len();
    let total_files: usize = authors.iter().map(|a| a.owned_files).sum();
    println!("authors count:{count} files:{total_files}");
}

/// Print only the author count.
pub fn print_terse(authors: &[AuthorSummary]) {
    println!("{}", authors.len());
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
