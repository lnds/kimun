//! Output of `km docs links`: the health of the graph of notes.

use std::error::Error;
use std::path::PathBuf;

use serde::Serialize;

use super::links::{Graph, is_entry_point};
use crate::cli::OutputMode;
use crate::report_helpers::{self, display_width, pad_to};

/// What `km docs links` found in the graph of notes.
pub struct LinksReport {
    pub notes: Vec<PathBuf>,
    pub links: usize,
    pub graph: Graph,
    /// Notes nothing links to that are no entry point.
    pub orphans: Vec<usize>,
    /// Notes linked to by two or more, most linked first.
    pub hubs: Vec<(usize, usize)>,
    pub groups: Vec<Vec<usize>>,
}

impl LinksReport {
    pub fn new(notes: Vec<PathBuf>, graph: Graph) -> Self {
        let fan_in = graph.fan_in();
        let orphans = match notes.len() {
            0 | 1 => Vec::new(),
            _ => (0..notes.len())
                .filter(|&n| fan_in[n] == 0 && !is_entry_point(&notes[n]))
                .collect(),
        };
        let mut hubs: Vec<(usize, usize)> = fan_in
            .iter()
            .enumerate()
            .filter(|(_, links)| **links > 1)
            .map(|(n, &links)| (n, links))
            .collect();
        hubs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| notes[a.0].cmp(&notes[b.0])));
        Self {
            links: graph.out.iter().map(|o| o.len()).sum(),
            groups: graph.groups(),
            notes,
            graph,
            orphans,
            hubs,
        }
    }

    fn shown(&self, note: usize) -> String {
        report_helpers::shown(&self.notes[note])
    }
}

pub fn print_report(report: &LinksReport, top: Option<usize>) {
    if report.notes.is_empty() {
        println!("No Markdown documents found.");
        return;
    }
    let limit = |total| report_helpers::shown_count(OutputMode::Table, top, total);
    println!(
        "Notes {}, links {}, broken {}, orphans {}, groups {} (largest {})",
        report.notes.len(),
        report.links,
        report.graph.broken.len(),
        report.orphans.len(),
        report.groups.len(),
        report.groups.first().map_or(0, Vec::len),
    );
    let broken: Vec<[String; 3]> = report.graph.broken[..limit(report.graph.broken.len())]
        .iter()
        .map(|b| [report.shown(b.from), b.line.to_string(), b.target.clone()])
        .collect();
    table(
        "Broken links — no file has the path, no note the name",
        ["Document", "Line", "Link"],
        &broken,
    );
    let orphans: Vec<[String; 1]> = report.orphans[..limit(report.orphans.len())]
        .iter()
        .map(|&n| [report.shown(n)])
        .collect();
    table("Orphan notes — no note links to them", ["Note"], &orphans);
    let hubs: Vec<[String; 2]> = report.hubs[..limit(report.hubs.len())]
        .iter()
        .map(|&(n, links)| [report.shown(n), links.to_string()])
        .collect();
    table(
        "Hubs — the notes most linked to",
        ["Note", "Linked from"],
        &hubs,
    );
    let islands = report.groups.iter().skip(1).filter(|g| g.len() > 1);
    let groups: Vec<[String; 2]> = islands
        .take(limit(report.groups.len()))
        .map(|g| {
            let names: Vec<String> = g.iter().take(3).map(|&n| report.shown(n)).collect();
            let more = match g.len() > 3 {
                true => format!(" (+{})", g.len() - 3),
                false => String::new(),
            };
            [g.len().to_string(), names.join(", ") + &more]
        })
        .collect();
    table(
        "Islands — groups of notes linked among themselves and to no other",
        ["Notes", "Group"],
        &groups,
    );
}

/// A titled table, when it has rows.
pub(super) fn table<const N: usize>(title: &str, header: [&str; N], rows: &[[String; N]]) {
    if rows.is_empty() {
        return;
    }
    let widths: Vec<usize> = (0..N)
        .map(|i| {
            rows.iter()
                .map(|r| display_width(&r[i]))
                .chain([display_width(header[i])])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let line = |cells: Vec<&str>| {
        let padded: Vec<String> = cells
            .iter()
            .zip(&widths)
            .map(|(cell, &width)| pad_to(cell, width))
            .collect();
        format!(" {}", padded.join("  ").trim_end())
    };
    let sep = report_helpers::separator(widths.iter().sum::<usize>() + 2 * N);
    println!();
    println!("{title}");
    println!("{sep}");
    println!("{}", line(header.to_vec()));
    println!("{sep}");
    for row in rows {
        println!("{}", line(row.iter().map(String::as_str).collect()));
    }
    println!("{sep}");
}

#[derive(Serialize)]
struct JsonBroken {
    document: String,
    line: usize,
    link: String,
}

#[derive(Serialize)]
struct JsonHub {
    note: String,
    linked_from: usize,
}

#[derive(Serialize)]
struct JsonLinks {
    notes: usize,
    links: usize,
    broken: Vec<JsonBroken>,
    orphans: Vec<String>,
    hubs: Vec<JsonHub>,
    groups: Vec<Vec<String>>,
}

pub fn print_json(report: &LinksReport, top: Option<usize>) -> Result<(), Box<dyn Error>> {
    let limit = |total| report_helpers::shown_count(OutputMode::Json, top, total);
    let json = JsonLinks {
        notes: report.notes.len(),
        links: report.links,
        broken: report
            .graph
            .broken
            .iter()
            .map(|b| JsonBroken {
                document: report.shown(b.from),
                line: b.line,
                link: b.target.clone(),
            })
            .collect(),
        orphans: report.orphans.iter().map(|&n| report.shown(n)).collect(),
        hubs: report.hubs[..limit(report.hubs.len())]
            .iter()
            .map(|&(n, links)| JsonHub {
                note: report.shown(n),
                linked_from: links,
            })
            .collect(),
        groups: report
            .groups
            .iter()
            .map(|g| g.iter().map(|&n| report.shown(n)).collect())
            .collect(),
    };
    report_helpers::print_json_stdout(&json)
}

pub fn print_short(report: &LinksReport) {
    println!(
        "links notes:{} links:{} broken:{} orphans:{} groups:{}",
        report.notes.len(),
        report.links,
        report.graph.broken.len(),
        report.orphans.len(),
        report.groups.len()
    );
}
