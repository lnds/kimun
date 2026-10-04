/// Report formatters for dependency graph analysis.
///
/// Provides table and JSON output showing per-file fan-in, fan-out,
/// coupling classification, and cycle membership. Cycles are printed
/// separately after the main table.
use std::path::PathBuf;

use crate::report_helpers;

use super::analyzer::{DepEntry, DepResult, JsonDepResult, UnsupportedLanguage};

const COL_LANG: usize = 10;
const COL_FAN_IN: usize = 6;
const COL_FAN_OUT: usize = 7;
const COL_CYCLE: usize = 5;
// spacing: 1 (lead) + 2 + 1 + 1 + 1 + 1 = 7
const FIXED_WIDTH: usize = 7 + COL_LANG + COL_FAN_IN + COL_FAN_OUT + COL_CYCLE;

/// Print a table of per-file dependency metrics and a cycle summary.
pub fn print_report(entries: &[DepEntry], result: &DepResult) {
    print!("{}", render_report(entries, result));
}

/// Render the table of per-file dependency metrics and the cycle summary.
fn render_report(entries: &[DepEntry], result: &DepResult) -> String {
    let mut lines = if entries.is_empty() {
        vec!["No source files found for dependency analysis.".to_string()]
    } else {
        let mut lines = table_lines(entries);
        lines.extend(cycle_lines(&result.cycles));
        lines
    };
    // Name the languages left out, so that their absence from the table is
    // not read as "no dependencies".
    lines.extend(unsupported_note(&result.unsupported));
    lines.join("\n") + "\n"
}

/// One table row, with the path column padded to `path_width`.
fn row(path_width: usize, cells: [&dyn std::fmt::Display; 5]) -> String {
    let [path, language, fan_in, fan_out, cycle] = cells;
    format!(
        " {path:<path_width$}  {language:>COL_LANG$} {fan_in:>COL_FAN_IN$} {fan_out:>COL_FAN_OUT$} {cycle:>COL_CYCLE$}"
    )
}

/// The title, the header and one row per entry, framed by separators.
fn table_lines(entries: &[DepEntry]) -> Vec<String> {
    let max_path = report_helpers::max_path_width(entries.iter().map(|e| e.path.as_path()), 4);
    let sep = report_helpers::separator((max_path + FIXED_WIDTH).max(72));

    let mut lines = vec![
        "Dependency Graph".to_string(),
        sep.clone(),
        row(
            max_path,
            [&"File", &"Language", &"Fan-In", &"Fan-Out", &"Cycle"],
        ),
        sep.clone(),
    ];
    lines.extend(entries.iter().map(|e| {
        let cycle = if e.in_cycle { "yes" } else { "no" };
        row(
            max_path,
            [
                &e.path.display(),
                &e.language,
                &e.fan_in,
                &e.fan_out,
                &cycle,
            ],
        )
    }));
    lines.push(sep);
    lines
}

/// The cycle summary that follows the table.
fn cycle_lines(cycles: &[Vec<PathBuf>]) -> Vec<String> {
    if cycles.is_empty() {
        return vec!["No dependency cycles detected.".to_string()];
    }
    let mut lines = vec![
        String::new(),
        format!("Dependency cycles: {}", cycles.len()),
    ];
    for (i, cycle) in cycles.iter().enumerate() {
        lines.push(format!("  Cycle {} ({} files):", i + 1, cycle.len()));
        lines.extend(cycle.iter().map(|p| format!("    {}", p.display())));
    }
    lines
}

fn unsupported_note(unsupported: &[UnsupportedLanguage]) -> Option<String> {
    if unsupported.is_empty() {
        return None;
    }
    let languages: Vec<String> = unsupported
        .iter()
        .map(|u| format!("{} {}", u.language, u.files))
        .collect();
    Some(format!(
        "Not analysed (unsupported language): {}",
        languages.join(", ")
    ))
}

/// Serialize dependency analysis as pretty-printed JSON to stdout.
pub fn print_json(result: &DepResult) -> Result<(), Box<dyn std::error::Error>> {
    let out = JsonDepResult::from(result);
    report_helpers::print_json_stdout(&out)
}

/// Print dependency analysis as a single compact line.
pub fn print_short(result: &DepResult) {
    println!(
        "deps files:{} cycles:{} max_fan_out:{} max_fan_in:{} unsupported:{}",
        result.entries.len(),
        result.cycles.len(),
        result.entries.iter().map(|e| e.fan_out).max().unwrap_or(0),
        result.entries.iter().map(|e| e.fan_in).max().unwrap_or(0),
        result.unsupported.iter().map(|u| u.files).sum::<usize>(),
    );
}

/// Print only the cycle count.
pub fn print_terse(result: &DepResult) {
    println!("{}", result.cycles.len());
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
