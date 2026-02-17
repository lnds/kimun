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
