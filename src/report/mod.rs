mod json;
mod markdown;

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::Serialize;

use crate::cycom;
use crate::dups;
use crate::hal;
use crate::indent;
use crate::loc::counter::{FileStats, LineKind, classify_reader, count_lines};
use crate::loc::language::detect;
use crate::loc::report::LanguageReport;
use crate::mi;
use crate::miv;
use crate::util::is_binary_reader;
use crate::walk;

/// Comprehensive project report combining all code metrics.
///
/// When serialized to JSON the structure is:
/// ```json
/// {
///   "path": "...",
///   "top": 20,
///   "include_tests": false,
///   "min_lines": 6,
///   "loc": [{ "name": "Rust", "files": 5, "blank": 20, "comment": 10, "code": 500 }],
///   "duplication": { "total_code_lines": ..., "duplicated_lines": ..., ... },
///   "indent": [{ "path": "...", "code_lines": ..., "stddev": ..., ... }],
///   "halstead": [{ "path": "...", "volume": ..., "effort": ..., "bugs": ... }],
///   "cyclomatic": [{ "path": "...", "functions": ..., "total": ..., ... }],
///   "mi_visual_studio": [{ "path": "...", "mi_score": ..., "level": "..." }],
///   "mi_verifysoft": [{ "path": "...", "mi_score": ..., "level": "..." }]
/// }
/// ```
///
/// Each per-file section includes `total_count` (total files before truncation)
/// and up to `top` entries sorted by the relevant metric.
#[derive(Debug, Serialize)]
pub struct ProjectReport {
    pub path: String,
    pub top: usize,
    pub include_tests: bool,
    pub min_lines: usize,
    pub loc: Vec<LanguageReport>,
    pub duplication: DupsSummary,
    pub indent: SectionResult<IndentEntry>,
    pub halstead: SectionResult<HalsteadEntry>,
    pub cyclomatic: SectionResult<CycomEntry>,
    pub mi_visual_studio: SectionResult<MiVisualStudioEntry>,
    pub mi_verifysoft: SectionResult<MiVerifysoftEntry>,
}

/// A section of per-file results with total count before truncation.
#[derive(Debug, Serialize)]
pub struct SectionResult<T> {
    pub total_count: usize,
    pub entries: Vec<T>,
}

#[derive(Debug, Serialize)]
pub struct DupsSummary {
    pub total_code_lines: usize,
    pub duplicated_lines: usize,
    pub duplication_percentage: f64,
    pub duplicate_groups: usize,
    pub files_with_duplicates: usize,
    pub largest_block: usize,
}

#[derive(Debug, Serialize)]
pub struct IndentEntry {
    pub path: String,
    pub code_lines: usize,
    pub stddev: f64,
    pub max_depth: usize,
    pub complexity: String,
}

#[derive(Debug, Serialize)]
pub struct HalsteadEntry {
    pub path: String,
    pub volume: f64,
    pub effort: f64,
    pub bugs: f64,
}

#[derive(Debug, Serialize)]
pub struct CycomEntry {
    pub path: String,
    pub functions: usize,
    pub total: usize,
    pub max: usize,
    pub avg: f64,
    pub level: String,
}

#[derive(Debug, Serialize)]
pub struct MiVisualStudioEntry {
    pub path: String,
    pub mi_score: f64,
    pub level: String,
}

#[derive(Debug, Serialize)]
pub struct MiVerifysoftEntry {
    pub path: String,
    pub mi_score: f64,
    pub level: String,
}

pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    top: usize,
    min_lines: usize,
) -> Result<(), Box<dyn Error>> {
    let report = build_report(path, include_tests, top, min_lines)?;

    if json {
        json::print_json(&report)?;
    } else {
        markdown::print_markdown(&report);
    }

    Ok(())
}

