//! Output of `km docs stale`: table, JSON and one line.

use std::error::Error;

use chrono::DateTime;
use serde::Serialize;

use super::stale::StaleDocument;
use crate::cli::OutputMode;
use crate::report_helpers::{self, display_width, pad_to};

const DAY: i64 = 86_400;

fn date(time: i64) -> String {
    DateTime::from_timestamp(time, 0)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

fn describes(stale: &StaleDocument) -> String {
    let first = stale.files[0].path.display().to_string();
    match stale.files.len() {
        1 => first,
        n => format!("{first} (+{})", n - 1),
    }
}

pub fn print_report(stale: &[StaleDocument], documents: usize, top: Option<usize>) {
    if stale.is_empty() {
        println!("No document of {documents} is behind the code it describes.");
        return;
    }
    let shown = report_helpers::shown_count(OutputMode::Table, top, stale.len());
    let rows: Vec<[String; 5]> = stale[..shown]
        .iter()
        .map(|s| {
            [
                report_helpers::shown(&s.document),
                date(s.updated),
                format!("{}d", s.behind() / DAY),
                s.commits.to_string(),
                describes(s),
            ]
        })
        .collect();
    let doc = rows
        .iter()
        .map(|r| display_width(&r[0]))
        .chain([8])
        .max()
        .unwrap_or(8);
    let sep = report_helpers::separator(doc + 48);
    let line = |r: [&str; 5]| {
        format!(
            " {}  {:<10}  {:>6}  {:>7}  {}",
            pad_to(r[0], doc),
            r[1],
            r[2],
            r[3],
            r[4]
        )
    };
    println!("{sep}");
    println!(
        "{}",
        line(["Document", "Updated", "Behind", "Commits", "Describes"])
    );
    println!("{sep}");
    for [a, b, c, d, e] in &rows {
        println!("{}", line([a, b, c, d, e]));
    }
    println!("{sep}");
    println!(
        " {} of {documents} documents are behind the code they describe{}",
        stale.len(),
        match shown < stale.len() {
            true => format!(" ({shown} shown)"),
            false => String::new(),
        }
    );
}

#[derive(Serialize)]
struct JsonFile {
    path: String,
    changed: String,
    commits: usize,
}

#[derive(Serialize)]
struct JsonStale {
    document: String,
    updated: String,
    days_behind: i64,
    commits: usize,
    files: Vec<JsonFile>,
}

impl From<&StaleDocument> for JsonStale {
    fn from(s: &StaleDocument) -> Self {
        let file = |f: &super::stale::Behind| JsonFile {
            path: f.path.display().to_string(),
            changed: date(f.changed),
            commits: f.commits,
        };
        JsonStale {
            document: report_helpers::shown(&s.document),
            updated: date(s.updated),
            days_behind: s.behind() / DAY,
            commits: s.commits,
            files: s.files.iter().map(file).collect(),
        }
    }
}

pub fn print_json(stale: &[StaleDocument], top: Option<usize>) -> Result<(), Box<dyn Error>> {
    let shown = report_helpers::shown_count(OutputMode::Json, top, stale.len());
    let entries: Vec<JsonStale> = stale[..shown].iter().map(JsonStale::from).collect();
    report_helpers::print_json_stdout(&entries)
}

pub fn print_short(stale: &[StaleDocument], documents: usize) {
    let behind = stale.iter().map(|s| s.behind() / DAY).max().unwrap_or(0);
    println!(
        "stale documents:{documents} behind:{} max_days_behind:{behind}",
        stale.len()
    );
}
