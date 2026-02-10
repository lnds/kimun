pub(crate) mod counter;
pub(crate) mod language;
pub(crate) mod report;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::Path;
use std::time::Instant;

use crate::util::hash_file;
use crate::walk;
use counter::{FileStats, count_lines};
use language::detect;
use report::{LanguageReport, VerboseStats, print_json, print_report};

pub fn run(path: &Path, verbose: bool, json: bool) -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    let mut stats_by_lang: HashMap<&'static str, (usize, FileStats)> = HashMap::new();
    let mut seen_hashes: HashSet<u64> = HashSet::new();
    let mut total_files: usize = 0;
    let mut unique_files: usize = 0;

    for entry in walk::walk(path, false) {
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

        total_files += 1;
        let file_path = entry.path();
        let spec = match detect(file_path) {
            Some(s) => s,
            None => {
                // Fallback: try shebang detection
                match walk::try_detect_shebang(file_path) {
                    Some(s) => s,
                    None => continue,
                }
            }
        };

        // Skip duplicate files (same content)
        if let Some(h) = hash_file(file_path)
            && !seen_hashes.insert(h)
        {
            continue;
        }

        match count_lines(file_path, spec) {
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
            Ok(None) => {} // binary, skip
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
                skipped_files: total_files - unique_files,
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
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn run_on_temp_dir_with_rust_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    // hello\n    println!(\"hi\");\n}\n",
        )
        .unwrap();

        // Should succeed without error
        run(dir.path(), false, false).unwrap();
    }

    #[test]
    fn run_on_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        // Should succeed and print "No recognized source files found."
        run(dir.path(), false, false).unwrap();
    }

    #[test]
    fn run_skips_binary_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("data.c"), b"hello\x00world").unwrap();
        // Should succeed — binary file silently skipped
        run(dir.path(), false, false).unwrap();
    }

    #[test]
    fn run_deduplicates_identical_files() {
        let dir = tempfile::tempdir().unwrap();
        let content = "int x = 1;\n";
        fs::write(dir.path().join("a.c"), content).unwrap();
        fs::write(dir.path().join("b.c"), content).unwrap();
        // Should succeed — one of the duplicates skipped
        run(dir.path(), false, false).unwrap();
    }

    #[test]
    fn run_with_shebang_detection() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("script"),
            "#!/usr/bin/env python3\nprint('hello')\n",
        )
        .unwrap();
        run(dir.path(), false, false).unwrap();
    }

    #[test]
    fn run_verbose_on_temp_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn x() {}\n").unwrap();
        // Should succeed with verbose stats printed
        run(dir.path(), true, false).unwrap();
    }

    #[test]
    fn run_verbose_with_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let content = "int x = 1;\n";
        fs::write(dir.path().join("a.c"), content).unwrap();
        fs::write(dir.path().join("b.c"), content).unwrap();
        // Should show skipped_files=1 (duplicate)
        run(dir.path(), true, false).unwrap();
    }

    #[test]
    fn run_verbose_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path(), true, false).unwrap();
    }

    #[test]
    fn run_json_output() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("main.rs"),
            "fn main() {\n    println!(\"hi\");\n}\n",
        )
        .unwrap();
        run(dir.path(), false, true).unwrap();
    }

    #[test]
    fn run_json_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path(), false, true).unwrap();
    }

    #[test]
    fn hash_file_works() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "hello world").unwrap();

        let h1 = hash_file(&path).unwrap();
        let h2 = hash_file(&path).unwrap();
        assert_eq!(h1, h2, "same content should produce same hash");
    }

    #[test]
    fn hash_file_nonexistent() {
        assert!(hash_file(Path::new("/nonexistent/file")).is_none());
    }
}