/// Build the full project report by walking the file tree once and running
/// all analyzers per file. Returns the report struct for output or testing.
pub fn build_report(
    path: &Path,
    include_tests: bool,
    top: usize,
    min_lines: usize,
) -> Result<ProjectReport, Box<dyn Error>> {
    // LOC aggregation (no dedup — every file counts; the duplication section
    // surfaces duplicates explicitly)
    let mut stats_by_lang: HashMap<&'static str, (usize, FileStats)> = HashMap::new();

    // Dups collection
    let mut dup_files: Vec<dups::detector::NormalizedFile> = Vec::new();
    let mut total_code_lines: usize = 0;

    // Per-file metrics
    let mut indent_results: Vec<IndentEntry> = Vec::new();
    let mut hal_results: Vec<HalsteadEntry> = Vec::new();
    let mut cycom_results: Vec<CycomEntry> = Vec::new();
    let mut mi_vs_results: Vec<MiVisualStudioEntry> = Vec::new();
    let mut mi_vf_results: Vec<MiVerifysoftEntry> = Vec::new();

    for entry in walk::walk(path, !include_tests) {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                eprintln!("warning: {err}");
                continue;
            }
        };

        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let file_path = entry.path();

        // walk::walk filters test directories, but individual test files
        // (e.g. foo_test.rs in a non-test directory) need a filename check.
        if !include_tests && walk::is_test_file(file_path) {
            continue;
        }

        let spec = match detect(file_path) {
            Some(s) => s,
            None => match walk::try_detect_shebang(file_path) {
                Some(s) => s,
                None => continue,
            },
        };

        // --- LOC ---
        match count_lines(file_path, spec) {
            Ok(Some(file_stats)) => {
                let entry = stats_by_lang
                    .entry(spec.name)
                    .or_insert_with(|| (0, FileStats::default()));
                entry.0 += 1;
                entry.1.blank += file_stats.blank;
                entry.1.comment += file_stats.comment;
                entry.1.code += file_stats.code;
            }
            Ok(None) => {} // binary
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
            }
        }

        // --- Binary check for remaining analyzers ---
        let file = match File::open(file_path) {
            Ok(f) => f,
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
                continue;
            }
        };
        let mut reader = BufReader::new(file);
        match is_binary_reader(&mut reader) {
            Ok(true) => continue,
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
                continue;
            }
            Ok(false) => {}
        }

        // --- Read file for line classification (used by MI) ---
        // Note: hal::analyze_file and cycom::analyze_file below each re-open
        // the file internally. Refactoring them to accept content would avoid
        // the extra reads, but OS page cache makes the cost negligible.
        // This read provides code_lines/comment_lines for MI computation.
        let content = match std::io::read_to_string(reader) {
            Ok(c) => c,
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
                continue;
            }
        };
        let kinds = classify_reader(content.as_bytes(), spec);
        let code_lines = kinds.iter().filter(|k| **k == LineKind::Code).count();
        let comment_lines = kinds.iter().filter(|k| **k == LineKind::Comment).count();

        // --- Dups: normalize file ---
        match dups::normalize_file(file_path, spec) {
            Ok(Some(nf)) => {
                total_code_lines += nf.lines.len();
                dup_files.push(nf);
            }
            Ok(None) => {}
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
            }
        }

        // --- Indent ---
        match indent::analyze_file(file_path, spec) {
            Ok(Some(fc)) => {
                indent_results.push(IndentEntry {
                    path: file_path.display().to_string(),
                    code_lines: fc.code_lines,
                    stddev: fc.stddev,
                    max_depth: fc.max_depth,
                    complexity: fc.complexity.as_str().to_string(),
                });
            }
            Ok(None) => {}
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
            }
        }

        // --- Halstead ---
        let volume_opt = match hal::analyze_file(file_path, spec) {
            Ok(Some(h)) => {
                let vol = h.metrics.volume;
                hal_results.push(HalsteadEntry {
                    path: file_path.display().to_string(),
                    volume: h.metrics.volume,
                    effort: h.metrics.effort,
                    bugs: h.metrics.bugs,
                });
                Some(vol)
            }
            Ok(None) => None,
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
                None
            }
        };

        // --- Cyclomatic ---
        let complexity_opt = match cycom::analyze_file(file_path, spec) {
            Ok(Some(c)) => {
                let total = c.total_complexity;
                cycom_results.push(CycomEntry {
                    path: file_path.display().to_string(),
                    functions: c.function_count,
                    total: c.total_complexity,
                    max: c.max_complexity,
                    avg: c.avg_complexity,
                    level: c.level.as_str().to_string(),
                });
                Some(total)
            }
            Ok(None) => None,
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
                None
            }
        };

        // --- MI computation (requires both Halstead volume and cyclomatic complexity) ---
        // Files are silently skipped for MI when: Halstead or cyclomatic analysis
        // returned None (e.g. no operators/operands), or compute_mi returns None
        // (e.g. zero code lines, zero volume). This is expected — not all source
        // files produce meaningful metrics for every analyzer.
        if let (Some(volume), Some(complexity)) = (volume_opt, complexity_opt) {
            let path_str = file_path.display().to_string();
            if let Some(m) = mi::analyzer::compute_mi(volume, complexity, code_lines) {
                mi_vs_results.push(MiVisualStudioEntry {
                    path: path_str.clone(),
                    mi_score: m.mi_score,
                    level: m.level.as_str().to_string(),
                });
            }
            if let Some(m) =
                miv::analyzer::compute_mi(volume, complexity, code_lines, comment_lines)
            {
                mi_vf_results.push(MiVerifysoftEntry {
                    path: path_str,
                    mi_score: m.mi_score,
                    level: m.level.as_str().to_string(),
                });
            }
        }
    }

    // --- Post-walk: Dups detection ---
    // Pass quiet=true to suppress boilerplate-skip notices in report context.
    let dup_groups = if dup_files.is_empty() {
        Vec::new()
    } else {
        dups::detector::detect_duplicates(&dup_files, min_lines, true)
    };

    let duplicated_lines: usize = dup_groups.iter().map(|g| g.duplicated_lines()).sum();
    let largest_block = dup_groups.iter().map(|g| g.line_count).max().unwrap_or(0);
    let files_with_dups: std::collections::HashSet<&Path> = dup_groups
        .iter()
        .flat_map(|g| g.locations.iter().map(|l| l.file_path.as_path()))
        .collect();
    let dup_percentage = if total_code_lines == 0 {
        0.0
    } else {
        duplicated_lines as f64 / total_code_lines as f64 * 100.0
    };

    // --- Build LOC reports ---
    let loc_reports: Vec<LanguageReport> = {
        let mut reports: Vec<LanguageReport> = stats_by_lang
            .into_iter()
            .map(|(name, (files, fs))| LanguageReport {
                name: name.to_string(),
                files,
                blank: fs.blank,
                comment: fs.comment,
                code: fs.code,
            })
            .collect();
        reports.sort_by(|a, b| b.code.cmp(&a.code).then_with(|| a.name.cmp(&b.name)));
        reports
    };

    // --- Sort, record totals, and truncate ---
    indent_results.sort_by(|a, b| b.stddev.total_cmp(&a.stddev));
    let indent_total = indent_results.len();
    indent_results.truncate(top);

    hal_results.sort_by(|a, b| b.effort.total_cmp(&a.effort));
    let hal_total = hal_results.len();
    hal_results.truncate(top);

    cycom_results.sort_by(|a, b| b.total.cmp(&a.total));
    let cycom_total = cycom_results.len();
    cycom_results.truncate(top);

    mi_vs_results.sort_by(|a, b| a.mi_score.total_cmp(&b.mi_score));
    let mi_vs_total = mi_vs_results.len();
    mi_vs_results.truncate(top);

    mi_vf_results.sort_by(|a, b| a.mi_score.total_cmp(&b.mi_score));
    let mi_vf_total = mi_vf_results.len();
    mi_vf_results.truncate(top);

    Ok(ProjectReport {
        path: path.display().to_string(),
        top,
        include_tests,
        min_lines,
        loc: loc_reports,
        duplication: DupsSummary {
            total_code_lines,
            duplicated_lines,
            duplication_percentage: dup_percentage,
            duplicate_groups: dup_groups.len(),
            files_with_duplicates: files_with_dups.len(),
            largest_block,
        },
        indent: SectionResult {
            total_count: indent_total,
            entries: indent_results,
        },
        halstead: SectionResult {
            total_count: hal_total,
            entries: hal_results,
        },
        cyclomatic: SectionResult {
            total_count: cycom_total,
            entries: cycom_results,
        },
        mi_visual_studio: SectionResult {
            total_count: mi_vs_total,
            entries: mi_vs_results,
        },
        mi_verifysoft: SectionResult {
            total_count: mi_vf_total,
            entries: mi_vf_results,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn run_on_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path(), false, false, 20, 6).unwrap();
    }

    #[test]
    fn run_on_empty_dir_json() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path(), true, false, 20, 6).unwrap();
    }

    #[test]
    fn run_on_rust_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
        )
        .unwrap();
        run(dir.path(), false, false, 20, 6).unwrap();
    }

    #[test]
    fn run_json_output() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
        )
        .unwrap();
        run(dir.path(), true, false, 20, 6).unwrap();
    }

    #[test]
    fn run_skips_binary() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
        run(dir.path(), false, false, 20, 6).unwrap();
    }

    // --- Tests that verify actual report structure ---

    #[test]
    fn build_report_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let report = build_report(dir.path(), false, 20, 6).unwrap();
        assert!(report.loc.is_empty());
        assert_eq!(report.duplication.total_code_lines, 0);
        assert_eq!(report.indent.total_count, 0);
        assert!(report.indent.entries.is_empty());
    }

    #[test]
    fn build_report_counts_loc() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "// comment\nfn main() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        let report = build_report(dir.path(), false, 20, 6).unwrap();

        assert_eq!(report.loc.len(), 1);
        assert_eq!(report.loc[0].name, "Rust");
        assert_eq!(report.loc[0].files, 1);
        assert_eq!(report.loc[0].comment, 1);
        assert_eq!(report.loc[0].code, 3);
    }

    #[test]
    fn build_report_no_dedup_for_loc() {
        let dir = tempfile::tempdir().unwrap();
        let content = "fn foo() {\n    let x = 1;\n}\n";
        fs::write(dir.path().join("a.rs"), content).unwrap();
        fs::write(dir.path().join("b.rs"), content).unwrap();
        let report = build_report(dir.path(), false, 20, 6).unwrap();

        // Both files counted — no dedup in report
        assert_eq!(report.loc[0].files, 2);
    }

    #[test]
    fn build_report_detects_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        // 7 code lines per file, all identical
        let code = "fn process() {\n    let x = read();\n    let y = transform(x);\n    write(y);\n    log(\"done\");\n    cleanup();\n}\n";
        fs::write(dir.path().join("a.rs"), code).unwrap();
        fs::write(dir.path().join("b.rs"), code).unwrap();
        let report = build_report(dir.path(), false, 20, 6).unwrap();

        // 1 duplicate group of 7 lines across 2 files
        assert_eq!(report.duplication.duplicate_groups, 1);
        // duplicated_lines = line_count * (locations - 1) = 7 * 1 = 7
        assert_eq!(report.duplication.duplicated_lines, 7);
        // total_code_lines = 7 * 2 files = 14
        assert_eq!(report.duplication.total_code_lines, 14);
        assert!((report.duplication.duplication_percentage - 50.0).abs() < 0.1);
        assert_eq!(report.duplication.files_with_duplicates, 2);
        assert_eq!(report.duplication.largest_block, 7);
    }

    #[test]
    fn build_report_top_truncates() {
        let dir = tempfile::tempdir().unwrap();
        // Create 3 files with different content so all produce indent results
        fs::write(dir.path().join("a.rs"), "fn a() {\n    let x = 1;\n}\n").unwrap();
        fs::write(
            dir.path().join("b.rs"),
            "fn b() {\n    let x = 1;\n    let y = 2;\n}\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("c.rs"),
            "fn c() {\n    let x = 1;\n    let y = 2;\n    let z = 3;\n}\n",
        )
        .unwrap();
        let report = build_report(dir.path(), false, 2, 6).unwrap();

        // 3 files analyzed, but only top 2 shown
        assert_eq!(report.indent.total_count, 3);
        assert_eq!(report.indent.entries.len(), 2);
    }

    #[test]
    fn build_report_excludes_tests() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("tests")).unwrap();
        fs::write(
            dir.path().join("tests/integration.rs"),
            "fn test() {\n    assert!(true);\n}\n",
        )
        .unwrap();
        let report = build_report(dir.path(), false, 20, 6).unwrap();

        // Test file in tests/ dir should be excluded
        assert!(report.loc.is_empty());
    }

    #[test]
    fn build_report_includes_tests_with_flag() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("tests")).unwrap();
        fs::write(
            dir.path().join("tests/integration.rs"),
            "fn test() {\n    assert!(true);\n}\n",
        )
        .unwrap();
        let report = build_report(dir.path(), true, 20, 6).unwrap();

        assert!(!report.loc.is_empty());
    }

    #[test]
    fn build_report_mi_computed() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
        )
        .unwrap();
        let report = build_report(dir.path(), false, 20, 6).unwrap();

        assert_eq!(report.mi_visual_studio.entries.len(), 1);
        assert_eq!(report.mi_verifysoft.entries.len(), 1);

        let vs = &report.mi_visual_studio.entries[0];
        // VS MI ~71.07 for this simple function — green level
        assert!((vs.mi_score - 71.07).abs() < 1.0);
        assert_eq!(vs.level, "green");

        let vf = &report.mi_verifysoft.entries[0];
        // Verifysoft MI ~121.54 — good level (no comments, so MIcw is zero)
        assert!((vf.mi_score - 121.54).abs() < 1.0);
        assert_eq!(vf.level, "good");
    }

    #[test]
    fn build_report_json_structure() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "// comment\nfn main() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        let report = build_report(dir.path(), false, 20, 6).unwrap();

        // Serialize to JSON and parse back to verify structure
        let json_str = serde_json::to_string(&report).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(value["path"].is_string());
        assert_eq!(value["top"], 20);
        assert_eq!(value["include_tests"], false);
        assert_eq!(value["min_lines"], 6);
        assert!(value["loc"].is_array());
        assert!(value["duplication"]["total_code_lines"].is_number());
        assert!(value["indent"]["total_count"].is_number());
        assert!(value["indent"]["entries"].is_array());
        assert!(value["halstead"]["total_count"].is_number());
        assert!(value["cyclomatic"]["total_count"].is_number());
        assert!(value["mi_visual_studio"]["total_count"].is_number());
        assert!(value["mi_verifysoft"]["total_count"].is_number());
    }

    #[test]
    fn build_report_min_lines_affects_dups() {
        let dir = tempfile::tempdir().unwrap();
        // 5 code lines per file
        let code = "fn f() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n}\n";
        fs::write(dir.path().join("a.rs"), code).unwrap();
        fs::write(dir.path().join("b.rs"), code).unwrap();

        // min_lines=3: block of 5 lines >= 3, so duplicates detected
        let report_low = build_report(dir.path(), false, 20, 3).unwrap();
        assert!(report_low.duplication.duplicate_groups > 0);

        // min_lines=100: block of 5 lines < 100, so no duplicates detected
        let report_high = build_report(dir.path(), false, 20, 100).unwrap();
        assert_eq!(report_high.duplication.duplicate_groups, 0);
    }
}
