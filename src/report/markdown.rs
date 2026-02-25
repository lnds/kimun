//! Markdown report formatter for the combined `km report` command.
//!
//! Generates a single markdown document with seven sections: lines of code
//! (language breakdown), duplication (project-wide stats), indentation
//! complexity, Halstead complexity, cyclomatic complexity, and two MI
//! variants (Visual Studio and verifysoft). Each section includes a
//! description, a markdown table of the top N files sorted by the most
//! relevant metric (worst first), and a note when entries are truncated.
//!
//! Paths and special characters are escaped for correct markdown rendering.

use super::ProjectReport;
use super::data::SectionResult;
use crate::hal::report::format_time;

/// Escape backslashes and pipe characters in file paths so markdown tables
/// render correctly. Backslashes must be escaped first to avoid double-escaping.
fn escape_md(s: &str) -> String {
    s.replace('\\', "\\\\").replace('|', "\\|")
}

/// Format a count as "top N of M" when truncated, or just "N" when
/// all entries are shown.
fn top_of(shown: usize, total: usize) -> String {
    if shown < total {
        format!("top {shown} of {total}")
    } else {
        format!("{total}")
    }
}

/// Print the full project report as a single markdown document.
/// Each metric section includes a description, a markdown table of the
/// top N files, and (when truncated) a note of how many were omitted.
pub fn print_markdown(report: &ProjectReport) {
    println!("# Code Metrics Report");
    println!();
    println!("**Path:** `{}`", report.path);
    println!();
    println!(
        "**Options:** top {} files per section, tests {}, min duplicate block {} lines",
        report.top,
        if report.include_tests {
            "included"
        } else {
            "excluded"
        },
        report.min_lines,
    );

    // --- Lines of Code ---
    // Language breakdown table with per-language counts and a totals row.
    // Languages sorted by code lines descending in the builder.
    println!();
    println!("## Lines of Code");
    println!();
    println!(
        "Physical line counts by language. Each line is classified as blank, comment, or code. \
         Mixed lines (code + comment) count as code. Files are detected by extension or shebang."
    );
    println!();
    if report.loc.is_empty() {
        println!("No recognized source files found.");
    } else {
        println!("| Language | Files | Blank | Comment | Code |");
        println!("|----------|------:|------:|--------:|-----:|");
        let mut total_files = 0usize;
        let mut total_blank = 0usize;
        let mut total_comment = 0usize;
        let mut total_code = 0usize;
        for r in &report.loc {
            println!(
                "| {} | {} | {} | {} | {} |",
                r.name, r.files, r.blank, r.comment, r.code
            );
            total_files += r.files;
            total_blank += r.blank;
            total_comment += r.comment;
            total_code += r.code;
        }
        println!(
            "| **Total** | **{}** | **{}** | **{}** | **{}** |",
            total_files, total_blank, total_comment, total_code
        );
    }

    // --- Duplication ---
    // Project-level stats: dup %, group count, largest block.
    // Calculated after the full walk using cross-file fingerprint matching.
    println!();
    println!("## Code Duplication");
    println!();
    println!("{}", report.duplication.description);
    println!();
    let d = &report.duplication;
    println!("| Metric | Value |");
    println!("|--------|------:|");
    println!("| Total code lines | {} |", d.total_code_lines);
    println!(
        "| Duplicated lines | {} ({:.1}%) |",
        d.duplicated_lines, d.duplication_percentage
    );
    println!("| Duplicate groups | {} |", d.duplicate_groups);
    println!("| Files with duplicates | {} |", d.files_with_duplicates);
    println!("| Largest block | {} lines |", d.largest_block);

    // --- Indentation ---
    // Worst first: highest stddev indicates deeply nested control flow.
    let indent = &report.indent;
    println!();
    println!(
        "## Indentation Complexity ({}, by stddev desc)",
        top_of(indent.entries.len(), indent.total_count)
    );
    println!();
    println!("{}", indent.description);
    println!();
    if indent.entries.is_empty() {
        println!("No data.");
    } else {
        println!("| File | Lines | StdDev | Max Depth | Level |");
        println!("|------|------:|-------:|----------:|-------|");
        for f in &indent.entries {
            println!(
                "| {} | {} | {:.2} | {} | {} |",
                escape_md(&f.path),
                f.code_lines,
                f.stddev,
                f.max_depth,
                f.complexity
            );
        }
    }

    // --- Halstead ---
    // Worst first: highest effort indicates most mental cost to understand.
    let hal = &report.halstead;
    println!();
    println!(
        "## Halstead Complexity ({}, by effort desc)",
        top_of(hal.entries.len(), hal.total_count)
    );
    println!();
    println!("{}", hal.description);
    println!();
    if hal.entries.is_empty() {
        println!("No data.");
    } else {
        println!("| File | Volume | Effort | Bugs | Time |");
        println!("|------|-------:|-------:|-----:|-----:|");
        for f in &hal.entries {
            println!(
                "| {} | {:.1} | {:.0} | {:.2} | {} |",
                escape_md(&f.path),
                f.volume,
                f.effort,
                f.bugs,
                format_time(f.time),
            );
        }
    }

    // --- Cyclomatic ---
    // Worst first: highest total complexity indicates most decision paths.
    let cycom = &report.cyclomatic;
    println!();
    println!(
        "## Cyclomatic Complexity ({}, by total desc)",
        top_of(cycom.entries.len(), cycom.total_count)
    );
    println!();
    println!("{}", cycom.description);
    println!();
    if cycom.entries.is_empty() {
        println!("No data.");
    } else {
        println!("| File | Functions | Total | Max | Avg | Level |");
        println!("|------|----------:|------:|----:|----:|-------|");
        for f in &cycom.entries {
            println!(
                "| {} | {} | {} | {} | {:.1} | {} |",
                escape_md(&f.path),
                f.functions,
                f.total,
                f.max,
                f.avg,
                f.level
            );
        }
    }

    // --- MI sections ---
    // Both variants share the same File | MI | Level table format via
    // the generic print_mi_section() helper.
    print_mi_section("Visual Studio", &report.mi_visual_studio, |e| {
        (&e.path, e.mi_score, &e.level)
    });
    print_mi_section("Verifysoft", &report.mi_verifysoft, |e| {
        (&e.path, e.mi_score, &e.level)
    });
}

/// Print a Maintainability Index section (File | MI | Level table).
/// Used for both the Visual Studio and verifysoft MI variants.
/// The `fields` closure extracts (path, score, level) from the generic entry type.
fn print_mi_section<T>(
    variant: &str,
    section: &SectionResult<T>,
    fields: impl Fn(&T) -> (&str, f64, &str),
) {
    println!();
    println!(
        "## Maintainability Index \u{2014} {} ({}, by MI asc)",
        variant,
        top_of(section.entries.len(), section.total_count)
    );
    println!();
    println!("{}", section.description);
    println!();
    if section.entries.is_empty() {
        println!("No data.");
    } else {
        println!("| File | MI | Level |");
        println!("|------|---:|-------|");
        for entry in &section.entries {
            let (path, mi_score, level) = fields(entry);
            println!("| {} | {:.1} | {} |", escape_md(path), mi_score, level);
        }
    }
}

#[cfg(test)]
#[path = "markdown_test.rs"]
mod tests;
