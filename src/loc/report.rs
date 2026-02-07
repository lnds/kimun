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
