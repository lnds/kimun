/// Markdown report formatter for the combined `cm report` command.
///
/// Generates a single markdown document with sections for lines of code,
/// duplication, indentation, Halstead, cyclomatic complexity, and
/// maintainability index (both Visual Studio and verifysoft variants).
use super::ProjectReport;
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

    // --- Indentation (sorted by stddev descending) ---
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

    // --- Halstead (sorted by effort descending) ---
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

    // --- Cyclomatic (sorted by total descending) ---
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

    // --- Maintainability Index: Visual Studio (sorted by MI ascending, worst first) ---
    let mi_vs = &report.mi_visual_studio;
    println!();
    println!(
        "## Maintainability Index \u{2014} Visual Studio ({}, by MI asc)",
        top_of(mi_vs.entries.len(), mi_vs.total_count)
    );
    println!();
    println!("{}", mi_vs.description);
    println!();
    if mi_vs.entries.is_empty() {
        println!("No data.");
    } else {
        println!("| File | MI | Level |");
        println!("|------|---:|-------|");
        for f in &mi_vs.entries {
            println!(
                "| {} | {:.1} | {} |",
                escape_md(&f.path),
                f.mi_score,
                f.level
            );
        }
    }

    // --- Maintainability Index: Verifysoft (sorted by MI ascending, worst first) ---
    let mi_vf = &report.mi_verifysoft;
    println!();
    println!(
        "## Maintainability Index \u{2014} Verifysoft ({}, by MI asc)",
        top_of(mi_vf.entries.len(), mi_vf.total_count)
    );
    println!();
    println!("{}", mi_vf.description);
    println!();
    if mi_vf.entries.is_empty() {
        println!("No data.");
    } else {
        println!("| File | MI | Level |");
        println!("|------|---:|-------|");
        for f in &mi_vf.entries {
            println!(
                "| {} | {:.1} | {} |",
                escape_md(&f.path),
                f.mi_score,
                f.level
            );
        }
    }
}

#[cfg(test)]
#[path = "markdown_test.rs"]
mod tests;
