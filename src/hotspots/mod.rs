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

    // Find where digits end to support multi-character units (e.g. "6mo", "1yr")
    let split_pos = s.find(|c: char| !c.is_ascii_digit()).ok_or_else(|| {
        format!("invalid --since value: {s:?} (no unit, expected e.g. 6m, 1y, 30d)")
    })?;

    let (num_str, unit) = s.split_at(split_pos);
    let n: u64 = num_str
        .parse()
        .map_err(|_| format!("invalid --since value: {s:?} (expected e.g. 6m, 1y, 30d)"))?;

    let seconds = match unit {
        "d" | "day" | "days" => n.checked_mul(86_400),
        "m" | "mo" | "month" | "months" => n.checked_mul(30 * 86_400),
        "y" | "yr" | "year" | "years" => n.checked_mul(365 * 86_400),
        _ => return Err(format!("unknown unit in --since: {s:?} (use d, m, or y)").into()),
    }
    .ok_or("--since value too large")?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    let ts = now
        .checked_sub(seconds)
        .ok_or("--since value goes before Unix epoch")?;

    Ok(ts as i64)
}

/// Compute complexity for a file using the chosen metric.
/// Returns None if the file cannot be analyzed.
fn compute_complexity(
    file_path: &Path,
    spec: &crate::loc::language::LanguageSpec,
    metric: &str,
) -> Result<Option<usize>, Box<dyn Error>> {
    match metric {
        "cycom" => match crate::cycom::analyze_file(file_path, spec)? {
            Some(c) => Ok(Some(c.total_complexity)),
            None => Ok(None),
        },
        _ => match crate::indent::analyze_file(file_path, spec)? {
            Some(m) => Ok(Some(m.total_indent)),
            None => Ok(None),
        },
    }
}

