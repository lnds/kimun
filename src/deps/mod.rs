//! Dependency graph analysis: internal module coupling via import parsing.
//!
//! Walks source files, extracts import/use/require statements per language,
//! resolves them to project-relative paths, and builds a directed graph.
//! Reports fan-in (how many files import this), fan-out (how many files this
//! imports), and detects dependency cycles using Tarjan's SCC algorithm.

mod analyzer;
mod elixir;
mod elixir_functions;
mod extractor;
pub mod graph;
pub mod heex;
mod kaikai;
mod phoenix;
mod report;
pub mod routes;

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::walk::{self, WalkConfig};

use analyzer::{DepEntry, DepResult, UnsupportedLanguage, build_graph};
pub use extractor::is_supported;
use graph::{FileGraph, Source};

/// Count the skipped files per language, largest group first.
fn count_unsupported(skipped: &[(PathBuf, String)]) -> Vec<UnsupportedLanguage> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for (_, language) in skipped {
        *counts.entry(language).or_default() += 1;
    }
    let mut unsupported: Vec<UnsupportedLanguage> = counts
        .into_iter()
        .map(|(language, files)| UnsupportedLanguage {
            language: language.to_string(),
            files,
        })
        .collect();
    unsupported.sort_by(|a, b| {
        b.files
            .cmp(&a.files)
            .then_with(|| a.language.cmp(&b.language))
    });
    unsupported
}

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

/// Walk files, extract and resolve their imports, and build the dependency graph.
fn analyze(cfg: &WalkConfig<'_>) -> DepResult {
    let go_module = detect_go_module(cfg.path);

    // Collect all source files with their language. A file whose language has
    // no import extractor stays out of the graph: listing it with zero
    // dependencies would pass an unmeasured file off as a measured one.
    let (all_files, skipped): (Vec<_>, Vec<_>) =
        walk::source_files(cfg.path, cfg.exclude_tests(), cfg.filter)
            .into_iter()
            .map(|(p, spec)| {
                let rel = p.strip_prefix(cfg.path).unwrap_or(&p).to_path_buf();
                (rel, spec.name.to_string())
            })
            .partition(|(_, language)| is_supported(language));
    let unsupported = count_unsupported(&skipped);

    // Build a set of known project-relative paths for fast lookup during resolution.
    // Skipped files stay in it: resolvers look for manifests such as `kai.toml`.
    let file_set: HashSet<PathBuf> = all_files
        .iter()
        .chain(&skipped)
        .map(|(p, _)| p.clone())
        .collect();

    // A file that cannot be read stays in the graph, with nothing to say.
    let sources: Vec<Source> = all_files
        .iter()
        .map(|(rel_path, language)| Source {
            path: rel_path.clone(),
            language: language.clone(),
            text: std::fs::read_to_string(cfg.path.join(rel_path)).unwrap_or_default(),
            templates: Vec::new(),
        })
        .collect();
    // One directory is analysed, with no manifests to tell projects apart.
    let edges = FileGraph::build(&sources, &file_set, go_module.as_deref(), &|_, _| false).edges();

    let mut result = build_graph(&all_files, &edges);
    result.unsupported = unsupported;
    result
}

/// Order entries by the requested key. Any other key means fan-out
/// descending, then fan-in descending.
fn sort_entries(entries: &mut [DepEntry], sort_by: &str) {
    match sort_by {
        "fan-in" => entries.sort_by_key(|e| Reverse(e.fan_in)),
        "fan-out" => entries.sort_by_key(|e| Reverse(e.fan_out)),
        _ => entries.sort_by(|a, b| {
            b.fan_out
                .cmp(&a.fan_out)
                .then_with(|| b.fan_in.cmp(&a.fan_in))
        }),
    }
}

/// The entries to display: every file in a cycle, or else the first `top`.
fn visible_entries(entries: &[DepEntry], cycles_only: bool, top: usize) -> Vec<&DepEntry> {
    if cycles_only {
        entries.iter().filter(|e| e.in_cycle).collect()
    } else {
        entries.iter().take(top).collect()
    }
}

/// Run dependency graph analysis: walk files, extract imports, build graph, output.
pub fn run(
    cfg: &WalkConfig<'_>,
    output: crate::cli::OutputMode,
    cycles_only: bool,
    sort_by: &str,
    top: usize,
) -> Result<(), Box<dyn Error>> {
    let mut result = analyze(cfg);

    sort_entries(&mut result.entries, sort_by);
    let entries = visible_entries(&result.entries, cycles_only, top);

    match output {
        crate::cli::OutputMode::Json => {
            let filtered = DepResult {
                entries: entries.iter().map(|e| (*e).clone()).collect(),
                cycles: result.cycles.clone(),
                unsupported: result.unsupported.clone(),
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
