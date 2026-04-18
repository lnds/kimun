//! Dependency graph analysis: internal module coupling via import parsing.
//!
//! Walks source files, extracts import/use/require statements per language,
//! resolves them to project-relative paths, and builds a directed graph.
//! Reports fan-in (how many files import this), fan-out (how many files this
//! imports), and detects dependency cycles using Tarjan's SCC algorithm.

mod analyzer;
mod extractor;
mod report;

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::walk::{self, WalkConfig};

use analyzer::{DepEntry, DepResult, build_graph, resolve_import};
use extractor::extract_imports;

/// Try to read the Go module name from `go.mod` in the project root.
fn detect_go_module(root: &Path) -> Option<String> {
    let content = std::fs::read_to_string(root.join("go.mod")).ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("module ") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Run dependency graph analysis: walk files, extract imports, build graph, output.
pub fn run(
    cfg: &WalkConfig<'_>,
    output: crate::cli::OutputMode,
    cycles_only: bool,
    sort_by: &str,
    top: usize,
) -> Result<(), Box<dyn Error>> {
    let go_module = detect_go_module(cfg.path);

    // Collect all source files with their language
    let all_files: Vec<(PathBuf, String)> =
        walk::source_files(cfg.path, cfg.exclude_tests(), cfg.filter)
            .into_iter()
            .map(|(p, spec)| {
                let rel = p.strip_prefix(cfg.path).unwrap_or(&p).to_path_buf();
                (rel, spec.name.to_string())
            })
            .collect();

    // Build a set of known project-relative paths for fast lookup during resolution
    let file_set: HashSet<PathBuf> = all_files.iter().map(|(p, _)| p.clone()).collect();

    // For each file, read content and extract + resolve imports
    let mut edges: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    for (rel_path, language) in &all_files {
        let abs_path = cfg.path.join(rel_path);
        let source = match std::fs::read_to_string(&abs_path) {
            Ok(s) => s,
            Err(_) => {
                edges.entry(rel_path.clone()).or_default();
                continue;
            }
        };

        let raw_imports = extract_imports(rel_path, language, &source);
        let resolved: Vec<PathBuf> = raw_imports
            .iter()
            .filter_map(|imp| {
                resolve_import(rel_path, imp, language, &file_set, go_module.as_deref())
            })
            .collect();

        // Dedup (same file can be imported multiple times)
        let mut deduped = resolved;
        deduped.sort();
        deduped.dedup();

        edges.insert(rel_path.clone(), deduped);
    }

    // Ensure every file has an entry (even with no imports)
    for (path, _) in &all_files {
        edges.entry(path.clone()).or_default();
    }

    let mut result = build_graph(&all_files, &edges);

    // Apply sort
    match sort_by {
        "fan-in" => result.entries.sort_by_key(|e| Reverse(e.fan_in)),
        "fan-out" => result.entries.sort_by_key(|e| Reverse(e.fan_out)),
        _ => {
            // Default: fan-out descending, then fan-in descending
            result.entries.sort_by(|a, b| {
                b.fan_out
                    .cmp(&a.fan_out)
                    .then_with(|| b.fan_in.cmp(&a.fan_in))
            });
        }
    }

    // Filter to cycles-only if requested
    let entries: Vec<&DepEntry> = if cycles_only {
        result.entries.iter().filter(|e| e.in_cycle).collect()
    } else {
        result.entries.iter().take(top).collect()
    };

    match output {
        crate::cli::OutputMode::Json => {
            let filtered = DepResult {
                entries: entries.iter().map(|e| (*e).clone()).collect(),
                cycles: result.cycles.clone(),
            };
            report::print_json(&filtered)
        }
        crate::cli::OutputMode::Short => {
            report::print_short(&result);
            Ok(())
        }
        crate::cli::OutputMode::Terse => {
            report::print_terse(&result);
            Ok(())
        }
        crate::cli::OutputMode::Github | crate::cli::OutputMode::Codeclimate => {
            Err(crate::cli::ERR_CI_FORMAT_ONLY.into())
        }
        crate::cli::OutputMode::Table => {
            let entries_vec: Vec<DepEntry> = entries.into_iter().cloned().collect();
            report::print_report(&entries_vec, &result);
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
