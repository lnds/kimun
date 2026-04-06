use std::path::Path;

use serde::Serialize;
use unicode_width::UnicodeWidthStr;

/// A single function row for per-function complexity breakdown reports.
pub trait PerFunctionRow {
    fn name(&self) -> &str;
    fn complexity(&self) -> usize;
    fn level_str(&self) -> &str;
}

/// A file entry that carries a path and a list of per-function rows.
pub trait PerFunctionFile {
    type Row: PerFunctionRow;
    fn path_str(&self) -> String;
    fn rows(&self) -> &[Self::Row];
}

/// Print a per-function complexity breakdown grouped by file.
///
/// `title` is printed as the report header (e.g. "Cyclomatic Complexity (per function)").
pub fn print_per_function_breakdown<F: PerFunctionFile>(title: &str, files: &[F]) {
    if files.is_empty() {
        println!("No recognized source files found.");
        return;
    }

    let sep = separator(78);
    println!("{title}");
    println!("{sep}");

    for f in files {
        println!();
        println!("{}:", f.path_str());

        let rows = f.rows();
        let max_name_len = rows
            .iter()
            .map(|r| display_width(r.name()))
            .max()
            .unwrap_or(10)
            .max(10);

        for r in rows {
            println!(
                "  {}  {:>5}  {}",
                pad_to(r.name(), max_name_len),
                r.complexity(),
                r.level_str(),
            );
        }
    }

    println!("{sep}");
}

/// Returns the display width of `s` in terminal columns.
///
/// Uses `unicode-width` so CJK and accented characters are measured correctly
/// rather than counting raw bytes or Unicode scalar values.
pub fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Left-pad `s` to `width` terminal display columns.
///
/// Rust's built-in `{:<N}` counts Unicode scalar values, not display columns.
/// This helper pads by display width so accented and CJK characters align.
pub fn pad_to(s: &str, width: usize) -> String {
    let padding = width.saturating_sub(UnicodeWidthStr::width(s));
    format!("{s}{}", " ".repeat(padding))
}

/// Emit a GitHub Actions workflow annotation.
///
/// `level` is one of `"warning"`, `"error"`, or `"notice"`.
/// `file` should be a path relative to the repository root.
/// Produces output like: `::warning file=src/foo.rs,line=42,title=Long Function::fn process is 80 lines`
pub fn github_annotation(level: &str, file: &str, line: usize, title: &str, message: &str) {
    println!("::{level} file={file},line={line},title={title}::{message}");
}

/// Compute the max display width for paths, with a minimum of `min`.
pub fn max_path_width<'a>(paths: impl Iterator<Item = &'a Path>, min: usize) -> usize {
    paths
        .map(|p| display_width(&p.display().to_string()))
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
