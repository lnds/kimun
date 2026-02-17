pub(crate) mod analyzer;
mod report;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::cycom;
use crate::dups;
use crate::hal;
use crate::indent;
use crate::loc::counter::{LineKind, classify_reader};
use crate::miv;
use crate::util::{find_test_block_start, is_binary_reader};
use crate::walk;

use analyzer::{
    DimensionScore, FileScore, ProjectScore, compute_project_score, normalize_complexity,
    normalize_duplication, normalize_file_size, normalize_halstead, normalize_indent, normalize_mi,
    score_to_grade,
};
use report::{print_json, print_report};

/// Per-file raw metrics collected during the walk.
struct FileMetrics {
    path: std::path::PathBuf,
    code_lines: usize,
    mi_score: Option<f64>,
    max_complexity: Option<usize>,
    indent_stddev: Option<f64>,
    halstead_effort: Option<f64>,
}

/// Dimension weights (must sum to 1.0).
/// MI gets the most weight (30%) because it's the most comprehensive metric
/// (combines Halstead volume, cyclomatic complexity, LOC, and comment ratio).
/// Halstead Effort (15%) uses per-LOC normalization to avoid penalizing large files.
/// Comment Ratio was removed (verifysoft MI already includes a comment weight term).
const W_MI: f64 = 0.30;
const W_CYCOM: f64 = 0.20;
const W_DUP: f64 = 0.15;
const W_INDENT: f64 = 0.15;
const W_HAL: f64 = 0.15;
const W_SIZE: f64 = 0.05;

/// All per-file dimension weights (excludes duplication, which is project-level).
const FILE_WEIGHTS: [(f64, &str); 5] = [
    (W_MI, "MI"),
    (W_CYCOM, "Cycom"),
    (W_INDENT, "Indent"),
    (W_HAL, "Halstead"),
    (W_SIZE, "Size"),
];

/// Default score for missing dimensions (neutral).
const MISSING_DIM_SCORE: f64 = 50.0;

pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    bottom: usize,
    min_lines: usize,
) -> Result<(), Box<dyn Error>> {
    let score = compute_score(path, include_tests, bottom, min_lines)?;

    // Show target in header when user specified an explicit path (not ".")
    let target = path.to_str().filter(|s| *s != ".").map(|s| s.to_string());

    if json {
        print_json(&score, target.as_deref())?;
    } else {
        print_report(&score, bottom, target.as_deref());
    }

    Ok(())
}

