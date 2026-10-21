//! Documents that describe what a change touched and were left behind.
//!
//! A document describes a file when it says so (`covers:` in its front
//! matter, or `[docs] covers` in `.kimun.toml`), when it names the file
//! (the references `km docs refs` reads), or when history says the two
//! change together. A document the change touches is taken as up to date.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use super::analyzer::MissingCoChange;
pub use crate::docs::describes::{Rules, is_document};
use crate::docs::describes::{globs, names, names_of};
use crate::docs::markdown;
use crate::git::{ChangeKind, FileDiffStat};

/// How a document is known to describe a changed file, strongest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Signal {
    /// The document says it covers the file.
    Declared,
    /// The document names the file.
    Referenced,
    /// The document usually changes with the file.
    CoChanged,
}

impl Signal {
    pub fn name(self) -> &'static str {
        match self {
            Signal::Declared => "declared",
            Signal::Referenced => "referenced",
            Signal::CoChanged => "co_changed",
        }
    }
}

/// Why a document describes what changed.
#[derive(Clone, Debug, PartialEq)]
pub enum Why {
    /// A glob the document covers.
    Covers(String),
    /// Where the document names a changed file, and whether the change
    /// deleted or moved that file, which leaves the reference dead.
    Names { line: usize, gone: bool },
    /// Of the commits of the strongest changed file, how many changed the
    /// document too.
    ChangedWith {
        shared: usize,
        commits: usize,
        confidence: f64,
    },
}

/// A document left behind by a change.
#[derive(Clone, Debug, PartialEq)]
pub struct Stale {
    pub document: PathBuf,
    pub signal: Signal,
    /// The changed files it describes, as they are after the change.
    pub describes: Vec<PathBuf>,
    pub why: Why,
}

/// A changed file under one of its names: its path, or the path it had
/// before a move, which the move leaves dead.
struct Named<'a> {
    name: &'a Path,
    path: &'a Path,
    gone: bool,
}

fn named(changes: &[FileDiffStat]) -> Vec<Named<'_>> {
    changes
        .iter()
        .flat_map(|c| {
            let current = Named {
                name: &c.path,
                path: &c.path,
                gone: c.kind == ChangeKind::Deleted,
            };
            let before = c.old_path.as_deref().filter(|old| *old != c.path);
            std::iter::once(current).chain(before.map(|old| Named {
                name: old,
                path: &c.path,
                gone: true,
            }))
        })
        .collect()
}

/// Where `document` names the changed files: each one described, with the
/// first line that names it.
fn referenced(document: &Path, text: &str, changes: &[Named<'_>]) -> Vec<(PathBuf, usize, bool)> {
    let mut found: BTreeMap<PathBuf, (usize, bool)> = BTreeMap::new();
    for reference in markdown::scan(text).references {
        let Some((name, whole)) = names_of(document, &reference) else {
            continue;
        };
        for change in changes.iter().filter(|c| names(&name, whole, c.name)) {
            // A directory above a deleted file is still there.
            let gone = change.gone && change.name.ends_with(&name);
            let entry = found
                .entry(change.path.to_path_buf())
                .or_insert((reference.line, gone));
            entry.1 |= gone;
        }
    }
    found
        .into_iter()
        .map(|(path, (line, gone))| (path, line, gone))
        .collect()
}

/// The documents left behind by `changes`. `documents` are the Markdown
/// documents after the change, relative to the root, with their text;
/// `co_changed` the files history expects to change with it.
pub fn stale(
    documents: &[(PathBuf, String)],
    changes: &[FileDiffStat],
    co_changed: &[MissingCoChange],
    rules: &Rules,
) -> Vec<Stale> {
    let in_change: HashSet<&Path> = changes
        .iter()
        .flat_map(|c| std::iter::once(c.path.as_path()).chain(c.old_path.as_deref()))
        .collect();
    // Documents describe; they are not described.
    let code: Vec<FileDiffStat> = changes
        .iter()
        .filter(|c| !is_document(&c.path))
        .cloned()
        .collect();
    let changed = named(&code);
    let mut found: Vec<Stale> = documents
        .iter()
        .filter(|(document, _)| !in_change.contains(document.as_path()))
        .filter(|(document, _)| !rules.ignores(document))
        .filter_map(|(document, text)| {
            declared(document, text, &code, rules)
                .or_else(|| referenced_stale(document, text, &changed))
        })
        .collect();
    let read: HashSet<PathBuf> = found.iter().map(|s| s.document.clone()).collect();
    found.extend(
        co_changed
            .iter()
            .filter(|m| is_document(&m.path) && !read.contains(&m.path))
            .filter(|m| !rules.ignores(&m.path))
            .map(|m| {
                let best = m.best();
                Stale {
                    document: m.path.clone(),
                    signal: Signal::CoChanged,
                    describes: m.triggers.iter().map(|t| t.path.clone()).collect(),
                    why: Why::ChangedWith {
                        shared: best.shared_commits,
                        commits: best.commits,
                        confidence: best.confidence,
                    },
                }
            }),
    );
    found.sort_by(|a, b| {
        a.signal
            .cmp(&b.signal)
            .then_with(|| a.document.cmp(&b.document))
    });
    found
}

fn declared(document: &Path, text: &str, changes: &[FileDiffStat], rules: &Rules) -> Option<Stale> {
    let covered = rules.covered_by(document, text);
    let mut first = None;
    let mut describes = Vec::new();
    for glob in &covered {
        let set = globs(std::slice::from_ref(glob)).ok()?;
        for change in changes {
            let matched = set.is_match(&change.path)
                || change
                    .old_path
                    .as_deref()
                    .is_some_and(|old| set.is_match(old));
            if matched && !describes.contains(&change.path) {
                first.get_or_insert_with(|| glob.clone());
                describes.push(change.path.clone());
            }
        }
    }
    Some(Stale {
        document: document.to_path_buf(),
        signal: Signal::Declared,
        describes,
        why: Why::Covers(first?),
    })
}

fn referenced_stale(document: &Path, text: &str, changed: &[Named<'_>]) -> Option<Stale> {
    let found = referenced(document, text, changed);
    let (_, line, _) = found.iter().min_by_key(|(_, line, _)| *line)?;
    Some(Stale {
        document: document.to_path_buf(),
        signal: Signal::Referenced,
        describes: found.iter().map(|(path, _, _)| path.clone()).collect(),
        why: Why::Names {
            line: *line,
            gone: found.iter().any(|(_, _, gone)| *gone),
        },
    })
}

#[cfg(test)]
#[path = "documentation_test.rs"]
mod tests;
