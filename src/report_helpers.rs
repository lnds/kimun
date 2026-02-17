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

/// Truncate results to `top` and output as JSON or table.
pub fn output_results<T>(
    results: &mut Vec<T>,
    top: usize,
    json: bool,
    print_json_fn: impl FnOnce(&[T]) -> Result<(), Box<dyn std::error::Error>>,
    print_report_fn: impl FnOnce(&[T]),
) -> Result<(), Box<dyn std::error::Error>> {
    results.truncate(top);
    if json {
        print_json_fn(results)
    } else {
        print_report_fn(results);
        Ok(())
    }
}

#[cfg(test)]
#[path = "report_helpers_test.rs"]
mod tests;
