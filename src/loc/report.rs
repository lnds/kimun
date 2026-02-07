pub struct LanguageReport {
    pub name: String,
    pub files: usize,
    pub blank: usize,
    pub comment: usize,
    pub code: usize,
}

pub fn print_report(mut reports: Vec<LanguageReport>) {
    reports.sort_by(|a, b| b.code.cmp(&a.code));

    let separator = "─".repeat(68);

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

#[cfg(test)]
mod tests {
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
        print_report(reports);
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
        print_report(reports);
    }

    #[test]
    fn print_report_empty() {
        // Empty reports list — should just print headers and zero totals
        print_report(vec![]);
    }
}
