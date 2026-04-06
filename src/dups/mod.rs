//! Duplicate code detection module.
//!
//! Finds repeated code blocks across files using content-hash based matching.
//! Normalizes lines (strips whitespace, filters non-code) before comparison,
//! optionally excluding inline `#[cfg(test)]` blocks. The detection algorithm
//! uses a sliding-window fingerprint to identify identical code sequences,
//! then groups them by severity (Critical for 3+ occurrences, Tolerable for 2).
pub(crate) mod detector;
pub(crate) mod report;

use std::collections::HashSet;
use std::error::Error;
use std::path::Path;

use crate::git::GitRepo;
use crate::loc::counter::LineKind;
use crate::loc::language::LanguageSpec;
use crate::util::{find_test_block_start, read_and_classify};
use crate::walk::WalkConfig;
use detector::{NormalizedFile, NormalizedLine, detect_duplicates};
use report::{DuplicationMetrics, display_limit, print_detailed, print_json, print_summary};

/// Normalize pre-read content (avoids re-reading the file).
pub(crate) fn normalize_content(lines: &[String], kinds: &[LineKind]) -> Vec<NormalizedLine> {
    lines
        .iter()
        .zip(kinds.iter())
        .enumerate()
        .filter(|(_, (_, kind))| **kind == LineKind::Code)
        .map(|(i, (line, _))| NormalizedLine {
            original_line_number: i + 1,
            content: line.trim().to_string(),
        })
        .collect()
}

/// Read a file, classify its lines, and normalize code lines for duplication.
/// Returns `None` for binary files. Strips test blocks when `exclude_tests`.
pub(crate) fn normalize_file(
    path: &Path,
    spec: &LanguageSpec,
    exclude_tests: bool,
) -> Result<Option<NormalizedFile>, Box<dyn Error>> {
    let (lines, kinds) = match read_and_classify(path, spec)? {
        Some(v) => v,
        None => return Ok(None),
    };

    // Strip inline #[cfg(test)] blocks when excluding tests (Rust-specific)
    let end = if exclude_tests {
        find_test_block_start(&lines)
    } else {
        lines.len()
    };

    Ok(Some(NormalizedFile {
        path: path.to_path_buf(),
        lines: normalize_content(&lines[..end], &kinds[..end]),
    }))
}

/// Compute duplication metrics for a given walk config without printing anything.
fn compute_metrics(cfg: &WalkConfig<'_>, min_lines: usize) -> DuplicationMetrics {
    let exclude_tests = cfg.exclude_tests();
    let mut files: Vec<NormalizedFile> = Vec::new();
    let mut total_code_lines: usize = 0;

    for (file_path, spec) in cfg.source_files() {
        match normalize_file(&file_path, spec, exclude_tests) {
            Ok(Some(nf)) => {
                total_code_lines += nf.lines.len();
                files.push(nf);
            }
            Ok(None) => {}
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
            }
        }
    }

    let groups = detect_duplicates(&files, min_lines, false);
    let duplicated_lines: usize = groups.iter().map(|g| g.duplicated_lines()).sum();
    let largest_block = groups.iter().map(|g| g.line_count).max().unwrap_or(0);
    let files_with_dups: HashSet<&Path> = groups
        .iter()
        .flat_map(|g| g.locations.iter().map(|l| l.file_path.as_path()))
        .collect();

    DuplicationMetrics {
        total_code_lines,
        duplicated_lines,
        duplicate_groups: groups.len(),
        files_with_duplicates: files_with_dups.len(),
        largest_block,
    }
}

/// Quality gate options for `km dups`.
/// All conditions are independent and checked after the report is printed.
#[derive(Debug, Clone, Default)]
pub struct DupsGate {
    /// Fail if the number of duplicate groups exceeds this limit.
    pub max_duplicates: Option<usize>,
    /// Fail if the duplicated-lines ratio exceeds this percentage.
    pub max_dup_ratio: Option<f64>,
    /// Fail if the current ratio is higher than at this git ref.
    pub fail_on_increase: Option<String>,
}

