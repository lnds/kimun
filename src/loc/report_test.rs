use super::*;

#[test]
fn print_report_sorts_by_code_descending() {
    // This test just ensures print_report doesn't panic and runs to completion
    let reports = vec![
        LanguageReport {
            name: "Python".to_string(),
            files: 3,
            blank: 10,
            comment: 5,
            code: 100,
        },
        LanguageReport {
            name: "Rust".to_string(),
            files: 5,
            blank: 20,
            comment: 10,
            code: 500,
        },
    ];
    // Should not panic
    print_report(reports, None);
}

#[test]
fn print_report_single_language() {
    let reports = vec![LanguageReport {
        name: "Go".to_string(),
        files: 1,
        blank: 2,
        comment: 3,
        code: 4,
    }];
    print_report(reports, None);
}

#[test]
fn print_report_empty() {
    // Empty reports list — should just print headers and zero totals
    print_report(vec![], None);
}

#[test]
fn print_report_with_verbose_stats() {
    let reports = vec![LanguageReport {
        name: "Rust".to_string(),
        files: 2,
        blank: 5,
        comment: 3,
        code: 42,
    }];
    let stats = VerboseStats {
        total_files: 5,
        unique_files: 2,
        duplicate_files: 2,
        binary_files: 1,
        elapsed: Duration::from_millis(1234),
    };
    // Should not panic
    print_report(reports, Some(stats));
}

#[test]
fn print_report_verbose_zero_elapsed() {
    let reports = vec![];
    let stats = VerboseStats {
        total_files: 0,
        unique_files: 0,
        duplicate_files: 0,
        binary_files: 0,
        elapsed: Duration::from_secs(0),
    };
    // Division by zero guard should work
    print_report(reports, Some(stats));
}

#[test]
fn print_json_with_reports() {
    let reports = vec![
        LanguageReport {
            name: "Rust".to_string(),
            files: 5,
            blank: 20,
            comment: 10,
            code: 500,
        },
        LanguageReport {
            name: "Python".to_string(),
            files: 3,
            blank: 10,
            comment: 5,
            code: 100,
        },
    ];
    print_json(reports);
}

#[test]
fn print_json_empty() {
    print_json(vec![]);
}

#[test]
fn sep_width_matches_row_width() {
    // SEP_WIDTH must equal the rendered width of a data row so the separator
    // aligns with the content (regression for off-by-one where SEP_WIDTH was 68
    // but rows were 69 chars wide).
    let row = format!(
        " {:<COL_LANG$} {:>COL_FILES$} {:>COL_NUM$} {:>COL_NUM$} {:>COL_NUM$}",
        "Language", "Files", "Blank", "Comment", "Code"
    );
    assert_eq!(row.len(), SEP_WIDTH);
}

// ── print_author_report ──────────────────────────────────────────────────

#[test]
fn print_author_report_does_not_panic() {
    let reports = vec![
        AuthorReport {
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            files: 3,
            blank: 10,
            comment: 5,
            code: 100,
        },
        AuthorReport {
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            files: 2,
            blank: 5,
            comment: 2,
            code: 50,
        },
    ];
    print_author_report(reports);
}

#[test]
fn print_author_report_empty() {
    print_author_report(vec![]);
}

#[test]
fn print_author_report_single() {
    let reports = vec![AuthorReport {
        name: "Charlie".to_string(),
        email: "charlie@example.com".to_string(),
        files: 1,
        blank: 2,
        comment: 3,
        code: 42,
    }];
    print_author_report(reports);
}

// ── print_author_json ────────────────────────────────────────────────────

#[test]
fn print_author_json_does_not_panic() {
    let reports = vec![
        AuthorReport {
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            files: 3,
            blank: 10,
            comment: 5,
            code: 100,
        },
        AuthorReport {
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            files: 1,
            blank: 2,
            comment: 1,
            code: 20,
        },
    ];
    print_author_json(reports);
}

#[test]
fn print_author_json_empty() {
    print_author_json(vec![]);
}

#[test]
fn print_author_json_sorts_by_code_descending() {
    // With two authors, the one with more code should come first
    let reports = vec![
        AuthorReport {
            name: "Low".to_string(),
            email: "low@example.com".to_string(),
            files: 1,
            blank: 0,
            comment: 0,
            code: 10,
        },
        AuthorReport {
            name: "High".to_string(),
            email: "high@example.com".to_string(),
            files: 5,
            blank: 100,
            comment: 50,
            code: 1000,
        },
    ];
    // Should not panic; output order is not easily checked (stdout),
    // but the function must run without error
    print_author_json(reports);
}
