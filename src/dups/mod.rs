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

use crate::loc::counter::LineKind;
use crate::loc::language::LanguageSpec;
use crate::util::{find_test_block_start, read_and_classify};
use crate::walk;
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

/// Run the full duplication analysis pipeline: walk files, normalize,
/// detect duplicates, and print results (summary, detailed, or JSON).
pub fn run(
    path: &Path,
    min_lines: usize,
    show_report: bool,
    show_all: bool,
    json: bool,
    exclude_tests: bool,
) -> Result<(), Box<dyn Error>> {
    let mut files: Vec<NormalizedFile> = Vec::new();
    let mut total_code_lines: usize = 0;

    for (file_path, spec) in walk::source_files(path, exclude_tests) {
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

    if json {
        let limit = display_limit(groups.len(), show_all);
        print_json(&metrics, &groups[..limit])?;
    } else if show_report {
        let limit = display_limit(groups.len(), show_all);
        print_detailed(&metrics, &groups[..limit], groups.len());
    } else {
        print_summary(&metrics, &groups);
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
