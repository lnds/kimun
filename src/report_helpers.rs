use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Serialize;
use unicode_width::UnicodeWidthStr;

/// A single function row for per-function complexity breakdown reports.
pub trait PerFunctionRow {
    fn name(&self) -> &str;
    fn start_line(&self) -> usize;
    fn complexity(&self) -> usize;
    fn level_str(&self) -> &str;
}

/// A file entry that carries a path and a list of per-function rows.
pub trait PerFunctionFile {
    type Row: PerFunctionRow;
    fn path_str(&self) -> String;
    fn rows(&self) -> &[Self::Row];
}

fn rows_at_least<R: PerFunctionRow>(rows: &[R], min_complexity: usize) -> Vec<&R> {
    rows.iter()
        .filter(|r| r.complexity() >= min_complexity)
        .collect()
}

/// Print a per-function complexity breakdown grouped by file, listing only
/// functions whose complexity is at least `min_complexity`.
///
/// `title` is printed as the report header (e.g. "Cyclomatic Complexity (per function)").
pub fn print_per_function_breakdown<F: PerFunctionFile>(
    title: &str,
    files: &[F],
    min_complexity: usize,
) {
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

        let rows = rows_at_least(f.rows(), min_complexity);
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

/// Build one CodeClimate JSON entry (GitLab Code Quality format).
///
/// `severity` is one of `"info"`, `"minor"`, `"major"`, `"critical"`, or `"blocker"`.
/// `fingerprint` is derived from `"file:line:title"` via FNV-1a, stable across runs
/// and unique per finding without requiring an external hash dependency.
pub fn codeclimate_entry(
    severity: &str,
    file: &str,
    line: usize,
    title: &str,
    description: &str,
) -> serde_json::Value {
    serde_json::json!({
        "description": description,
        "fingerprint": fnv1a_hex(&format!("{file}:{line}:{title}")),
        "severity": severity,
        "location": { "path": file, "lines": { "begin": line } }
    })
}

/// FNV-1a 64-bit hash, hex-encoded. Used for CodeClimate fingerprints.
fn fnv1a_hex(s: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Map a raw complexity score to a CodeClimate severity string.
///
/// Thresholds match the SonarQube/Clippy conventions used by cogcom and cycom:
/// `< 10 → "info"`, `10-14 → "minor"`, `15-24 → "major"`, `≥ 25 → "critical"`.
pub fn complexity_severity(c: usize) -> &'static str {
    match c {
        25.. => "critical",
        15..=24 => "major",
        10..=14 => "minor",
        _ => "info",
    }
}

/// Collect CodeClimate entries for one file's function rows into `out`.
fn append_codeclimate_entries<R: PerFunctionRow>(
    path: &str,
    rows: &[R],
    min_complexity: usize,
    title: &str,
    kind: &str,
    out: &mut Vec<serde_json::Value>,
) {
    for func in rows {
        if func.complexity() < min_complexity {
            continue;
        }
        let description = format!(
            "function '{}' has {kind} complexity {} (threshold: {min_complexity})",
            func.name(),
            func.complexity(),
        );
        out.push(codeclimate_entry(
            complexity_severity(func.complexity()),
            path,
            func.start_line(),
            title,
            &description,
        ));
    }
}

/// Emit a CodeClimate JSON array for functions whose complexity meets the threshold.
///
/// Generic over any `PerFunctionFile` so cogcom and cycom share one implementation.
/// `title` is the CodeClimate issue title (e.g. `"Cognitive Complexity"`);
/// `kind` is the lowercase word used in the description (e.g. `"cognitive"`).
pub fn print_codeclimate_complexity<F>(
    files: &[F],
    min_complexity: usize,
    title: &str,
    kind: &str,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: PerFunctionFile,
{
    let mut entries = Vec::new();
    for f in files {
        append_codeclimate_entries(
            &f.path_str(),
            f.rows(),
            min_complexity,
            title,
            kind,
            &mut entries,
        );
    }
    print_json_stdout(&entries)
}

/// Compute the max display width for paths, with a minimum of `min`.
pub fn max_path_width<'a>(paths: impl Iterator<Item = &'a Path>, min: usize) -> usize {
    paths
        .map(|p| display_width(&shown(p)))
        .max()
        .unwrap_or(min)
        .max(min)
}

/// The width of the path column of a table that shows the first `shown`
/// of `paths`.
pub fn path_column<'a>(paths: impl Iterator<Item = &'a Path>, shown: usize) -> usize {
    max_path_width(paths.take(shown), 4)
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

/// Where reports show paths from (`set_path_base`).
static PATH_BASE: OnceLock<PathBuf> = OnceLock::new();

/// Set where reports show paths from, for the command about to run on
/// `path`: the root of the git repository that holds it, or `path` itself
/// outside one. Then every command shows a file the same way, wherever it
/// is run from and however the path is written.
pub fn set_path_base(path: &Path) {
    let repo_root = git2::Repository::discover(path)
        .ok()
        .and_then(|repo| repo.workdir().map(Path::to_path_buf));
    if let Ok(base) = repo_root
        .unwrap_or_else(|| path.to_path_buf())
        .canonicalize()
    {
        let _ = PATH_BASE.set(base);
    }
}

/// A path as reports show it: relative to the path base, without a leading
/// `./`. A path that cannot be resolved from here, such as a path of the
/// repository read from history, is shown as it is.
pub fn shown(path: &Path) -> String {
    let relative = PATH_BASE.get().and_then(|base| {
        let full = path.canonicalize().ok()?;
        full.strip_prefix(base).ok().map(Path::to_path_buf)
    });
    let text = relative.as_deref().unwrap_or(path).display().to_string();
    match text.strip_prefix("./") {
        Some(rest) => rest.to_string(),
        None => text,
    }
}

/// Rows a table shows when `--top` is not given.
pub const DEFAULT_TOP: usize = 20;

/// How many of `total` entries an output lists: a table `--top` rows
/// (`DEFAULT_TOP` when not given), JSON every entry unless `--top` is given,
/// and the summaries (`short`, `terse`) and CI annotations every entry.
pub fn shown_count(output: crate::cli::OutputMode, top: Option<usize>, total: usize) -> usize {
    use crate::cli::OutputMode;
    let limit = match output {
        OutputMode::Table => top.unwrap_or(DEFAULT_TOP),
        OutputMode::Json => top.unwrap_or(usize::MAX),
        _ => usize::MAX,
    };
    limit.min(total)
}

/// The label of a totals row computed over `total` files of which a table
/// shows `shown`.
pub fn total_label(total: usize, shown: usize) -> String {
    if shown < total {
        format!(" Total ({total} files, {shown} shown)")
    } else {
        format!(" Total ({total} files)")
    }
}

/// Print `results` in `output`: JSON lists the entries `shown_count`
/// allows; a table gets every entry, to total them, and how many to show;
/// the summaries get every entry.
pub fn output_results<T>(
    results: &[T],
    top: Option<usize>,
    output: crate::cli::OutputMode,
    print_json_fn: impl FnOnce(&[T]) -> Result<(), Box<dyn std::error::Error>>,
    print_report_fn: impl FnOnce(&[T], usize),
    print_short_fn: impl FnOnce(&[T]),
    print_terse_fn: impl FnOnce(&[T]),
) -> Result<(), Box<dyn std::error::Error>> {
    let shown = shown_count(output, top, results.len());
    match output {
        crate::cli::OutputMode::Terse => {
            print_terse_fn(results);
            Ok(())
        }
        crate::cli::OutputMode::Short => {
            print_short_fn(results);
            Ok(())
        }
        crate::cli::OutputMode::Json => print_json_fn(&results[..shown]),
        crate::cli::OutputMode::Github | crate::cli::OutputMode::Codeclimate => {
            Err(crate::cli::ERR_CI_FORMAT_ONLY.into())
        }
        crate::cli::OutputMode::Table => {
            print_report_fn(results, shown);
            Ok(())
        }
    }
}

/// The Maintainability Index of a set of files: the mean of each file's
/// MI weighted by its code lines, with values below 0 taken as 0, as the
/// legacy model of `km score` takes them. A plain mean would let a few
/// small files outweigh the bulk of the code, and an unbounded negative MI
/// outweigh every other file.
pub fn weighted_mi(files: impl Iterator<Item = (f64, usize)>) -> f64 {
    let (sum, lines) = files.fold((0.0, 0usize), |(sum, lines), (mi, loc)| {
        let loc = loc.max(1);
        (sum + mi.max(0.0) * loc as f64, lines + loc)
    });
    if lines == 0 { 0.0 } else { sum / lines as f64 }
}

#[cfg(test)]
#[path = "report_helpers_test.rs"]
mod tests;
