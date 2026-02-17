/// Report formatters for temporal coupling analysis.
///
/// Displays file pairs that frequently change together in commits,
/// ranked by coupling strength (shared_commits / min(commits_a, commits_b)).
use serde::Serialize;

use super::analyzer::FileCoupling;
use crate::report_helpers;

/// Print a table of temporally coupled file pairs with strength and level.
pub fn print_report(pairs: &[FileCoupling], total: usize) {
    if pairs.is_empty() {
        println!("No coupled file pairs found.");
        return;
    }

    let max_a_len = report_helpers::max_path_width(pairs.iter().map(|p| p.file_a.as_path()), 6);
    let max_b_len = report_helpers::max_path_width(pairs.iter().map(|p| p.file_b.as_path()), 6);
    let header_width = max_a_len + max_b_len + 35;
    let separator = report_helpers::separator(header_width.max(78));

    println!("Temporal Coupling — Files That Change Together");
    println!("{separator}");
    println!(
        " {:<aw$}  {:<bw$}  {:>6}  {:>8}  {:>8}",
        "File A",
        "File B",
        "Shared",
        "Strength",
        "Level",
        aw = max_a_len,
        bw = max_b_len,
    );
    println!("{separator}");

    for p in pairs {
        println!(
            " {:<aw$}  {:<bw$}  {:>6}  {:>8.2}  {:>8}",
            p.file_a.display(),
            p.file_b.display(),
            p.shared_commits,
            p.strength,
            p.level.label(),
            aw = max_a_len,
            bw = max_b_len,
        );
    }

    println!("{separator}");
    if total > pairs.len() {
        println!();
        println!(
            "{total} coupled pairs found ({shown} shown).",
            shown = pairs.len()
        );
    }
}

/// JSON-serializable representation of a coupled file pair.
#[derive(Serialize)]
struct JsonEntry {
    file_a: String,
    file_b: String,
    shared_commits: usize,
    commits_a: usize,
    commits_b: usize,
    strength: f64,
    level: String,
}

/// Serialize coupled file pairs as pretty-printed JSON to stdout.
pub fn print_json(pairs: &[FileCoupling]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<JsonEntry> = pairs
        .iter()
        .map(|p| JsonEntry {
            file_a: p.file_a.display().to_string(),
            file_b: p.file_b.display().to_string(),
            shared_commits: p.shared_commits,
            commits_a: p.commits_a,
            commits_b: p.commits_b,
            strength: (p.strength * 100.0).round() / 100.0,
            level: p.level.label().to_string(),
        })
        .collect();

    report_helpers::print_json_stdout(&entries)
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
