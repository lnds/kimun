/// Report formatters for knowledge map (code ownership) analysis.
///
/// Provides table and JSON output showing per-file primary owner,
/// ownership concentration, contributor count, and knowledge loss risk.
use serde::Serialize;

use super::analyzer::{AuthorSummary, BusFactor, FileOwnership};
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

/// Print the bus factor report as a human-readable table.
pub fn print_bus_factor_report(bf: &BusFactor) {
    if bf.total_lines == 0 {
        println!("No blame data found for bus factor analysis.");
        return;
    }

    let risk_label = match bf.factor {
        0 => "no data",
        1 => "CRITICAL — one person holds most project knowledge",
        2 => "HIGH — two people hold critical knowledge",
        3 => "MODERATE — three people hold critical knowledge",
        _ => "LOW — knowledge is distributed across several contributors",
    };

    println!("Project Bus Factor: {}", bf.factor);
    println!();
    println!(
        " Losing {} key {} would put {:.0}% of the project's knowledge at risk.",
        bf.factor,
        if bf.factor == 1 {
            "contributor"
        } else {
            "contributors"
        },
        bf.threshold,
    );
    println!(" Risk: {risk_label}");
    println!();

    let max_author_len = bf
        .contributors
        .iter()
        .map(|e| report_helpers::display_width(&e.author))
        .max()
        .unwrap_or(6)
        .max(6);

    let header_width = 6 + max_author_len + 8 + 8 + 11 + 8;
    let separator = report_helpers::separator(header_width.max(70));

    println!("{separator}");
    println!(
        " {:>4}  {:<aw$}  {:>8}  {:>7}  {:>10}",
        "Rank",
        "Author",
        "Lines",
        "Share",
        "Cumulative",
        aw = max_author_len,
    );
    println!("{separator}");

    for (i, entry) in bf.contributors.iter().enumerate() {
        let marker = if entry.cumulative_pct >= bf.threshold
            && (i == 0 || bf.contributors[i - 1].cumulative_pct < bf.threshold)
        {
            format!("  ← {:.0}% threshold", bf.threshold)
        } else {
            String::new()
        };

        println!(
            " {:>4}  {:<aw$}  {:>8}  {:>6.2}%  {:>9.2}%{}",
            i + 1,
            entry.author,
            entry.lines,
            entry.pct,
            entry.cumulative_pct,
            marker,
            aw = max_author_len,
        );
    }

    println!("{separator}");
}

/// Print bus factor as a single compact line.
pub fn print_bus_factor_short(bf: &BusFactor) {
    println!(
        "bus-factor factor:{} contributors:{} lines:{}",
        bf.factor,
        bf.contributors.len(),
        bf.total_lines
    );
}

/// Print only the bus factor number.
pub fn print_bus_factor_terse(bf: &BusFactor) {
    println!("{}", bf.factor);
}

/// JSON-serializable bus factor output.
pub fn print_bus_factor_json(bf: &BusFactor) -> Result<(), Box<dyn std::error::Error>> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct JsonBusFactorEntry {
        author: String,
        lines: usize,
        pct: f64,
        cumulative_pct: f64,
        is_critical: bool,
    }

    #[derive(Serialize)]
    struct JsonBusFactor {
        factor: usize,
        threshold: f64,
        total_lines: usize,
        contributors: Vec<JsonBusFactorEntry>,
    }

    let out = JsonBusFactor {
        factor: bf.factor,
        threshold: bf.threshold,
        total_lines: bf.total_lines,
        contributors: bf
            .contributors
            .iter()
            .map(|e| JsonBusFactorEntry {
                author: e.author.clone(),
                lines: e.lines,
                pct: (e.pct * 100.0).round() / 100.0,
                cumulative_pct: (e.cumulative_pct * 100.0).round() / 100.0,
                is_critical: e.is_critical,
            })
            .collect(),
    };

    report_helpers::print_json_stdout(&out)
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