fn compute_score(
    path: &Path,
    include_tests: bool,
    bottom: usize,
    min_lines: usize,
) -> Result<ProjectScore, Box<dyn Error>> {
    let exclude_tests = !include_tests;
    let mut file_metrics: Vec<FileMetrics> = Vec::new();
    let mut dup_files: Vec<dups::detector::NormalizedFile> = Vec::new();
    let mut total_code_lines: usize = 0;

    for (file_path, spec) in walk::source_files(path, exclude_tests) {
        let file = match File::open(&file_path) {
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

        let content = match std::io::read_to_string(reader) {
            Ok(c) => c,
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
                continue;
            }
        };
        let lines: Vec<String> = content.lines().map(String::from).collect();
        let kinds = classify_reader(content.as_bytes(), spec);

        let code_lines = kinds.iter().filter(|k| **k == LineKind::Code).count();
        let comment_lines = kinds.iter().filter(|k| **k == LineKind::Comment).count();

        // Indent
        let indent_stddev = indent::analyzer::analyze(&lines, &kinds, 4).map(|m| m.stddev);

        // Halstead
        let hal_metrics = hal::analyze_content(&lines, &kinds, spec);
        let halstead_effort = hal_metrics.as_ref().map(|h| h.effort);
        let volume = hal_metrics.map(|h| h.volume);

        // Cyclomatic
        let cycom_result = cycom::analyze_content(&lines, &kinds, spec);
        let max_complexity = cycom_result.as_ref().map(|c| c.max_complexity);
        let total_complexity = cycom_result.map(|c| c.total_complexity);

        // MI (verifysoft variant)
        let mi_score = if let (Some(vol), Some(compl)) = (volume, total_complexity) {
            miv::analyzer::compute_mi(vol, compl, code_lines, comment_lines).map(|m| m.mi_score)
        } else {
            None
        };

        // Skip non-code files (Markdown, TOML, JSON, etc.) — no analyzable metrics
        if mi_score.is_none() && max_complexity.is_none() && halstead_effort.is_none() {
            continue;
        }

        // Dups: strip inline #[cfg(test)] blocks before normalization (Rust-specific)
        let dup_end = if exclude_tests {
            find_test_block_start(&lines)
        } else {
            lines.len()
        };
        let normalized = dups::normalize_content(&lines[..dup_end], &kinds[..dup_end]);
        total_code_lines += normalized.len();
        dup_files.push(dups::detector::NormalizedFile {
            path: file_path.to_path_buf(),
            lines: normalized,
        });

        file_metrics.push(FileMetrics {
            path: file_path.to_path_buf(),
            code_lines,
            mi_score,
            max_complexity,
            indent_stddev,
            halstead_effort,
        });
    }

    // Duplication (project-level)
    let dup_groups = if dup_files.is_empty() {
        Vec::new()
    } else {
        dups::detector::detect_duplicates(&dup_files, min_lines, true)
    };
    let duplicated_lines: usize = dup_groups.iter().map(|g| g.duplicated_lines()).sum();
    let dup_percent = if total_code_lines == 0 {
        0.0
    } else {
        duplicated_lines as f64 / total_code_lines as f64 * 100.0
    };

    let total_loc: usize = file_metrics.iter().map(|f| f.code_lines).sum();
    let files_analyzed = file_metrics.len();

    if files_analyzed == 0 {
        let dimensions = build_empty_dimensions();
        return Ok(ProjectScore {
            score: 0.0,
            grade: score_to_grade(0.0),
            files_analyzed: 0,
            total_loc: 0,
            dimensions,
            needs_attention: vec![],
        });
    }

    let dimensions = build_dimensions(&file_metrics, total_loc, dup_percent);
    let project_score = compute_project_score(&dimensions);
    let mut file_scores: Vec<FileScore> = file_metrics.iter().map(score_file).collect();
    file_scores.sort_by(|a, b| a.score.total_cmp(&b.score));
    file_scores.truncate(bottom);

    Ok(ProjectScore {
        score: project_score,
        grade: score_to_grade(project_score),
        files_analyzed,
        total_loc,
        dimensions,
        needs_attention: file_scores,
    })
}

fn build_dimensions(
    file_metrics: &[FileMetrics],
    total_loc: usize,
    dup_percent: f64,
) -> Vec<DimensionScore> {
    let mi_dim = weighted_mean(file_metrics, total_loc, |f| f.mi_score.map(normalize_mi));
    let cycom_dim = weighted_mean(file_metrics, total_loc, |f| {
        f.max_complexity.map(normalize_complexity)
    });
    let indent_dim = weighted_mean(file_metrics, total_loc, |f| {
        f.indent_stddev.map(normalize_indent)
    });
    let hal_dim = weighted_mean(file_metrics, total_loc, |f| {
        f.halstead_effort
            .map(|e| normalize_halstead(e, f.code_lines))
    });
    let size_dim = weighted_mean(file_metrics, total_loc, |f| {
        Some(normalize_file_size(f.code_lines))
    });
    let dup_dim = normalize_duplication(dup_percent);

    let dim = |name, weight, score| DimensionScore {
        name,
        weight,
        score,
        grade: score_to_grade(score),
    };

    vec![
        dim("Maintainability Index", W_MI, mi_dim),
        dim("Cyclomatic Complexity", W_CYCOM, cycom_dim),
        dim("Duplication", W_DUP, dup_dim),
        dim("Indentation Complexity", W_INDENT, indent_dim),
        dim("Halstead Effort", W_HAL, hal_dim),
        dim("File Size", W_SIZE, size_dim),
    ]
}

