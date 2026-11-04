//! Copies of changed values: files outside a change that still hold the
//! values it replaced, and not those it brought.
//!
//! Some values are kept in step by hand across projects: a list of country
//! codes, a map of time zones, the states of an order. When one copy
//! changes and the other does not, neither the history nor the imports tell.
//! The values themselves do. Matching is lexical, so it stays a hint.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::git::FileDiffStat;

pub use super::values::literals;

/// Fewest values a changed group must hold before a copy of it is looked
/// for: below this, matches are chance.
const MIN_VALUES: usize = 3;

/// Share of the values before the change a group elsewhere must hold to be
/// taken for a copy.
const MIN_SHARE: f64 = 0.6;

/// Words that, in a comment near a group, say it mirrors another.
const MIRRORS: [&str; 7] = [
    "mirror",
    "copy of",
    "copied from",
    "keep in sync",
    "in sync with",
    "same as",
    "duplicated from",
];

/// A group of values elsewhere that looks like the copy of one a change
/// modifies.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Copy {
    pub file: PathBuf,
    /// First line of the group, from 1.
    pub line: usize,
    /// The changed file whose values it holds, and where they are.
    pub of: PathBuf,
    pub of_line: usize,
    /// Values it shares with the group as it was before the change.
    pub matched: Vec<String>,
    /// Values the change brought that it lacks.
    pub missing: Vec<String>,
    /// Values the change took out that it still holds.
    pub kept: Vec<String>,
    /// Whether a comment beside it names the changed file as what it
    /// mirrors: then it is most likely a copy.
    pub mentions: bool,
}

/// A run of consecutive lines that hold literal values: a list, a map, an
/// enumeration.
#[derive(Debug, PartialEq)]
struct Group {
    first: usize,
    last: usize,
    values: BTreeSet<String>,
}

/// The groups of values of a text, in order. Lines are numbered from 1.
fn groups(text: &str) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let values = literals(line);
        if values.is_empty() {
            continue;
        }
        match groups.last_mut() {
            Some(group) if group.last + 1 == i + 1 => {
                group.last = i + 1;
                group.values.extend(values);
            }
            _ => groups.push(Group {
                first: i + 1,
                last: i + 1,
                values: values.into_iter().collect(),
            }),
        }
    }
    groups
}

/// What a change does to one group of values of a file.
#[derive(Debug, PartialEq)]
pub struct Edit {
    pub file: PathBuf,
    pub line: usize,
    /// The values of the group before the change.
    pub before: BTreeSet<String>,
    /// The values the change brought into it.
    pub added: BTreeSet<String>,
    /// The values the change took out of it.
    pub dropped: BTreeSet<String>,
    /// Names a comment may call the file by: its stem, and the last part of
    /// the modules it defines.
    names: Vec<String>,
}

/// The values of the lines given.
fn values_of(lines: &[String]) -> BTreeSet<String> {
    lines.iter().flat_map(|l| literals(l)).collect()
}

/// The groups of values `change` modifies in `head`, the file after it,
/// each with its values before and after. A group too small to be told
/// apart by its values is left out.
pub fn edits(change: &FileDiffStat, head: &str) -> Vec<Edit> {
    let inserted = values_of(&change.inserted);
    let removed = values_of(&change.removed);
    let brought: BTreeSet<String> = inserted.difference(&removed).cloned().collect();
    let taken: BTreeSet<String> = removed.difference(&inserted).cloned().collect();
    if brought.is_empty() && taken.is_empty() {
        return Vec::new();
    }
    let touched = change.lines_in(head);
    let names = names_of(&change.path, head);
    groups(head)
        .into_iter()
        .filter(|g| touched.iter().any(|&l| (g.first..=g.last + 1).contains(&l)))
        .filter_map(|g| {
            let added: BTreeSet<String> = brought.intersection(&g.values).cloned().collect();
            let mut before: BTreeSet<String> = g.values.difference(&added).cloned().collect();
            let dropped: BTreeSet<String> = taken.difference(&g.values).cloned().collect();
            before.extend(dropped.iter().cloned());
            let changed = !added.is_empty() || !dropped.is_empty();
            (changed && before.len() >= MIN_VALUES).then(|| Edit {
                file: change.path.clone(),
                line: g.first,
                before,
                added,
                dropped,
                names: names.clone(),
            })
        })
        .collect()
}

/// Names a comment may call a file by, in lower case: its stem, and the
/// last part of each module it declares (`defmodule Geo.Countries`).
fn names_of(path: &Path, text: &str) -> Vec<String> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(str::to_lowercase);
    let modules = text.lines().filter_map(|line| {
        let module = line.trim_start().strip_prefix("defmodule ")?;
        let module = module.split_whitespace().next()?;
        Some(module.to_lowercase())
    });
    stem.into_iter()
        .chain(modules)
        .filter(|n| n.chars().count() >= 3)
        .collect()
}

/// Whether the lines around a group, from a few above it to its last,
/// say it mirrors something named in `names`.
fn mentions(lines: &[&str], group: &Group, names: &[String]) -> bool {
    let from = group.first.saturating_sub(4);
    lines[from..group.last.min(lines.len())].iter().any(|line| {
        let line = line.to_lowercase();
        MIRRORS.iter().any(|m| line.contains(m)) && names.iter().any(|n| line.contains(n.as_str()))
    })
}

/// Whether the values of `group` look like the copy of what `edit`
/// changed, left as it was before the change.
fn copy_of(edit: &Edit, group: &Group, file: &Path, lines: &[&str]) -> Option<Copy> {
    let matched: Vec<String> = edit.before.intersection(&group.values).cloned().collect();
    let needed = MIN_VALUES.max((edit.before.len() as f64 * MIN_SHARE).ceil() as usize);
    let missing: Vec<String> = edit.added.difference(&group.values).cloned().collect();
    let kept: Vec<String> = edit.dropped.intersection(&group.values).cloned().collect();
    let behind = !missing.is_empty() || !kept.is_empty();
    // A group that holds what the change brought is the new value already.
    let updated = !edit.added.is_empty() && missing.is_empty();
    (matched.len() >= needed && behind && !updated).then(|| Copy {
        file: file.to_path_buf(),
        line: group.first,
        of: edit.file.clone(),
        of_line: edit.line,
        matched,
        missing,
        kept,
        mentions: mentions(lines, group, &edit.names),
    })
}

/// The copies of what `edits` changed among `files`, the files after the
/// change that it does not touch. Those a comment ties to the changed file
/// come first, then those that share more values.
pub fn copies(edits: &[Edit], files: &[(PathBuf, String)]) -> Vec<Copy> {
    if edits.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<Copy> = files
        .iter()
        .flat_map(|(path, text)| {
            let lines: Vec<&str> = text.lines().collect();
            groups(text)
                .into_iter()
                .flat_map(|group| {
                    edits
                        .iter()
                        .filter_map(|edit| copy_of(edit, &group, path, &lines))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect();
    found.sort_by(|a, b| {
        (
            !a.mentions,
            std::cmp::Reverse(a.matched.len()),
            &a.file,
            a.line,
        )
            .cmp(&(
                !b.mentions,
                std::cmp::Reverse(b.matched.len()),
                &b.file,
                b.line,
            ))
    });
    found
}

#[cfg(test)]
#[path = "copies_test.rs"]
mod tests;
