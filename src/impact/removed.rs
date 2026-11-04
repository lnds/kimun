//! What a change removes, and who still uses it once the change is made:
//! public functions gone from a file that remains, and files deleted.
//!
//! A caller left behind breaks at run time, or when another project is
//! built, and nothing else in the report points at it.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::deps::graph::{FileGraph, Narrowed, Related, Source, Unnarrowed};
use crate::deps::{elixir_functions, elixir_modules, python_names};
use crate::git::{ChangeKind, FileDiffStat};

use super::protection::{Protection, Tests};
use super::reading::{self, graphed_language};
use super::role::{Role, roles_of};
use super::structural::{function_of, short_call};

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

/// What a change to `source` comes to, with the public functions it
/// removes counted among those it changes, and those functions apart.
pub fn told(
    source: &Source,
    stat: &FileDiffStat,
) -> (Result<Narrowed, Unnarrowed>, BTreeSet<String>) {
    let removed = functions_removed(&source.language, stat, &source.text);
    let mut told = source.changed_functions(stat.lines_in(&source.text));
    if let Ok(narrowed) = &mut told {
        narrowed.functions.extend(removed.iter().cloned());
    }
    (told, removed)
}

/// The deleted files in a graph. Each is there only for the files that
/// still use it: no one's user, no test, and not measured.
pub struct Deleted {
    pub all: HashSet<usize>,
    /// Those that were sources: the origins among them.
    pub sources: HashSet<usize>,
}

impl Deleted {
    /// Read `sources` into a graph, with the files `changes` deletes as they
    /// were, and the role of each file.
    pub fn read(
        sources: Vec<(PathBuf, String)>,
        changes: &[&FileDiffStat],
        related: Related,
    ) -> (Vec<Source>, FileGraph, Vec<Role>, Deleted) {
        let deleted = deleted_files(changes, &sources);
        let paths: HashSet<PathBuf> = deleted.iter().map(|(p, _)| p.clone()).collect();
        let (sources, graph) = reading::read(sources.into_iter().chain(deleted).collect(), related);
        let mut roles = roles_of(&graph);
        let deleted = Deleted::mark(&graph, &mut roles, &paths);
        (sources, graph, roles, deleted)
    }

    /// Find the files of `paths` in `graph`, and make each `Role::Other`.
    fn mark(graph: &FileGraph, roles: &mut [Role], paths: &HashSet<PathBuf>) -> Self {
        let all: HashSet<usize> = (0..graph.files.len())
            .filter(|&f| paths.contains(&graph.files[f]))
            .collect();
        let sources = all
            .iter()
            .copied()
            .filter(|&f| roles[f] == Role::Source)
            .collect();
        for &f in &all {
            roles[f] = Role::Other;
        }
        Deleted { all, sources }
    }
}

/// The sources that still use `origin`, or `function` of it, with the
/// calls that tell. `users` holds, for each file, the sources that use it.
fn callers(
    graph: &FileGraph,
    users: &[Vec<usize>],
    tests: &Tests,
    origin: usize,
    function: Option<&str>,
) -> Vec<Caller> {
    let mut callers: Vec<Caller> = users[origin]
        .iter()
        .filter_map(|&user| {
            let used = graph.uses_of(user).find(|u| u.to == origin)?;
            let calls: Vec<String> = used
                .calls
                .iter()
                .filter(|c| function.is_none_or(|name| function_of(c) == name))
                .map(|c| short_call(c))
                .collect();
            (function.is_none() || !calls.is_empty()).then(|| Caller {
                file: graph.files[user].clone(),
                calls,
                protection: tests.protection(user, &users[user]),
            })
        })
        .collect();
    callers.sort_by(|a, b| a.file.cmp(&b.file));
    callers
}

/// What the change removes, each with the sources that still use it: the
/// `deleted` files, and the functions `gone` from each file that remains.
pub fn still_used(
    graph: &FileGraph,
    users: &[Vec<usize>],
    tests: &Tests,
    deleted: &HashSet<usize>,
    gone: &BTreeMap<usize, BTreeSet<String>>,
) -> Vec<Removed> {
    let whole = deleted.iter().map(|&f| (f, None));
    let functions = gone
        .iter()
        .flat_map(|(&f, names)| names.iter().map(move |n| (f, Some(n.as_str()))));
    let mut removed: Vec<Removed> = whole
        .chain(functions)
        .map(|(f, function)| Removed {
            file: graph.files[f].clone(),
            function: function.map(str::to_string),
            callers: callers(graph, users, tests, f, function),
        })
        .collect();
    removed.sort_by(|a, b| (&a.file, &a.function).cmp(&(&b.file, &b.function)));
    removed
}

#[cfg(test)]
#[path = "removed_test.rs"]
mod tests;
