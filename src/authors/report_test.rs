use super::*;
use crate::authors::analyzer::AuthorSummary;

fn sample() -> Vec<AuthorSummary> {
    vec![
        AuthorSummary {
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            owned_files: 10,
            lines: 500,
            languages: vec!["Python".to_string(), "Rust".to_string()],
            last_active: 1_700_000_000,
        },
        AuthorSummary {
            name: "Björn Ångström".to_string(),
            email: "bjorn@example.com".to_string(),
            owned_files: 3,
            lines: 120,
            languages: vec!["Rust".to_string()],
            last_active: 1_690_000_000,
        },
    ]
}

#[test]
fn print_report_does_not_panic() {
    print_report(&sample());
}

#[test]
fn print_report_empty() {
    print_report(&[]);
}

#[test]
fn print_json_does_not_panic() {
    print_json(&sample());
}

#[test]
fn print_json_empty() {
    print_json(&[]);
}

#[test]
fn separator_matches_row_width() {
    // Verify that the separator and content rows have the same display width
    // (regression: mismatched widths cause visual misalignment).
    let authors = sample();

    let col_author = authors
        .iter()
        .map(|a| UnicodeWidthStr::width(a.name.as_str()))
        .max()
        .unwrap_or(0)
        .max(UnicodeWidthStr::width("Author"));

    let col_langs = authors
        .iter()
        .map(|a| a.languages.join(", ").len())
        .max()
        .unwrap_or(0)
        .max("Languages".len());

    let sep_width = 1 + col_author + 1 + COL_OWNED + 1 + COL_LINES + 2 + col_langs + 1 + COL_DATE;

    // Build a header row and verify its display width matches sep_width.
    let row = format!(
        " {} {:>COL_OWNED$} {:>COL_LINES$}  {:<col_langs$} {:>COL_DATE$}",
        pad_to("Author", col_author),
        "Owned",
        "Lines",
        "Languages",
        "Last Active",
    );

    assert_eq!(UnicodeWidthStr::width(row.as_str()), sep_width);
}
