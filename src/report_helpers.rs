use std::path::Path;

use serde::Serialize;

/// Compute the max display width for paths, with a minimum of `min`.
pub fn max_path_width<'a>(paths: impl Iterator<Item = &'a Path>, min: usize) -> usize {
    paths
        .map(|p| p.display().to_string().len())
        .max()
        .unwrap_or(min)
        .max(min)
}

/// Print a horizontal separator of box-drawing chars.
pub fn separator(width: usize) -> String {
    "\u{2500}".repeat(width)
}

/// Serialize to pretty JSON and print to stdout.
pub fn print_json_stdout(value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

use crate::cli::OutputMode;

/// Truncate results to `top` and output based on `OutputMode`.
pub fn output_results_mode<T>(
    results: &mut Vec<T>,
    top: usize,
    output: OutputMode,
    print_json_fn: impl FnOnce(&[T]) -> Result<(), Box<dyn std::error::Error>>,
    print_report_fn: impl FnOnce(&[T]),
    print_short_fn: impl FnOnce(&[T]),
    print_terse_fn: impl FnOnce(&[T]),
) -> Result<(), Box<dyn std::error::Error>> {
    results.truncate(top);
    match output {
        OutputMode::Terse => {
            print_terse_fn(results);
            Ok(())
        }
        OutputMode::Short => {
            print_short_fn(results);
            Ok(())
        }
        OutputMode::Json => print_json_fn(results),
        OutputMode::Table => {
            print_report_fn(results);
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "report_helpers_test.rs"]
mod tests;
