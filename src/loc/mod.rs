mod counter;
mod language;
mod report;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

use ignore::WalkBuilder;

use std::fs::{self, File};
use std::io::{BufRead, BufReader};

use counter::{count_lines, FileStats};
use language::{detect, detect_by_shebang};
use report::{print_report, LanguageReport};

fn hash_file(path: &Path) -> Option<u64> {
    let content = fs::read(path).ok()?;
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    Some(hasher.finish())
}

pub fn run(path: &Path) -> Result<(), Box<dyn Error>> {
    let mut stats_by_lang: HashMap<&'static str, (usize, FileStats)> = HashMap::new();
    let mut seen_hashes: HashSet<u64> = HashSet::new();

    let walker = WalkBuilder::new(path)
        .follow_links(false)
        .build();

    for entry in walker {
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
        let spec = match detect(file_path) {
            Some(s) => s,
            None => {
                // Fallback: try shebang detection
                match try_detect_shebang(file_path) {
                    Some(s) => s,
                    None => continue,
                }
            }
        };

        // Skip duplicate files (same content)
        if let Some(h) = hash_file(file_path) {
            if !seen_hashes.insert(h) {
                continue;
            }
        }

        match count_lines(file_path, spec) {
            Ok(Some(file_stats)) => {
                let entry = stats_by_lang.entry(spec.name).or_insert_with(|| {
                    (0, FileStats::default())
                });
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
        println!("No recognized source files found.");
    } else {
        print_report(reports);
    }

    Ok(())
}

fn try_detect_shebang(path: &Path) -> Option<&'static language::LanguageSpec> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    reader.read_line(&mut first_line).ok()?;
    detect_by_shebang(&first_line)
}