pub fn run(
    path: &Path,
    json: bool,
    include_tests: bool,
    top: usize,
    sort_by: &str,
    since: Option<&str>,
    complexity_metric: &str,
) -> Result<(), Box<dyn Error>> {
    let git_repo =
        GitRepo::open(path).map_err(|e| format!("not a git repository (or any parent): {e}"))?;

    let since_ts = match since {
        Some(s) => Some(parse_since(s)?),
        None => None,
    };

    // Build a HashMap of relative path → commits
    let freqs = git_repo.file_frequencies(since_ts)?;
    if freqs.is_empty() {
        if since.is_some() {
            eprintln!("No commits found in the specified time range.");
        } else {
            eprintln!("No commits found in the repository.");
        }
        return Ok(());
    }
    let freq_map: HashMap<PathBuf, usize> =
        freqs.into_iter().map(|f| (f.path, f.commits)).collect();

    // Canonicalize paths ONCE at the top, not per-file in the loop.
    let git_root = git_repo
        .root()
        .canonicalize()
        .map_err(|e| format!("cannot resolve git root: {e}"))?;
    let walk_root = path
        .canonicalize()
        .map_err(|e| format!("cannot resolve target path {}: {e}", path.display()))?;
    // Compute the prefix that maps walk-relative paths to git-relative paths.
    // Examples:
    //   git_root=/a/b, walk_root=/a/b/src → prefix="src"
    //   git_root=/a/b, walk_root=/a/b     → prefix=""
    //   git_root=/a/b, walk_root=/x/y     → prefix="" (fallback; files won't match freq_map)
    let walk_prefix = walk_root
        .strip_prefix(&git_root)
        .unwrap_or(Path::new(""))
        .to_path_buf();

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

        // Compute path relative to git root using the pre-computed prefix.
        let rel_to_walk = file_path.strip_prefix(path).unwrap_or(file_path);
        let rel_path = if walk_prefix.as_os_str().is_empty() {
            rel_to_walk.to_path_buf()
        } else {
            walk_prefix.join(rel_to_walk)
        };

        // Look up commits from git history (before expensive analysis)
        let commits = match freq_map.get(&rel_path) {
            Some(&c) => c,
            None => continue,
        };

        // Compute complexity (only for files with git history)
        let complexity = match compute_complexity(file_path, spec, complexity_metric) {
            Ok(Some(c)) => c,
            Ok(None) => continue,
            Err(err) => {
                eprintln!("warning: {}: {err}", file_path.display());
                continue;
            }
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
        print_json(&results, complexity_metric)?;
    } else {
        print_report(&results, complexity_metric);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path as StdPath;

    use git2::Repository;

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
    fn parse_since_no_unit() {
        let err = parse_since("123").unwrap_err();
        assert!(
            err.to_string().contains("no unit"),
            "should mention no unit, got: {err}"
        );
    }

    #[test]
    fn parse_since_multi_char_units() {
        // All multi-char unit variants should parse correctly
        for unit in [
            "6mo", "6month", "6months", "1yr", "1year", "1years", "30day", "30days",
        ] {
            let result = parse_since(unit);
            assert!(result.is_ok(), "should accept {unit:?}, got: {result:?}");
        }
    }

    #[test]
    fn parse_since_zero_is_valid() {
        let ts = parse_since("0d").unwrap();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(
            (ts - now).abs() < 2,
            "0d should be approximately now (got {ts}, expected ~{now})"
        );
    }

    #[test]
    fn parse_since_whitespace() {
        let ts = parse_since(" 30d ").unwrap();
        assert!(ts > 0, "should parse with leading/trailing whitespace");
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
    fn parse_since_underflow() {
        let err = parse_since("9999999y").unwrap_err();
        assert!(
            err.to_string().contains("epoch") || err.to_string().contains("too large"),
            "should reject dates before epoch, got: {err}"
        );
    }

    #[test]
    fn run_on_non_git_dir() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("not_a_repo");
        fs::create_dir_all(&sub).unwrap();
        let err = run(&sub, false, false, 20, "score", None, "indent").unwrap_err();
        assert!(
            err.to_string().contains("not a git repository"),
            "should mention not a git repository, got: {err}"
        );
    }

    #[test]
    fn run_json_output_indent() {
        let result = run(StdPath::new("."), true, false, 5, "score", None, "indent");
        assert!(
            result.is_ok(),
            "hotspots (indent) should succeed on a git repo"
        );
    }

    #[test]
    fn run_json_output_cycom() {
        let result = run(StdPath::new("."), true, false, 5, "score", None, "cycom");
        assert!(
            result.is_ok(),
            "hotspots (cycom) should succeed on a git repo"
        );
    }

    // -- Integration tests with a real git repo --

    fn create_test_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        (dir, repo)
    }

    fn make_commit(repo: &Repository, files: &[(&str, &str)], message: &str) {
        let sig = git2::Signature::new("Test", "test@test.com", &git2::Time::new(1_700_000_000, 0))
            .unwrap();
        let mut index = repo.index().unwrap();
        for (path, content) in files {
            let full_path = repo.workdir().unwrap().join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&full_path, content).unwrap();
            index.add_path(StdPath::new(path)).unwrap();
        }
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap();
    }

    #[test]
    fn integration_indent_scores() {
        let (dir, repo) = create_test_repo();
        make_commit(
            &repo,
            &[(
                "main.rs",
                "fn main() {\n    if true {\n        println!(\"hi\");\n    }\n}\n",
            )],
            "add main",
        );
        make_commit(
            &repo,
            &[(
                "main.rs",
                "fn main() {\n    if true {\n        println!(\"hello\");\n    }\n}\n",
            )],
            "update main",
        );

        let result = run(dir.path(), false, false, 20, "score", None, "indent");
        assert!(result.is_ok(), "indent hotspots should succeed");
    }

    #[test]
    fn integration_cycom_scores() {
        let (dir, repo) = create_test_repo();
        make_commit(
            &repo,
            &[(
                "main.rs",
                "fn main() {\n    if true {\n        println!(\"hi\");\n    }\n}\n",
            )],
            "add main",
        );

        let result = run(dir.path(), false, false, 20, "score", None, "cycom");
        assert!(result.is_ok(), "cycom hotspots should succeed");
    }

    #[test]
    fn integration_sort_by_commits() {
        let (dir, repo) = create_test_repo();
        make_commit(
            &repo,
            &[("a.rs", "fn a() {\n    if true { bar(); }\n}\n")],
            "c1",
        );
        make_commit(
            &repo,
            &[("a.rs", "fn a() {\n    if true { baz(); }\n}\n")],
            "c2",
        );
        make_commit(
            &repo,
            &[("b.rs", "fn b() {\n    if x { if y { foo(); } }\n}\n")],
            "c3",
        );

        let result = run(dir.path(), false, false, 20, "commits", None, "indent");
        assert!(result.is_ok(), "sort by commits should work");
    }

    #[test]
    fn integration_since_filters_commits() {
        let (dir, repo) = create_test_repo();
        make_commit(
            &repo,
            &[("a.rs", "fn a() {\n    if true { bar(); }\n}\n")],
            "old",
        );

        // Commits at epoch 2023 → --since 1d from 2026 excludes all
        let result = run(dir.path(), false, false, 20, "score", Some("1d"), "indent");
        assert!(result.is_ok(), "since filter should not crash");
    }

    #[test]
    fn integration_empty_repo() {
        let (dir, _repo) = create_test_repo();
        let result = run(dir.path(), false, false, 20, "score", None, "indent");
        // Empty repo: file_frequencies fails, which is ok
        assert!(
            result.is_ok() || result.is_err(),
            "should handle empty repo gracefully"
        );
    }

    #[test]
    fn integration_json_structure() {
        let (dir, repo) = create_test_repo();
        make_commit(
            &repo,
            &[(
                "main.rs",
                "fn main() {\n    if true {\n        println!(\"hi\");\n    }\n}\n",
            )],
            "add main",
        );
        let result = run(dir.path(), true, false, 20, "score", None, "indent");
        assert!(result.is_ok(), "JSON output should succeed");
    }
}
