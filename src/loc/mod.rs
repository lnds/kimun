/// Lines-of-code counting module (like `cloc`).
///
/// Walks the directory tree, deduplicates files by content hash,
/// counts blank/comment/code lines per language via a character-level
/// FSM, and aggregates results for table or JSON output.
pub(crate) mod counter;
mod fsm;
mod lang_macro;
pub(crate) mod language;
pub(crate) mod report;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::Path;
use std::time::Instant;

use crate::util::hash_file;
use crate::walk;
use counter::{FileStats, count_lines};
use report::{LanguageReport, VerboseStats, print_json, print_report};

/// Walk source files, deduplicate by content hash, count lines per
/// language, and print a summary table (or JSON when `json` is true).
pub fn run(
    path: &Path,
    verbose: bool,
    json: bool,
    include_tests: bool,
) -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    let mut stats_by_lang: HashMap<&'static str, (usize, FileStats)> = HashMap::new();
    let mut seen_hashes: HashSet<u64> = HashSet::new();
    let mut total_files: usize = 0;
    let mut unique_files: usize = 0;
    let mut duplicate_files: usize = 0;
    let mut binary_files: usize = 0;

    for (file_path, spec) in walk::source_files(path, !include_tests) {
        total_files += 1;

        // Skip duplicate files (same content)
        if let Some(h) = hash_file(&file_path)
            && !seen_hashes.insert(h)
        {
            duplicate_files += 1;
            continue;
        }

        match count_lines(&file_path, spec) {
            Ok(Some(file_stats)) => {
                unique_files += 1;
                let entry = stats_by_lang
                    .entry(spec.name)
                    .or_insert_with(|| (0, FileStats::default()));
                entry.0 += 1;
                entry.1.blank += file_stats.blank;
                entry.1.comment += file_stats.comment;
                entry.1.code += file_stats.code;
            }
            Ok(None) => {
                binary_files += 1;
            }
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
            }
        }
    }

    let reports: Vec<LanguageReport> = stats_by_lang
        .into_iter()
        .map(|(name, (files, fs))| LanguageReport {
            name: name.to_string(),
            files,
            blank: fs.blank,
            comment: fs.comment,
            code: fs.code,
        })
        .collect();

    if reports.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::json!({"languages": [], "totals": {"files": 0, "blank": 0, "comment": 0, "code": 0}})
            );
        } else {
            println!("No recognized source files found.");
        }
    } else if json {
        print_json(reports);
    } else {
        let verbose_stats = if verbose {
            Some(VerboseStats {
                total_files,
                unique_files,
                duplicate_files,
                binary_files,
                elapsed: start.elapsed(),
            })
        } else {
            None
        };
        print_report(reports, verbose_stats);
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
