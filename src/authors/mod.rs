/// Author summary analysis — who owns what across the codebase.
///
/// Walks source files, runs git blame on each, and aggregates per-author:
/// files owned (primary contributor), total lines, languages, last active date.
pub mod analyzer;
mod report;

use std::error::Error;

use crate::cli::OutputMode;
use crate::git::GitRepo;
use crate::util::parse_since;
use crate::walk::{self, WalkConfig};
use analyzer::compute_authors;
use report::{print_json, print_report, print_short, print_terse};

/// Run author summary analysis and print results.
pub fn run(
    cfg: &WalkConfig<'_>,
    output: OutputMode,
    since: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let git = GitRepo::open(cfg.path)
        .map_err(|e| format!("not a git repository (or any parent): {e}"))?;

    let since_ts = since.map(parse_since).transpose()?;

    let (walk_root, prefix) = git.walk_prefix(cfg.path)?;

    // Collect all (language, blames) pairs across source files.
    let mut file_blames: Vec<(String, Vec<crate::git::BlameInfo>)> = Vec::new();

    for (file_path, spec) in walk::source_files(&walk_root, cfg.exclude_tests(), cfg.filter) {
        let rel = GitRepo::to_git_path(&walk_root, &prefix, &file_path);
        let mut blames = match git.blame_file(&rel) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("warning: blame {}: {e}", rel.display());
                continue;
            }
        };

        // Filter to lines last touched since the cutoff.
        if let Some(ts) = since_ts {
            blames.retain(|b| b.last_commit_time >= ts);
            if blames.is_empty() {
                continue;
            }
        }

        file_blames.push((spec.name.to_string(), blames));
    }

    if file_blames.is_empty() {
        if output == OutputMode::Json {
            println!("[]");
        } else {
            println!("No authors found.");
        }
        return Ok(());
    }

    let refs: Vec<(&str, &[crate::git::BlameInfo])> = file_blames
        .iter()
        .map(|(lang, blames)| (lang.as_str(), blames.as_slice()))
        .collect();

    let authors = compute_authors(&refs);

    match output {
        OutputMode::Json => print_json(&authors),
        OutputMode::Short => print_short(&authors),
        OutputMode::Terse => print_terse(&authors),
        OutputMode::Github => {
            return Err("--format github is only supported by cycom, cogcom, and smells".into());
        }
        OutputMode::Table => print_report(&authors),
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
