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
use std::fs::File;
use std::io::BufReader;
use std::time::Instant;

use crate::git::GitRepo;
use crate::util::hash_file;
use crate::walk::{self, WalkConfig};
use counter::{FileStats, LineKind, classify_reader, count_lines};
use report::{
    AuthorReport, LanguageReport, VerboseStats, print_author_json, print_author_report,
    print_author_short, print_author_terse, print_json, print_report, print_short, print_terse,
};

/// Walk source files, deduplicate by content hash, count lines per
/// language, and print a summary table or other format per `output`.
pub fn run(
    cfg: &WalkConfig<'_>,
    verbose: bool,
    output: crate::cli::OutputMode,
) -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    let mut stats_by_lang: HashMap<&'static str, (usize, FileStats)> = HashMap::new();
    let mut seen_hashes: HashSet<u64> = HashSet::new();
    let mut total_files: usize = 0;
    let mut unique_files: usize = 0;
    let mut duplicate_files: usize = 0;
    let mut binary_files: usize = 0;

    for (file_path, spec) in cfg.source_files() {
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
        match output {
            crate::cli::OutputMode::Json => {
                println!(
                    "{}",
                    serde_json::json!({"languages": [], "totals": {"files": 0, "blank": 0, "comment": 0, "code": 0}})
                );
            }
            _ => {
                println!("No recognized source files found.");
            }
        }
    } else {
        match output {
            crate::cli::OutputMode::Json => print_json(reports),
            crate::cli::OutputMode::Short => print_short(&reports),
            crate::cli::OutputMode::Terse => print_terse(&reports),
            crate::cli::OutputMode::Github | crate::cli::OutputMode::Codeclimate => {
                return Err(crate::cli::ERR_CI_FORMAT_ONLY.into());
            }
            crate::cli::OutputMode::Table => {
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
        }
    }

    Ok(())
}

/// Walk source files in a git repository, attribute each line to its author
/// via `git blame`, classify lines with the FSM, and print a per-author table.
pub fn run_by_author(
    cfg: &WalkConfig<'_>,
    output: crate::cli::OutputMode,
) -> Result<(), Box<dyn Error>> {
    let git = GitRepo::open(cfg.path)?;
    let (canonical_walk, prefix) = git.walk_prefix(cfg.path)?;

    // author email → (name, stats, files_set)
    let mut by_author: HashMap<String, (String, FileStats, HashSet<String>)> = HashMap::new();
    let mut seen_hashes: HashSet<u64> = HashSet::new();

    for (file_path, spec) in walk::source_files(&canonical_walk, cfg.exclude_tests(), cfg.filter) {
        if hash_file(&file_path).is_some_and(|h| !seen_hashes.insert(h)) {
            continue;
        }

        let Ok(file) = File::open(&file_path) else {
            continue;
        };
        let reader = BufReader::new(file);
        let kinds = classify_reader(reader, spec);

        let rel = GitRepo::to_git_path(&canonical_walk, &prefix, &file_path);
        let Ok(hunks) = git.blame_hunks(&rel) else {
            continue;
        };

        let file_key = rel.display().to_string();

        for hunk in hunks {
            let key = hunk.email.clone();
            let entry = by_author
                .entry(key)
                .or_insert_with(|| (hunk.author.clone(), FileStats::default(), HashSet::new()));

            entry.0 = hunk.author.clone();
            entry.2.insert(file_key.clone());

            for line_idx in (hunk.start_line - 1)..(hunk.start_line - 1 + hunk.lines) {
                match kinds.get(line_idx) {
                    Some(LineKind::Code) => entry.1.code += 1,
                    Some(LineKind::Comment) => entry.1.comment += 1,
                    Some(LineKind::Blank) => entry.1.blank += 1,
                    None => {}
                }
            }
        }
    }

    if by_author.is_empty() {
        match output {
            crate::cli::OutputMode::Json => {
                println!(
                    "{}",
                    serde_json::json!({"authors": [], "totals": {"files": 0, "blank": 0, "comment": 0, "code": 0}})
                );
            }
            _ => {
                println!("No recognized source files found.");
            }
        }
        return Ok(());
    }

    let reports: Vec<AuthorReport> = by_author
        .into_iter()
        .map(|(email, (name, fs, files))| AuthorReport {
            name,
            email,
            files: files.len(),
            blank: fs.blank,
            comment: fs.comment,
            code: fs.code,
        })
        .collect();

    match output {
        crate::cli::OutputMode::Json => print_author_json(reports),
        crate::cli::OutputMode::Short => print_author_short(&reports),
        crate::cli::OutputMode::Terse => print_author_terse(&reports),
        crate::cli::OutputMode::Github | crate::cli::OutputMode::Codeclimate => {
            return Err(crate::cli::ERR_CI_FORMAT_ONLY.into());
        }
        crate::cli::OutputMode::Table => print_author_report(reports),
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