fn score_file(f: &FileMetrics) -> FileScore {
    let mut issues: Vec<String> = Vec::new();
    let file_weight_sum: f64 = FILE_WEIGHTS.iter().map(|(w, _)| w).sum();

    let mi_s = f
        .mi_score
        .map(|mi| {
            let s = normalize_mi(mi);
            if s < 60.0 {
                issues.push(format!("MI: {mi:.0}"));
            }
            s
        })
        .unwrap_or(MISSING_DIM_SCORE);

    let cycom_s = f
        .max_complexity
        .map(|c| {
            let s = normalize_complexity(c);
            if s < 60.0 {
                issues.push(format!("Complexity: {c}"));
            }
            s
        })
        .unwrap_or(MISSING_DIM_SCORE);

    let indent_s = f
        .indent_stddev
        .map(|sd| {
            let s = normalize_indent(sd);
            if s < 60.0 {
                issues.push(format!("Indent: {sd:.1}"));
            }
            s
        })
        .unwrap_or(MISSING_DIM_SCORE);

    let hal_s = f
        .halstead_effort
        .map(|e| {
            let s = normalize_halstead(e, f.code_lines);
            if s < 60.0 {
                issues.push(format!("Effort: {e:.0}"));
            }
            s
        })
        .unwrap_or(MISSING_DIM_SCORE);

    let size_s = normalize_file_size(f.code_lines);
    if f.code_lines > 1000 {
        issues.push(format!("Size: {} LOC", f.code_lines));
    }

    let weighted_sum =
        mi_s * W_MI + cycom_s * W_CYCOM + indent_s * W_INDENT + hal_s * W_HAL + size_s * W_SIZE;
    let file_score = weighted_sum / file_weight_sum;

    FileScore {
        path: f.path.clone(),
        score: file_score,
        grade: score_to_grade(file_score),
        loc: f.code_lines,
        issues,
    }
}

/// LOC-weighted mean of a normalized dimension across all files.
fn weighted_mean(
    files: &[FileMetrics],
    total_loc: usize,
    score_fn: impl Fn(&FileMetrics) -> Option<f64>,
) -> f64 {
    if total_loc == 0 {
        return 0.0;
    }
    let mut weighted_sum = 0.0;
    let mut weight_sum = 0usize;
    for f in files {
        if let Some(s) = score_fn(f) {
            let w = f.code_lines.max(1); // at least 1 to count the file
            weighted_sum += s * w as f64;
            weight_sum += w;
        }
    }
    if weight_sum == 0 {
        0.0
    } else {
        weighted_sum / weight_sum as f64
    }
}

