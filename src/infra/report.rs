//! Output of `km infra smells`: table, JSON, one line, and CI annotations.

use std::error::Error;

use serde::Serialize;

use super::FileSmells;
use super::smells::{ALL, Found, Smell};
use crate::cli::OutputMode;
use crate::report_helpers::{self, table};

pub fn total(files: &[FileSmells]) -> usize {
    files.iter().map(|f| f.found.len()).sum()
}

/// Every smell with its file shown, the most severe first.
fn rows(files: &[FileSmells]) -> Vec<(String, &Found)> {
    let mut rows: Vec<(String, &Found)> = files
        .iter()
        .flat_map(|f| {
            let shown = report_helpers::shown(&f.path);
            f.found.iter().map(move |found| (shown.clone(), found))
        })
        .collect();
    rows.sort_by(|a, b| (a.1.smell, &a.0, a.1.line).cmp(&(b.1.smell, &b.0, b.1.line)));
    rows
}

pub fn print_report(files: &[FileSmells], read: usize, top: Option<usize>) {
    if read == 0 {
        println!("No infrastructure files found.");
        return;
    }
    let rows = rows(files);
    let shown = report_helpers::shown_count(OutputMode::Table, top, rows.len());
    let cells: Vec<[String; 4]> = rows[..shown]
        .iter()
        .map(|(path, found)| {
            [
                path.clone(),
                found.line.to_string(),
                found.smell.title().to_string(),
                found.detail.clone(),
            ]
        })
        .collect();
    table(
        "Infrastructure smells — security smells of infrastructure code",
        ["File", "Line", "Smell", "Detail"],
        &cells,
    );
    let counts: Vec<String> = ALL
        .iter()
        .map(|&smell| (smell, rows.iter().filter(|r| r.1.smell == smell).count()))
        .filter(|(_, count)| *count > 0)
        .map(|(smell, count)| format!("{} {count}", smell.title().to_lowercase()))
        .collect();
    println!();
    println!(
        " {} smells in {} of {read} files{}{}",
        rows.len(),
        files.len(),
        match counts.is_empty() {
            true => "",
            false => ": ",
        },
        counts.join(", "),
    );
}

#[derive(Serialize)]
struct JsonSmell<'a> {
    kind: Smell,
    line: usize,
    detail: &'a str,
}

#[derive(Serialize)]
struct JsonFile<'a> {
    path: String,
    language: &'a str,
    smells: Vec<JsonSmell<'a>>,
    total: usize,
}

pub fn print_json(files: &[FileSmells], top: Option<usize>) -> Result<(), Box<dyn Error>> {
    let shown = report_helpers::shown_count(OutputMode::Json, top, files.len());
    let entries: Vec<JsonFile> = files[..shown]
        .iter()
        .map(|f| JsonFile {
            path: report_helpers::shown(&f.path),
            language: f.language,
            smells: f
                .found
                .iter()
                .map(|s| JsonSmell {
                    kind: s.smell,
                    line: s.line,
                    detail: &s.detail,
                })
                .collect(),
            total: f.found.len(),
        })
        .collect();
    report_helpers::print_json_stdout(&entries)
}

pub fn print_short(files: &[FileSmells], read: usize) {
    println!(
        "infra smells files:{read} with_smells:{} total:{}",
        files.len(),
        total(files)
    );
}

pub fn print_github(files: &[FileSmells]) {
    for (path, found) in rows(files) {
        let level = match found.smell.severity() {
            "major" => "warning",
            _ => "notice",
        };
        let title = found.smell.title();
        report_helpers::github_annotation(level, &path, found.line, title, &found.detail);
    }
}

pub fn print_codeclimate(files: &[FileSmells]) -> Result<(), Box<dyn Error>> {
    let entries: Vec<_> = rows(files)
        .into_iter()
        .map(|(path, found)| {
            let smell = found.smell;
            report_helpers::codeclimate_entry(
                smell.severity(),
                &path,
                found.line,
                smell.title(),
                &found.detail,
            )
        })
        .collect();
    report_helpers::print_json_stdout(&entries)
}
