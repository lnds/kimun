//! Output of `km docs refs`: table, JSON, one line, and CI annotations.

use std::error::Error;

use serde::Serialize;

use super::RefsReport;
use super::markdown::Form;
use super::refs::Missing;
use crate::report_helpers;

const COL_LINE: usize = 5;
const TITLE: &str = "Dead reference";

/// Each dead reference as its document shown, the reference and why.
fn rows(report: &RefsReport) -> impl Iterator<Item = (String, &super::refs::Dead)> {
    report.with_dead.iter().flat_map(|document| {
        let shown = report_helpers::shown(&document.path);
        document.dead.iter().map(move |dead| (shown.clone(), dead))
    })
}

pub fn print_report(report: &RefsReport) {
    if report.documents == 0 {
        println!("No Markdown documents found.");
        return;
    }
    let rows: Vec<_> = rows(report).collect();
    if !rows.is_empty() {
        let col_doc = rows
            .iter()
            .map(|(doc, _)| report_helpers::display_width(doc))
            .max()
            .unwrap_or(0)
            .max(8);
        let col_ref = rows
            .iter()
            .map(|(_, dead)| report_helpers::display_width(&dead.reference))
            .max()
            .unwrap_or(0)
            .max(9);
        let separator = report_helpers::separator(1 + col_doc + 1 + COL_LINE + 1 + col_ref + 16);
        println!("{separator}");
        println!(
            " {} {:>COL_LINE$} {} Problem",
            report_helpers::pad_to("Document", col_doc),
            "Line",
            report_helpers::pad_to("Reference", col_ref),
        );
        println!("{separator}");
        for (doc, dead) in &rows {
            println!(
                " {} {:>COL_LINE$} {} {}",
                report_helpers::pad_to(doc, col_doc),
                dead.line,
                report_helpers::pad_to(&dead.reference, col_ref),
                dead.missing.label(),
            );
        }
        println!("{separator}");
    }
    println!(
        " {} dead of {} references, in {} of {} documents",
        report.dead(),
        report.references,
        report.with_dead.len(),
        report.documents,
    );
}

#[derive(Serialize)]
struct JsonDead<'a> {
    document: String,
    line: usize,
    reference: &'a str,
    form: Form,
    missing: Missing,
}

#[derive(Serialize)]
struct JsonReport<'a> {
    documents: usize,
    references: usize,
    dead: Vec<JsonDead<'a>>,
}

pub fn print_json(report: &RefsReport) -> Result<(), Box<dyn Error>> {
    let dead = rows(report)
        .map(|(document, dead)| JsonDead {
            document,
            line: dead.line,
            reference: &dead.reference,
            form: dead.form,
            missing: dead.missing,
        })
        .collect();
    report_helpers::print_json_stdout(&JsonReport {
        documents: report.documents,
        references: report.references,
        dead,
    })
}

pub fn print_short(report: &RefsReport) {
    println!(
        "refs documents:{} references:{} dead:{}",
        report.documents,
        report.references,
        report.dead()
    );
}

fn message(dead: &super::refs::Dead) -> String {
    format!("{}: {}", dead.reference, dead.missing.label())
}

pub fn print_github(report: &RefsReport) {
    for (document, dead) in rows(report) {
        report_helpers::github_annotation("warning", &document, dead.line, TITLE, &message(dead));
    }
}

pub fn print_codeclimate(report: &RefsReport) -> Result<(), Box<dyn Error>> {
    let entries: Vec<_> = rows(report)
        .map(|(document, dead)| {
            let title = format!("{TITLE}: {}", dead.reference);
            report_helpers::codeclimate_entry("minor", &document, dead.line, &title, &message(dead))
        })
        .collect();
    report_helpers::print_json_stdout(&entries)
}
