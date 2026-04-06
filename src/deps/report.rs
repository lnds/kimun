/// Report formatters for dependency graph analysis.
///
/// Provides table and JSON output showing per-file fan-in, fan-out,
/// coupling classification, and cycle membership. Cycles are printed
/// separately after the main table.
use crate::report_helpers;

use super::analyzer::{DepEntry, DepResult, JsonDepResult};

const COL_LANG: usize = 10;
const COL_FAN_IN: usize = 6;
const COL_FAN_OUT: usize = 7;
const COL_CYCLE: usize = 5;
// spacing: 1 (lead) + 2 + 1 + 1 + 1 + 1 = 7
const FIXED_WIDTH: usize = 7 + COL_LANG + COL_FAN_IN + COL_FAN_OUT + COL_CYCLE;

/// Print a table of per-file dependency metrics and a cycle summary.
pub fn print_report(entries: &[DepEntry], result: &DepResult) {
    if entries.is_empty() {
        println!("No source files found for dependency analysis.");
        return;
    }

    let max_path = report_helpers::max_path_width(entries.iter().map(|e| e.path.as_path()), 4);
    let header_width = max_path + FIXED_WIDTH;
    let sep = report_helpers::separator(header_width.max(72));

    println!("Dependency Graph");
    println!("{sep}");
    println!(
        " {:<pw$}  {:>COL_LANG$} {:>COL_FAN_IN$} {:>COL_FAN_OUT$} {:>COL_CYCLE$}",
        "File",
        "Language",
        "Fan-In",
        "Fan-Out",
        "Cycle",
        pw = max_path,
    );
    println!("{sep}");

    for e in entries {
        println!(
            " {:<pw$}  {:>COL_LANG$} {:>COL_FAN_IN$} {:>COL_FAN_OUT$} {:>COL_CYCLE$}",
            e.path.display(),
            e.language,
            e.fan_in,
            e.fan_out,
            if e.in_cycle { "yes" } else { "no" },
            pw = max_path,
        );
    }

    println!("{sep}");

    if result.cycles.is_empty() {
        println!("No dependency cycles detected.");
    } else {
        println!();
        println!("Dependency cycles: {}", result.cycles.len());
        for (i, cycle) in result.cycles.iter().enumerate() {
            println!("  Cycle {} ({} files):", i + 1, cycle.len());
            for p in cycle {
                println!("    {}", p.display());
            }
        }
    }
}

/// Serialize dependency analysis as pretty-printed JSON to stdout.
pub fn print_json(result: &DepResult) -> Result<(), Box<dyn std::error::Error>> {
    let out = JsonDepResult::from(result);
    report_helpers::print_json_stdout(&out)
}