fn build_empty_dimensions() -> Vec<DimensionScore> {
    build_dimensions(&[], 0, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn weights_sum_to_one() {
        let total = W_MI + W_CYCOM + W_DUP + W_INDENT + W_HAL + W_SIZE;
        assert!(
            (total - 1.0).abs() < 1e-10,
            "dimension weights must sum to 1.0, got {total}"
        );
    }

    #[test]
    fn file_weights_match_constants() {
        // Ensure FILE_WEIGHTS stays in sync with the individual constants
        let file_sum: f64 = FILE_WEIGHTS.iter().map(|(w, _)| w).sum();
        let expected = W_MI + W_CYCOM + W_INDENT + W_HAL + W_SIZE;
        assert!(
            (file_sum - expected).abs() < 1e-10,
            "FILE_WEIGHTS sum should match non-dup constants"
        );
    }

    #[test]
    fn weighted_mean_all_none() {
        let files = vec![FileMetrics {
            path: "a.rs".into(),
            code_lines: 100,

            mi_score: None,
            max_complexity: None,
            indent_stddev: None,
            halstead_effort: None,
        }];
        let result = weighted_mean(&files, 100, |_| None);
        assert!((result - 0.0).abs() < 0.01, "all None → 0, got {result}");
    }

    #[test]
    fn weighted_mean_total_loc_zero() {
        let files: Vec<FileMetrics> = vec![];
        let result = weighted_mean(&files, 0, |_| Some(80.0));
        assert!((result - 0.0).abs() < 0.01, "total_loc=0 → 0, got {result}");
    }

    #[test]
    fn weighted_mean_single_file() {
        let files = vec![FileMetrics {
            path: "a.rs".into(),
            code_lines: 100,

            mi_score: Some(85.0),
            max_complexity: Some(5),
            indent_stddev: Some(1.0),
            halstead_effort: Some(1000.0),
        }];
        let result = weighted_mean(&files, 100, |f| f.mi_score);
        assert!(
            (result - 85.0).abs() < 0.01,
            "single file → same value, got {result}"
        );
    }

    #[test]
    fn weighted_mean_loc_weighted() {
        let files = vec![
            FileMetrics {
                path: "small.rs".into(),
                code_lines: 10,

                mi_score: Some(100.0),
                max_complexity: None,
                indent_stddev: None,
                halstead_effort: None,
            },
            FileMetrics {
                path: "big.rs".into(),
                code_lines: 90,

                mi_score: Some(50.0),
                max_complexity: None,
                indent_stddev: None,
                halstead_effort: None,
            },
        ];
        let result = weighted_mean(&files, 100, |f| f.mi_score);
        // (100*10 + 50*90) / 100 = (1000 + 4500) / 100 = 55
        assert!(
            (result - 55.0).abs() < 0.01,
            "LOC-weighted → 55, got {result}"
        );
    }

    #[test]
    fn run_on_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path(), false, false, 10, 6).unwrap();
    }

    #[test]
    fn run_on_rust_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "// a module\nfn main() {\n    let x = 1;\n    let y = x + 2;\n    println!(\"{}\", y);\n}\n",
        )
        .unwrap();
        let score = compute_score(dir.path(), false, 10, 6).unwrap();
        assert!(
            score.score > 50.0,
            "simple code should score well, got {}",
            score.score
        );
        assert_eq!(score.files_analyzed, 1);
        assert!(score.total_loc > 0);
        assert_eq!(score.dimensions.len(), 6);
    }

    #[test]
    fn run_json_output() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        run(dir.path(), true, false, 10, 6).unwrap();
    }

    #[test]
    fn run_includes_tests_with_flag() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("tests")).unwrap();
        fs::write(
            dir.path().join("tests/integration.rs"),
            "fn test() {\n    assert!(true);\n}\n",
        )
        .unwrap();
        let score = compute_score(dir.path(), true, 10, 6).unwrap();
        assert_eq!(score.files_analyzed, 1);
    }

    #[test]
    fn run_excludes_tests_by_default() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("tests")).unwrap();
        fs::write(
            dir.path().join("tests/integration.rs"),
            "fn test() {\n    assert!(true);\n}\n",
        )
        .unwrap();
        let score = compute_score(dir.path(), false, 10, 6).unwrap();
        assert_eq!(score.files_analyzed, 0);
    }

    #[test]
    fn run_on_current_repo() {
        // Smoke test on the actual repo
        run(Path::new("."), false, false, 5, 6).unwrap();
    }

    #[test]
    fn find_test_block_start_finds_cfg_test() {
        let lines = vec![
            "fn foo() {}".to_string(),
            "#[cfg(test)]".to_string(),
            "mod tests {}".to_string(),
        ];
        assert_eq!(find_test_block_start(&lines), 1);
    }

    #[test]
    fn find_test_block_start_with_leading_spaces() {
        let lines = vec![
            "fn foo() {}".to_string(),
            "  #[cfg(test)]  ".to_string(),
            "mod tests {}".to_string(),
        ];
        assert_eq!(find_test_block_start(&lines), 1);
    }

    #[test]
    fn find_test_block_start_no_match() {
        let lines = vec!["fn foo() {}".to_string(), "fn bar() {}".to_string()];
        assert_eq!(find_test_block_start(&lines), 2);
    }

    #[test]
    fn find_test_block_start_empty() {
        let lines: Vec<String> = vec![];
        assert_eq!(find_test_block_start(&lines), 0);
    }

    #[test]
    fn excludes_markdown_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("README.md"), "# Hello\n\nWorld\n").unwrap();
        let score = compute_score(dir.path(), false, 10, 6).unwrap();
        assert_eq!(score.files_analyzed, 0, "Markdown should be excluded");
    }

    #[test]
    fn excludes_toml_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\n",
        )
        .unwrap();
        let score = compute_score(dir.path(), false, 10, 6).unwrap();
        assert_eq!(score.files_analyzed, 0, "TOML should be excluded");
    }

    #[test]
    fn excludes_json_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("data.json"), "{\"key\": \"value\"}\n").unwrap();
        let score = compute_score(dir.path(), false, 10, 6).unwrap();
        assert_eq!(score.files_analyzed, 0, "JSON should be excluded");
    }

    #[test]
    fn run_on_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("lib.rs");
        fs::write(
            &file,
            "/// Docs\nfn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
        )
        .unwrap();
        let score = compute_score(&file, false, 10, 6).unwrap();
        assert_eq!(score.files_analyzed, 1);
        assert!(score.total_loc > 0);
    }

    #[test]
    fn dimensions_sum_to_100_percent() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    let x = 1;\n}\n",
        )
        .unwrap();
        let score = compute_score(dir.path(), false, 10, 6).unwrap();
        let total_weight: f64 = score.dimensions.iter().map(|d| d.weight).sum();
        assert!(
            (total_weight - 1.0).abs() < 0.001,
            "weights should sum to 1.0, got {total_weight}"
        );
    }
}
