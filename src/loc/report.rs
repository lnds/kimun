use std::time::Duration;

use serde::Serialize;

use crate::report_helpers;

#[derive(Debug, Serialize)]
pub struct LanguageReport {
    pub name: String,
    pub files: usize,
    pub blank: usize,
    pub comment: usize,
    pub code: usize,
}

pub struct VerboseStats {
    pub total_files: usize,
    pub unique_files: usize,
    pub skipped_files: usize,
    pub elapsed: Duration,
}

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
        println!("{:>8} files skipped.", stats.skipped_files);
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

#[derive(Serialize)]
struct JsonOutput {
    languages: Vec<LanguageReport>,
    totals: JsonTotals,
}

#[derive(Serialize)]
struct JsonTotals {
    files: usize,
    blank: usize,
    comment: usize,
    code: usize,
}

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

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
