use super::ProjectReport;

/// Escape backslashes and pipe characters in file paths so markdown tables
/// render correctly. Backslashes must be escaped first to avoid double-escaping.
fn escape_md(s: &str) -> String {
    s.replace('\\', "\\\\").replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_md_no_special_chars() {
        assert_eq!(escape_md("src/main.rs"), "src/main.rs");
    }

    #[test]
    fn escape_md_pipe() {
        assert_eq!(escape_md("foo|bar.rs"), "foo\\|bar.rs");
    }

    #[test]
    fn escape_md_backslash_and_pipe() {
        assert_eq!(escape_md("path\\|file.rs"), "path\\\\\\|file.rs");
    }

    #[test]
    fn top_of_truncated() {
        assert_eq!(top_of(5, 20), "top 5 of 20");
    }

    #[test]
    fn top_of_not_truncated() {
        assert_eq!(top_of(3, 3), "3");
    }

    #[test]
    fn top_of_zero() {
        assert_eq!(top_of(0, 0), "0");
    }
}

/// Format "top N of M" or just "N" when not truncated.
fn top_of(shown: usize, total: usize) -> String {
    if shown < total {
        format!("top {shown} of {total}")
    } else {
        format!("{total}")
    }
}

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
    if hal.entries.is_empty() {
        println!("No data.");
    } else {
        println!("| File | Volume | Effort | Bugs |");
        println!("|------|-------:|-------:|-----:|");
        for f in &hal.entries {
            println!(
                "| {} | {:.1} | {:.0} | {:.2} |",
                escape_md(&f.path),
                f.volume,
                f.effort,
                f.bugs
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
        "## Maintainability Index — Visual Studio ({}, by MI asc)",
        top_of(mi_vs.entries.len(), mi_vs.total_count)
    );
    println!();
    println!(
        "Normalized 0\u{2013}100 scale, no comment weight. Thresholds: green (20+), yellow (10\u{2013}19), red (0\u{2013}9)."
    );
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
        "## Maintainability Index — Verifysoft ({}, by MI asc)",
        top_of(mi_vf.entries.len(), mi_vf.total_count)
    );
    println!();
    println!(
        "Unbounded scale with comment weight. Thresholds: good (85+), moderate (65\u{2013}84), difficult (<65)."
    );
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