/// Run the full duplication analysis pipeline: walk files, normalize,
/// detect duplicates, and print results (summary, detailed, or JSON).
///
/// Quality gates in `gate` are checked after the report is printed so CI
/// logs are always complete before any non-zero exit:
/// - `max_duplicates`: fails if the number of duplicate groups exceeds the
///   limit — `--max-duplicates 0` fails on any duplicate.
/// - `max_dup_ratio`: enforces a percentage ceiling — `--max-dup-ratio 5.0`
///   fails when more than 5% of code lines are duplicated.
/// - `fail_on_increase`: compares the current ratio against a git ref and
///   fails if duplication has grown since then.
pub fn run(
    cfg: &WalkConfig<'_>,
    min_lines: usize,
    show_report: bool,
    show_all: bool,
    json: bool,
    gate: DupsGate,
) -> Result<(), Box<dyn Error>> {
    let exclude_tests = cfg.exclude_tests();
    let mut files: Vec<NormalizedFile> = Vec::new();
    let mut total_code_lines: usize = 0;

    for (file_path, spec) in cfg.source_files() {
        match normalize_file(&file_path, spec, exclude_tests) {
            Ok(Some(nf)) => {
                total_code_lines += nf.lines.len();
                files.push(nf);
            }
            Ok(None) => {} // binary, skip
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
            }
        }
    }

    if files.is_empty() {
        if json {
            let metrics = DuplicationMetrics {
                total_code_lines: 0,
                duplicated_lines: 0,
                duplicate_groups: 0,
                files_with_duplicates: 0,
                largest_block: 0,
            };
            print_json(&metrics, &[])?;
        } else {
            println!("No recognized source files found.");
        }
        return Ok(());
    }

    let groups = detect_duplicates(&files, min_lines, json);

    let duplicated_lines: usize = groups.iter().map(|g| g.duplicated_lines()).sum();
    let largest_block = groups.iter().map(|g| g.line_count).max().unwrap_or(0);

    let files_with_dups: HashSet<&Path> = groups
        .iter()
        .flat_map(|g| g.locations.iter().map(|l| l.file_path.as_path()))
        .collect();

    let metrics = DuplicationMetrics {
        total_code_lines,
        duplicated_lines,
        duplicate_groups: groups.len(),
        files_with_duplicates: files_with_dups.len(),
        largest_block,
    };

    // Always print first so CI logs show the full report before any gate error.
    if json {
        let limit = display_limit(groups.len(), show_all);
        print_json(&metrics, &groups[..limit])?;
    } else if show_report {
        let limit = display_limit(groups.len(), show_all);
        print_detailed(&metrics, &groups[..limit], groups.len());
    } else {
        print_summary(&metrics, &groups);
    }

    if gate.max_duplicates.is_some_and(|max| groups.len() > max) {
        let max = gate.max_duplicates.unwrap();
        return Err(format!(
            "quality gate failed: {} duplicate groups found (limit: {max})",
            groups.len()
        )
        .into());
    }

    if let Some(max_pct) = gate.max_dup_ratio {
        let actual_pct = if metrics.total_code_lines > 0 {
            metrics.duplicated_lines as f64 / metrics.total_code_lines as f64 * 100.0
        } else {
            0.0
        };
        if actual_pct > max_pct {
            return Err(format!(
                "quality gate failed: {actual_pct:.1}% duplication ratio exceeds limit of {max_pct:.1}%"
            )
            .into());
        }
    }

    if let Some(git_ref) = gate.fail_on_increase.as_deref() {
        let after_ratio = if metrics.total_code_lines > 0 {
            metrics.duplicated_lines as f64 / metrics.total_code_lines as f64 * 100.0
        } else {
            0.0
        };

        let repo = GitRepo::open(cfg.path)?;
        let tmpdir = tempfile::tempdir()?;
        repo.extract_tree_to_dir(git_ref, tmpdir.path())?;

        let (_, prefix) = repo.walk_prefix(cfg.path)?;
        let tmp_path = if prefix.as_os_str().is_empty() {
            tmpdir.path().to_path_buf()
        } else {
            tmpdir.path().join(&prefix)
        };

        let ref_cfg = WalkConfig::new(&tmp_path, cfg.include_tests, cfg.filter);
        let ref_metrics = compute_metrics(&ref_cfg, min_lines);

        let before_ratio = if ref_metrics.total_code_lines > 0 {
            ref_metrics.duplicated_lines as f64 / ref_metrics.total_code_lines as f64 * 100.0
        } else {
            0.0
        };

        // Compare at 0.01% resolution (the display precision) to avoid spurious
        // failures from floating-point noise smaller than one display unit.
        if (after_ratio * 100.0).round() > (before_ratio * 100.0).round() {
            return Err(format!(
                "quality gate failed: duplication increased from {before_ratio:.2}% to {after_ratio:.2}% vs {git_ref}"
            )
            .into());
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
