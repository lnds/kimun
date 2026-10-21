//! Documents that describe what a change touched and were left behind.
//!
//! A document describes a file when it says so (`covers:` in its front
//! matter, or `[docs] covers` in `.kimun.toml`), when it names the file
//! (the references `km docs refs` reads), or when history says the two
//! change together. A document the change touches is taken as up to date.

use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};

use super::analyzer::MissingCoChange;
use crate::config::DocsConfig;
use crate::docs::markdown::{self, Form};
use crate::docs::refs::{decode, is_external, path_in, relative};
use crate::git::{ChangeKind, FileDiffStat};

/// Documents that record history as it was and never follow the code.
const RECORDS: [&str; 3] = ["changelog", "changes", "history"];

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

/// What tells which documents describe which files.
pub struct Rules {
    /// Code globs and the documents that cover them, from `[docs] covers`.
    covers: Vec<(String, GlobSet)>,
    ignore: GlobSet,
}

impl Rules {
    pub fn new(config: &DocsConfig) -> Result<Self, Box<dyn Error>> {
        let covers = config
            .covers
            .iter()
            .map(|(code, documents)| Ok((code.clone(), globs(documents.as_slice())?)))
            .collect::<Result<_, Box<dyn Error>>>()?;
        Ok(Self {
            covers,
            ignore: globs(&config.ignore)?,
        })
    }

    /// Whether a document never needs to follow the code: a changelog, or
    /// a path of `[docs] ignore`.
    fn ignores(&self, document: &Path) -> bool {
        let stem = document
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        RECORDS.contains(&stem.as_str()) || self.ignore.is_match(document)
    }

    /// The code globs `document` covers, by its front matter and by
    /// `[docs] covers`.
    fn covered_by(&self, document: &Path, text: &str) -> Vec<String> {
        let mut covered = declared_in(text);
        covered.extend(
            self.covers
                .iter()
                .filter(|(_, documents)| documents.is_match(document))
                .map(|(code, _)| code.clone()),
        );
        covered
    }
}

/// The set of `patterns`, each also as a directory: `lib/billing` covers
/// what is under it.
fn globs(patterns: &[String]) -> Result<GlobSet, Box<dyn Error>> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let trimmed = pattern.trim_end_matches('/');
        for glob in [trimmed.to_string(), format!("{trimmed}/**")] {
            builder.add(Glob::new(&glob).map_err(|e| format!("invalid pattern {pattern}: {e}"))?);
        }
    }
    Ok(builder.build()?)
}

/// The globs of `covers:` in a document's front matter: a list in
/// brackets, a block list, or a single value.
fn declared_in(text: &str) -> Vec<String> {
    let unquote = |s: &str| s.trim().trim_matches(['"', '\'']).to_string();
    let mut globs = Vec::new();
    let mut in_list = false;
    for line in markdown::front_matter(text) {
        if let Some(value) = line.strip_prefix("covers:").map(str::trim) {
            in_list = value.is_empty();
            match value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
                Some(items) => globs.extend(items.split(',').map(unquote)),
                None if !value.is_empty() => globs.push(unquote(value)),
                None => {}
            }
        } else if in_list {
            match line.trim_start().strip_prefix("- ") {
                Some(item) => globs.push(unquote(item)),
                None => in_list = line.trim().is_empty(),
            }
        }
    }
    globs.retain(|glob| !glob.is_empty());
    globs
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

/// The path a reference names, relative to the root, and whether it must
/// be named whole: a link leads to one path, while a path in a code span
/// may be the end of a longer one (`deps/graph.rs`).
fn names_of(document: &Path, reference: &markdown::Reference) -> Option<(PathBuf, bool)> {
    let dir = document.parent().unwrap_or(Path::new(""));
    let from_document = |path: &str| relative(&dir.join(path), Path::new(""));
    match reference.form {
        Form::Link | Form::Image => {
            let target = decode(&reference.target);
            if is_external(&target) {
                return None;
            }
            let path = target.split(['#', '?']).next().filter(|p| !p.is_empty())?;
            Some(match path.strip_prefix('/') {
                Some(rooted) => (relative(Path::new(rooted), Path::new("")), true),
                None => (from_document(path), true),
            })
        }
        Form::Code => {
            let path = path_in(&reference.target)?;
            Some(match path.starts_with("./") || path.starts_with("../") {
                true => (from_document(path), true),
                false => (PathBuf::from(path.trim_end_matches('/')), false),
            })
        }
    }
}

/// Whether a reference to `name` names `changed`: the same path, the end
/// of it for a path in code, or a directory of two or more names above it
/// (a top directory such as `src/` holds too much to describe a file).
fn names(name: &Path, whole: bool, changed: &Path) -> bool {
    name == changed
        || (!whole && changed.ends_with(name))
        || (name.components().count() >= 2 && changed.starts_with(name))
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
    let changed = named(changes);
    let mut found: Vec<Stale> = documents
        .iter()
        .filter(|(document, _)| !in_change.contains(document.as_path()))
        .filter(|(document, _)| !rules.ignores(document))
        .filter_map(|(document, text)| {
            declared(document, text, changes, rules)
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

/// Whether a path is a document whose references can be read.
pub fn is_document(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "md" | "mdx" | "markdown"))
}

#[cfg(test)]
#[path = "documentation_test.rs"]
mod tests;
