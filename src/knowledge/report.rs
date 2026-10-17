/// Report formatters for knowledge map (code ownership) analysis.
///
/// Provides table and JSON output showing per-file primary owner,
/// ownership concentration, contributor count, and knowledge loss risk.
use serde::Serialize;

use super::analyzer::{AUTHOR_SHARE, AuthorSummary, BusFactor, BusFactorEntry, FileOwnership};
use crate::report_helpers;

const COL_LANG: usize = 10;
const COL_LINES: usize = 7;
const COL_OWN_PCT: usize = 5; // "Own%"
const COL_CONTRIB: usize = 7; // "Contrib"
const COL_RISK: usize = 8; // "CRITICAL"
// 1 (lead) + 2 (after path) + 1 (after lang) + 2 (after lines) + 1 + 1 + 1 (between last cols)
const COL_SPACING: usize = 9;
const FIXED_WIDTH: usize =
    COL_SPACING + COL_LANG + COL_LINES + COL_OWN_PCT + COL_CONTRIB + COL_RISK;

/// Print a table of per-file ownership with risk assessment and
/// a summary of files at risk of knowledge loss (inactive primary owner).
pub fn print_report(files: &[FileOwnership]) {
    if files.is_empty() {
        println!("No files found for knowledge map analysis.");
        return;
    }

    let max_path_len = report_helpers::max_path_width(files.iter().map(|f| f.path.as_path()), 4);
    let max_owner_len = files
        .iter()
        .map(|f| report_helpers::display_width(&f.primary_owner))
        .max()
        .unwrap_or(5)
        .max(5);

    let header_width = max_path_len + max_owner_len + FIXED_WIDTH;
    let separator = report_helpers::separator(header_width.max(78));

    println!("Knowledge Map — Code Ownership");
    println!("{separator}");
    println!(
        " {:<pw$}  {:>10} {:>7}  {} {:>5} {:>7} {:>8}",
        "File",
        "Language",
        "Lines",
        report_helpers::pad_to("Owner", max_owner_len),
        "Own%",
        "Contrib",
        "Risk",
        pw = max_path_len,
    );
    println!("{separator}");

    for f in files {
        println!(
            " {:<pw$}  {:>10} {:>7}  {} {:>4.0}% {:>7} {:>8}",
            f.path.display(),
            f.language,
            f.total_lines,
            report_helpers::pad_to(&f.primary_owner, max_owner_len),
            f.ownership_pct,
            f.contributors,
            f.risk.label(),
            pw = max_path_len,
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

/// Print knowledge map as a single compact line.
pub fn print_short(files: &[FileOwnership]) {
    let count = files.len();
    let critical = files
        .iter()
        .filter(|f| f.risk == super::analyzer::RiskLevel::Critical)
        .count();
    let loss = files.iter().filter(|f| f.knowledge_loss).count();
    println!("knowledge files:{count} critical:{critical} loss_risk:{loss}");
}

/// Print only the count of files at critical risk.
pub fn print_terse(files: &[FileOwnership]) {
    let critical = files
        .iter()
        .filter(|f| f.risk == super::analyzer::RiskLevel::Critical)
        .count();
    println!("{critical}");
}

/// JSON-serializable representation of a single file's ownership data.
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

/// Serialize per-file ownership data as pretty-printed JSON to stdout.
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

const SUM_COL_FILES: usize = 5;
const SUM_COL_LINES: usize = 7;
const SUM_COL_RISK: usize = 8;
const SUM_COL_LOSS: usize = 4;
// spacing: 1+2+1+1+1+1+1 = 8 fixed chars between/around dynamic columns
const SUMMARY_SPACING: usize = 8;

/// Print a table aggregating ownership by author.
pub fn print_summary_report(authors: &[AuthorSummary]) {
    if authors.is_empty() {
        println!("No ownership data found.");
        return;
    }

    let max_author_len = authors
        .iter()
        .map(|a| report_helpers::display_width(&a.author))
        .max()
        .unwrap_or(6)
        .max(6);
    let max_lang_len = authors
        .iter()
        .map(|a| a.languages.join(", ").len())
        .max()
        .unwrap_or(9)
        .max(9);

    let header_width = max_author_len
        + max_lang_len
        + SUM_COL_FILES
        + SUM_COL_LINES
        + SUM_COL_RISK
        + SUM_COL_LOSS
        + SUMMARY_SPACING;
    let separator = report_helpers::separator(header_width.max(78));

    println!("Knowledge Summary — Ownership by Author");
    println!("{separator}");
    println!(
        " {:<aw$}  {:>SUM_COL_FILES$} {:>SUM_COL_LINES$}  {:<lw$} {:>SUM_COL_RISK$} {:>SUM_COL_LOSS$}",
        "Author",
        "Files",
        "Lines",
        "Languages",
        "Risk",
        "Loss",
        aw = max_author_len,
        lw = max_lang_len,
    );
    println!("{separator}");

    for a in authors {
        let langs = a.languages.join(", ");
        println!(
            " {:<aw$}  {:>SUM_COL_FILES$} {:>SUM_COL_LINES$}  {:<lw$} {:>SUM_COL_RISK$} {:>SUM_COL_LOSS$}",
            report_helpers::pad_to(&a.author, max_author_len),
            a.files_owned,
            a.total_lines,
            langs,
            a.worst_risk.label(),
            a.knowledge_loss_files,
            aw = max_author_len,
            lw = max_lang_len,
        );
    }

    println!("{separator}");
}

/// Print summary as a single compact line.
pub fn print_summary_short(authors: &[AuthorSummary]) {
    let count = authors.len();
    let total_files: usize = authors.iter().map(|a| a.files_owned).sum();
    let total_loss: usize = authors.iter().map(|a| a.knowledge_loss_files).sum();
    println!("knowledge-summary authors:{count} files:{total_files} loss_risk:{total_loss}");
}

/// Print only the author count from summary.
pub fn print_summary_terse(authors: &[AuthorSummary]) {
    println!("{}", authors.len());
}

/// JSON-serializable representation of a single author's ownership summary.
#[derive(Serialize)]
struct JsonSummaryEntry {
    author: String,
    files_owned: usize,
    total_lines: usize,
    languages: Vec<String>,
    worst_risk: String,
    knowledge_loss_files: usize,
}

/// Serialize per-author ownership summary as pretty-printed JSON to stdout.
pub fn print_summary_json(authors: &[AuthorSummary]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<JsonSummaryEntry> = authors
        .iter()
        .map(|a| JsonSummaryEntry {
            author: a.author.clone(),
            files_owned: a.files_owned,
            total_lines: a.total_lines,
            languages: a.languages.clone(),
            worst_risk: a.worst_risk.label().to_string(),
            knowledge_loss_files: a.knowledge_loss_files,
        })
        .collect();
    report_helpers::print_json_stdout(&entries)
}

/// Label of the risk a bus factor stands for.
fn bus_factor_risk(factor: usize) -> &'static str {
    match factor {
        0 => "CRITICAL — the authors of most of the code are already gone",
        1 => "CRITICAL — one person holds most project knowledge",
        2 => "HIGH — two people hold critical knowledge",
        3 => "MODERATE — three people hold critical knowledge",
        _ => "LOW — knowledge is distributed across several contributors",
    }
}

/// Render the bus factor report as a table, at most `top` contributors
/// beyond those counted in the factor. `since` is the activity window, if
/// one was given.
pub fn render_bus_factor_report(bf: &BusFactor, top: usize, since: Option<&str>) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    if bf.total_lines == 0 {
        out.push_str("No blame data found for bus factor analysis.\n");
        return out;
    }

    let _ = writeln!(out, "Project Bus Factor: {}", bf.factor);
    let _ = writeln!(out);
    if let (Some(pct), Some(since)) = (bf.orphaned_by_inactivity, since)
        && pct > 0.0
    {
        let _ = writeln!(
            out,
            " {pct:.0}% of the code is already in files whose authors have not committed in {since}."
        );
    }
    if bf.factor > 0 {
        let orphaned = bf
            .contributors
            .iter()
            .filter_map(|e| e.orphaned_pct)
            .next_back()
            .unwrap_or(0.0);
        let _ = writeln!(
            out,
            " If {}, {orphaned:.0}% of the code is in files with no author left.",
            if bf.factor == 1 {
                "this contributor leaves".to_string()
            } else {
                format!("these {} contributors leave", bf.factor)
            },
        );
    }
    let _ = writeln!(out, " Risk: {}", bus_factor_risk(bf.factor));
    let _ = writeln!(
        out,
        " Authors of a file hold at least {:.0}% of the lines of its main author; files weigh by lines; threshold {:.0}%.",
        AUTHOR_SHARE * 100.0,
        bf.threshold,
    );
    let _ = writeln!(out);

    let shown = bf.factor + top;
    let rows = &bf.contributors[..shown.min(bf.contributors.len())];
    let author = |e: &BusFactorEntry| match e.active {
        Some(false) => format!("{} (inactive)", e.author),
        _ => e.author.clone(),
    };
    let max_author_len = rows
        .iter()
        .map(|e| report_helpers::display_width(&author(e)))
        .max()
        .unwrap_or(6)
        .max(6);

    let header_width = 6 + max_author_len + 7 + 8 + 9 + 10;
    let separator = report_helpers::separator(header_width.max(70));

    let _ = writeln!(out, "{separator}");
    let _ = writeln!(
        out,
        " {:>4}  {:<aw$}  {:>5}  {:>8}  {:>7}  {:>8}",
        "Rank",
        "Author",
        "Files",
        "Lines",
        "Share",
        "Orphaned",
        aw = max_author_len,
    );
    let _ = writeln!(out, "{separator}");

    for (i, entry) in rows.iter().enumerate() {
        let orphaned = entry
            .orphaned_pct
            .map_or(String::new(), |pct| format!("{pct:>7.1}%"));
        let name = author(entry);
        let pad = max_author_len.saturating_sub(report_helpers::display_width(&name));
        let _ = writeln!(
            out,
            " {:>4}  {name}{:pad$}  {:>5}  {:>8}  {:>6.2}%  {:>8}",
            i + 1,
            "",
            entry.files,
            entry.lines,
            entry.pct,
            orphaned,
        );
    }

    let _ = writeln!(out, "{separator}");
    let hidden = bf.contributors.len() - rows.len();
    if hidden > 0 {
        let _ = writeln!(out, " {hidden} more contributors (see --top)");
    }
    out
}

/// Print the bus factor report as a human-readable table.
pub fn print_bus_factor_report(bf: &BusFactor, top: usize, since: Option<&str>) {
    print!("{}", render_bus_factor_report(bf, top, since));
}

/// Print bus factor as a single compact line.
pub fn print_bus_factor_short(bf: &BusFactor) {
    let inactive = bf.orphaned_by_inactivity.map_or(String::new(), |pct| {
        format!(" orphaned_by_inactivity:{pct:.1}")
    });
    println!(
        "bus-factor factor:{} threshold:{:.0} files:{} lines:{} contributors:{}{inactive}",
        bf.factor,
        bf.threshold,
        bf.total_files,
        bf.total_lines,
        bf.contributors.len(),
    );
}

/// Print only the bus factor number.
pub fn print_bus_factor_terse(bf: &BusFactor) {
    println!("{}", bf.factor);
}

/// JSON-serializable bus factor output.
pub fn print_bus_factor_json(bf: &BusFactor) -> Result<(), Box<dyn std::error::Error>> {
    report_helpers::print_json_stdout(&bus_factor_json(bf))
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

#[derive(Serialize)]
struct JsonBusFactorEntry {
    author: String,
    files: usize,
    lines: usize,
    pct: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    orphaned_pct: Option<f64>,
    is_critical: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<bool>,
}

#[derive(Serialize)]
struct JsonBusFactor {
    factor: usize,
    threshold: f64,
    author_share: f64,
    total_files: usize,
    total_lines: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    orphaned_by_inactivity: Option<f64>,
    contributors: Vec<JsonBusFactorEntry>,
}

fn bus_factor_json(bf: &BusFactor) -> JsonBusFactor {
    JsonBusFactor {
        factor: bf.factor,
        threshold: bf.threshold,
        author_share: AUTHOR_SHARE,
        total_files: bf.total_files,
        total_lines: bf.total_lines,
        orphaned_by_inactivity: bf.orphaned_by_inactivity.map(round2),
        contributors: bf
            .contributors
            .iter()
            .map(|e| JsonBusFactorEntry {
                author: e.author.clone(),
                files: e.files,
                lines: e.lines,
                pct: round2(e.pct),
                orphaned_pct: e.orphaned_pct.map(round2),
                is_critical: e.is_critical,
                active: e.active,
            })
            .collect(),
    }
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
