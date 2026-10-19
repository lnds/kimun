//! The radius past the files that call what changed, followed through
//! functions: a file is reached when one of its public functions calls a
//! function already reached, and only those functions carry the radius on.
//! Where functions are not known (another language, an `import` without
//! `only:`, a `use`, a module named without a call, a call outside every
//! function), the whole file counts
//! and every user of it goes on, as file by file.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::deps::Calls;
use crate::deps::graph::{FileGraph, Related, Source};

/// What of a file is reached.
#[derive(Debug, Clone, PartialEq)]
pub enum Reach {
    /// Any of its functions, or functions are not known.
    All,
    /// These public functions, by name.
    Functions(BTreeSet<String>),
}

/// `Module.function` split at its last dot.
fn split(call: &str) -> Option<(&str, &str)> {
    call.rsplit_once('.')
}

/// The levels of the radius, from distance 1, and whether `max_distance`
/// cut it short.
pub struct Levels {
    pub levels: Vec<Vec<usize>>,
    pub cut: bool,
}

pub struct Walk<'a> {
    graph: &'a FileGraph,
    /// The calls of each file's public functions, where told.
    calls: Vec<Option<Calls>>,
    /// For each file, the sources that use it.
    users: &'a [Vec<usize>],
    related: Related<'a>,
}

impl<'a> Walk<'a> {
    pub fn new(
        graph: &'a FileGraph,
        sources: &[Source],
        users: &'a [Vec<usize>],
        related: Related<'a>,
    ) -> Walk<'a> {
        let calls = sources.iter().map(Source::function_calls).collect();
        Walk {
            graph,
            calls,
            users,
            related,
        }
    }

    /// What of `file` is reached through what `reached` holds.
    fn reach_of(&self, file: usize, reached: &HashMap<usize, Reach>) -> Option<Reach> {
        let own = self.calls[file].as_ref();
        let mut functions = BTreeSet::new();
        for used in self.graph.uses_of(file).filter(|u| u.to != file) {
            let Some(target) = reached.get(&used.to) else {
                continue;
            };
            // A module named without a call (a router listing controllers,
            // a struct) tells no function: the whole file counts.
            let refers = used.calls.is_empty();
            let (Some(own), Reach::Functions(names), false) = (own, target, used.whole || refers)
            else {
                return Some(Reach::All);
            };
            let hits = |call: &String| {
                split(call).is_some_and(|(module, function)| {
                    names.contains(function)
                        && self.graph.defining(module, file, self.related) == Some(used.to)
                })
            };
            let found: Vec<&String> = own
                .iter()
                .filter(|(_, calls)| calls.iter().any(hits))
                .map(|(name, _)| name)
                .collect();
            // A reached function called outside every function: the module
            // body runs it, and nothing tells which functions depend on it.
            let elsewhere = used
                .calls
                .iter()
                .any(|c| split(c).is_some_and(|(_, f)| names.contains(f)));
            if found.is_empty() && elsewhere {
                return Some(Reach::All);
            }
            functions.extend(found.into_iter().cloned());
        }
        (!functions.is_empty()).then_some(Reach::Functions(functions))
    }

    /// The radius from `origins` (each with the functions the change
    /// affects in it, or `None` when not narrowed) through `calling`, the
    /// files at distance 1, at most `max_distance` levels.
    pub fn levels(
        &self,
        origins: &BTreeMap<usize, Option<BTreeSet<String>>>,
        calling: &[usize],
        max_distance: Option<usize>,
    ) -> Levels {
        let mut reached: HashMap<usize, Reach> = origins
            .iter()
            .map(|(&file, told)| {
                let reach = told.clone().map_or(Reach::All, Reach::Functions);
                (file, reach)
            })
            .collect();
        // The files that call what changed are at distance 1 whatever
        // their functions tell: their exposure already says so.
        let mut found: Vec<(usize, Reach)> = calling
            .iter()
            .filter(|f| !reached.contains_key(f))
            .map(|&f| (f, self.reach_of(f, &reached).unwrap_or(Reach::All)))
            .collect();
        let mut levels = Vec::new();
        loop {
            if found.is_empty() {
                return Levels { levels, cut: false };
            }
            if max_distance.is_some_and(|max| levels.len() == max) {
                return Levels { levels, cut: true };
            }
            let mut level: Vec<usize> = found.iter().map(|(f, _)| *f).collect();
            level.sort_unstable();
            reached.extend(found);
            let mut users: Vec<usize> = level
                .iter()
                .flat_map(|&f| &self.users[f])
                .copied()
                .collect();
            users.sort_unstable();
            users.dedup();
            found = users
                .into_iter()
                .filter(|f| !reached.contains_key(f))
                .filter_map(|f| Some((f, self.reach_of(f, &reached)?)))
                .collect();
            levels.push(level);
        }
    }
}

#[cfg(test)]
#[path = "function_reach_test.rs"]
mod tests;
