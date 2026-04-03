/// Code age analysis — how recently was each file last modified in git?
///
/// Walks source files, resolves each one's last-commit timestamp via git,
/// and classifies as Active / Stale / Frozen based on configurable thresholds.
pub mod analyzer;
mod report;

use std::error::Error;
use std::path::PathBuf;

use chrono::Utc;

use crate::git::GitRepo;
use crate::walk::{self, WalkConfig};
use analyzer::{AgeStatus, AgeThresholds, classify};
use report::{print_json, print_report};

/// Run code age analysis and print results.
///
/// `active_days` / `frozen_days` define the Active/Stale/Frozen boundaries.
/// `status_filter` restricts output to "active", "stale", or "frozen".
/// Files not found in git history (e.g. untracked) are skipped with a warning.
pub fn run(
    cfg: &WalkConfig<'_>,
    json: bool,
    active_days: u64,
    frozen_days: u64,
    sort_by: &str,
    status_filter: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let git = GitRepo::open(cfg.path)
        .map_err(|e| format!("not a git repository (or any parent): {e}"))?;

    let (walk_root, prefix) = git.walk_prefix(cfg.path)?;

    let source_files: Vec<(PathBuf, &'static crate::loc::language::LanguageSpec)> =
        walk::source_files(&walk_root, cfg.exclude_tests(), cfg.filter);

    // Build git-relative paths for the blame query.
    let git_paths: Vec<PathBuf> = source_files
        .iter()
        .map(|(p, _)| GitRepo::to_git_path(&walk_root, &prefix, p))
        .collect();

    let last_modified = git.last_modified_per_file(&git_paths)?;

    if active_days >= frozen_days {
        return Err(format!(
            "--active-days ({active_days}) must be less than --frozen-days ({frozen_days})"
        )
        .into());
    }

    let now = Utc::now().timestamp();
    let thresholds = AgeThresholds {
        active_days,
        frozen_days,
    };

    let mut files: Vec<_> = source_files
        .into_iter()
        .filter_map(|(file_path, spec)| {
            let rel = GitRepo::to_git_path(&walk_root, &prefix, &file_path);
            match last_modified.get(&rel) {
                Some(&ts) => Some(classify(rel, spec.name, ts, now, &thresholds)),
                None => {
                    eprintln!("warning: no git history for {}", rel.display());
                    None
                }
            }
        })
        .collect();

    if let Some(filter) = status_filter {
        let keep = match filter {
            "active" => AgeStatus::Active,
            "stale" => AgeStatus::Stale,
            _ => AgeStatus::Frozen,
        };
        files.retain(|f| f.status == keep);
    }

    match sort_by {
        "status" => files.sort_by_key(|f| (f.status as u8, f.last_modified)),
        "file" => files.sort_by(|a, b| a.path.cmp(&b.path)),
        _ => files.sort_by_key(|f| f.last_modified), // "date" — oldest first
    }

    if json {
        print_json(&files);
    } else {
        print_report(&files, &thresholds);
    }

    Ok(())
}
