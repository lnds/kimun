//! What a change removes, and who still uses it once the change is made:
//! public functions gone from a file that remains, and files deleted.
//!
//! A caller left behind breaks at run time, or when another project is
//! built, and nothing else in the report points at it.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::deps::{elixir_functions, elixir_modules, python_names};
use crate::git::{ChangeKind, FileDiffStat};

use super::protection::Protection;
use super::reading::graphed_language;

/// Something the change removes, and the files that still use it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Removed {
    pub file: PathBuf,
    /// The function removed; `None` when the whole file is deleted.
    pub function: Option<String>,
    pub callers: Vec<Caller>,
}

/// A source file that still uses what the change removes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Caller {
    pub file: PathBuf,
    /// The calls to what was removed, when the language tells them.
    pub calls: Vec<String>,
    pub protection: Protection,
}

/// The public names a text defines, for the languages that tell them.
fn public_names(language: &str, text: &str) -> BTreeSet<String> {
    match language {
        "Elixir" | "Elixir Script" => elixir_functions(text)
            .into_iter()
            .filter(|f| f.public)
            .map(|f| f.name)
            .collect(),
        "Python" => python_names(text).into_iter().collect(),
        _ => BTreeSet::new(),
    }
}

/// The public functions the change removes from a file that remains: those
/// defined on the lines it deletes that `head`, the file after it, no
/// longer defines. A function whose definition only changed is still there.
pub fn functions_removed(language: &str, change: &FileDiffStat, head: &str) -> BTreeSet<String> {
    if change.removed.is_empty() || change.kind == ChangeKind::Deleted {
        return BTreeSet::new();
    }
    let gone = public_names(language, &change.removed.join("\n"));
    if gone.is_empty() {
        return gone;
    }
    let kept = public_names(language, head);
    gone.difference(&kept).cloned().collect()
}

/// Whether a source is Elixir, where files are found by the modules they
/// define rather than by path.
fn is_elixir(path: &Path) -> bool {
    matches!(graphed_language(path), Some("Elixir" | "Elixir Script"))
}

/// The files the change deletes, as they were, so that who still uses them
/// can be found in the graph: the ones in a language that has one and that
/// `sources`, the files after the change, do not hold. An Elixir file whose
/// modules are all still defined elsewhere was moved, not removed.
pub fn deleted_files(
    changes: &[&FileDiffStat],
    sources: &[(PathBuf, String)],
) -> Vec<(PathBuf, String)> {
    let present: HashSet<&Path> = sources.iter().map(|(p, _)| p.as_path()).collect();
    let deleted: Vec<(PathBuf, String)> = changes
        .iter()
        .filter(|c| c.kind == ChangeKind::Deleted && !c.removed.is_empty())
        .filter(|c| graphed_language(&c.path).is_some() && !present.contains(c.path.as_path()))
        .map(|c| (c.path.clone(), c.removed.join("\n") + "\n"))
        .collect();
    if !deleted.iter().any(|(path, _)| is_elixir(path)) {
        return deleted;
    }
    let defined: HashSet<String> = sources
        .iter()
        .filter(|(path, _)| is_elixir(path))
        .flat_map(|(_, text)| elixir_modules(text))
        .collect();
    deleted
        .into_iter()
        .filter(|(path, text)| {
            !is_elixir(path)
                || elixir_modules(text)
                    .iter()
                    .any(|module| !defined.contains(module))
        })
        .collect()
}

#[cfg(test)]
#[path = "removed_test.rs"]
mod tests;
