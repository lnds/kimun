use serde::Serialize;

use super::analyzer::FileOwnership;
use crate::report_helpers;

pub fn print_report(files: &[FileOwnership]) {
    if files.is_empty() {
        println!("No files found for knowledge map analysis.");
        return;
    }

    let max_path_len = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);
    let max_owner_len = files
        .iter()
        .map(|f| f.primary_owner.len())
        .max()
        .unwrap_or(5)
        .max(5);

    // path + 2 + lang(10) + 1 + lines(7) + 1 + owner + 1 + own%(5) + 1 + contrib(7) + 1 + risk(8) + 1
    let header_width = max_path_len + max_owner_len + 45;
    let separator = report_helpers::separator(header_width.max(78));

    println!("Knowledge Map — Code Ownership");
    println!("{separator}");
    println!(
        " {:<pw$}  {:>10} {:>7}  {:<ow$} {:>5} {:>7} {:>8}",
        "File",
        "Language",
        "Lines",
        "Owner",
        "Own%",
        "Contrib",
        "Risk",
        pw = max_path_len,
        ow = max_owner_len
    );
    println!("{separator}");

    for f in files {
        println!(
            " {:<pw$}  {:>10} {:>7}  {:<ow$} {:>4.0}% {:>7} {:>8}",
            f.path.display(),
            f.language,
            f.total_lines,
            f.primary_owner,
            f.ownership_pct,
            f.contributors,
            f.risk.label(),
            pw = max_path_len,
            ow = max_owner_len
        );
    }

    println!("{separator}");

    let loss_count = files.iter().filter(|f| f.knowledge_loss).count();
    if loss_count > 0 {
        println!();
        println!("Files with knowledge loss risk (primary owner inactive): {loss_count}");
        for f in files.iter().filter(|f| f.knowledge_loss) {
            println!("  {} ({})", f.path.display(), f.primary_owner);
        }
    }
}

#[derive(Serialize)]
struct JsonEntry {
    path: String,
    language: String,
    total_lines: usize,
    primary_owner: String,
    ownership_pct: f64,
    contributors: usize,
    risk: String,
    knowledge_loss: bool,
}

pub fn print_json(files: &[FileOwnership]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<JsonEntry> = files
        .iter()
        .map(|f| JsonEntry {
            path: f.path.display().to_string(),
            language: f.language.clone(),
            total_lines: f.total_lines,
            primary_owner: f.primary_owner.clone(),
            ownership_pct: (f.ownership_pct * 10.0).round() / 10.0,
            contributors: f.contributors,
            risk: f.risk.label().to_string(),
            knowledge_loss: f.knowledge_loss,
        })
        .collect();

    report_helpers::print_json_stdout(&entries)
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
