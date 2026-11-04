//! The origins of the structural radius: the changed sources, with what
//! the change comes to in each, and the deleted ones.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use crate::deps::graph::{FileGraph, Narrowed, Source, Unnarrowed};

use super::removed::{self, Deleted};
use super::role::Role;
use super::structural::Changed;

/// Each origin with the functions the change affects in it, if known, and
/// the public functions it removes, which count among them.
pub type Origins = BTreeMap<usize, Result<Narrowed, Unnarrowed>>;

/// The origins of the radius: the changed sources and the deleted ones,
/// and what each file that remains has removed.
pub fn origins_of(
    graph: &FileGraph,
    sources: &[Source],
    roles: &[Role],
    deleted: &Deleted,
    stat_of: &HashMap<&Path, Changed>,
) -> (Origins, BTreeMap<usize, BTreeSet<String>>) {
    let mut gone = BTreeMap::new();
    let changed =
        |f: usize| roles[f] == Role::Source && stat_of.contains_key(graph.files[f].as_path());
    let origins = (0..graph.files.len())
        .filter(|&f| changed(f) || deleted.sources.contains(&f))
        .map(|f| {
            if deleted.all.contains(&f) {
                return (f, Err(Unnarrowed::NoLines));
            }
            let (told, removed) = removed::told(&sources[f], stat_of[graph.files[f].as_path()]);
            if !removed.is_empty() {
                gone.insert(f, removed);
            }
            (f, told)
        })
        .collect();
    (origins, gone)
}
