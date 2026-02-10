mod report;

use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::git::GitRepo;
use crate::loc::language::detect;
use crate::walk;
use report::{print_json, print_report};

pub struct FileHotspot {
    pub path: PathBuf,
    pub language: String,
    pub commits: usize,
    pub complexity: usize,
    pub score: usize,
}

/// Parse a duration string like "6m", "1y", "30d" into a Unix timestamp
/// representing that far back from now.
///
/// Approximations: 1 month = 30 days, 1 year = 365 days.
fn parse_since(s: &str) -> Result<i64, Box<dyn Error>> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty --since value".into());
    }

    let (num_str, unit) = s.split_at(s.len() - 1);
    let n: u64 = num_str
        .parse()
        .map_err(|_| format!("invalid --since value: {s:?} (expected e.g. 6m, 1y, 30d)"))?;

    let seconds = match unit {
        "d" => n.checked_mul(86_400),
        "m" => n.checked_mul(30 * 86_400),
        "y" => n.checked_mul(365 * 86_400),
        _ => return Err(format!("unknown unit in --since: {s:?} (use d, m, or y)").into()),
    }
    .ok_or("--since value too large")?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    Ok((now - seconds) as i64)
}

pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    top: usize,
    sort_by: &str,
    since: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let git_repo =
        GitRepo::open(path).map_err(|e| format!("not a git repository (or any parent): {e}"))?;

    let since_ts = match since {
        Some(s) => Some(parse_since(s)?),
        None => None,
    };

    // Build a HashMap of relative path → commits
    let freqs = git_repo.file_frequencies(since_ts)?;
    let freq_map: HashMap<PathBuf, usize> =
        freqs.into_iter().map(|f| (f.path, f.commits)).collect();

    let git_root = git_repo.root().canonicalize()?;
    let exclude_tests = !include_tests;
    let mut results: Vec<FileHotspot> = Vec::new();

    for entry in walk::walk(path, exclude_tests) {
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

        if exclude_tests && walk::is_test_file(file_path) {
            continue;
        }

        let spec = match detect(file_path) {
            Some(s) => s,
            None => match walk::try_detect_shebang(file_path) {
                Some(s) => s,
                None => continue,
            },
        };

        // Compute cyclomatic complexity
        let complexity = match crate::cycom::analyze_file(file_path, spec) {
            Ok(Some(c)) => c.total_complexity,
            Ok(None) => continue,
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
                continue;
            }
        };

        // Resolve walk path to absolute so we can strip the (absolute) git root.
        // Git returns relative paths from the repo root; after stripping we get
        // the same relative path that git uses in its diff output.
        let abs_path = match file_path.canonicalize() {
            Ok(p) => p,
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
                continue;
            }
        };
        let rel_path = match abs_path.strip_prefix(&git_root) {
            Ok(rel) => rel.to_path_buf(),
            Err(_) => {
                eprintln!("warning: file outside git root: {}", file_path.display());
                continue;
            }
        };

        // Look up commits from git history
        let commits = match freq_map.get(&rel_path) {
            Some(&c) => c,
            None => continue, // no git history for this file
        };

        let score = commits * complexity;

        results.push(FileHotspot {
            path: rel_path,
            language: spec.name.to_string(),
            commits,
            complexity,
            score,
        });
    }

    // Sort by chosen metric descending
    match sort_by {
        "commits" => results.sort_by(|a, b| b.commits.cmp(&a.commits)),
        "complexity" => results.sort_by(|a, b| b.complexity.cmp(&a.complexity)),
        _ => results.sort_by(|a, b| b.score.cmp(&a.score)),
    }

    results.truncate(top);

    if json {
        print_json(&results)?;
    } else {
        print_report(&results);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_since_days() {
        let ts = parse_since("30d").unwrap();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let expected = now - 30 * 86_400;
        assert!(
            (ts - expected).abs() < 2,
            "timestamp should be within 2s of 30 days ago (got {ts}, expected ~{expected})"
        );
    }

    #[test]
    fn parse_since_months() {
        let ts = parse_since("6m").unwrap();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let expected = now - 6 * 30 * 86_400;
        assert!(
            (ts - expected).abs() < 2,
            "timestamp should be within 2s of 6 months ago (got {ts}, expected ~{expected})"
        );
    }

    #[test]
    fn parse_since_years() {
        let ts = parse_since("1y").unwrap();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let expected = now - 365 * 86_400;
        assert!(
            (ts - expected).abs() < 2,
            "timestamp should be within 2s of 1 year ago (got {ts}, expected ~{expected})"
        );
    }

    #[test]
    fn parse_since_invalid_unit() {
        let err = parse_since("5x").unwrap_err();
        assert!(
            err.to_string().contains("unknown unit"),
            "should mention unknown unit, got: {err}"
        );
    }

    #[test]
    fn parse_since_invalid_number() {
        let err = parse_since("abcd").unwrap_err();
        assert!(
            err.to_string().contains("invalid"),
            "should mention invalid value, got: {err}"
        );
    }

    #[test]
    fn parse_since_empty() {
        let err = parse_since("").unwrap_err();
        assert!(
            err.to_string().contains("empty"),
            "should mention empty value, got: {err}"
        );
    }

    #[test]
    fn parse_since_overflow() {
        let err = parse_since("999999999999999999999y").unwrap_err();
        assert!(
            err.to_string().contains("too large") || err.to_string().contains("invalid"),
            "should reject overflow, got: {err}"
        );
    }

    #[test]
    fn run_on_non_git_dir() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("not_a_repo");
        std::fs::create_dir_all(&sub).unwrap();
        let err = run(&sub, false, false, 20, "score", None).unwrap_err();
        assert!(
            err.to_string().contains("not a git repository"),
            "should mention not a git repository, got: {err}"
        );
    }

    #[test]
    fn run_json_output() {
        // Run on this repo itself — should succeed
        let result = run(Path::new("."), true, false, 5, "score", None);
        assert!(result.is_ok(), "hotspots should succeed on a git repo");
    }
}
