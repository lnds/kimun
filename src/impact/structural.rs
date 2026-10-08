//! Structural radius: the source files that use what a change touches, and
//! which of them no test exercises.
//!
//! This is the answer to "the tests of the module I changed pass; who else
//! uses it?". It reads the dependency graph backwards from the changed
//! files. A dependent that no test file refers to is unprotected: the change
//! can break it without any test noticing.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::deps::graph::{FileGraph, Related, Unnarrowed};
use crate::git::{ChangeKind, FileDiffStat};

use super::protection::{Protection, Tests};
use super::reading::{self, graphed_language};
use super::role::{self, Role, roles_of};
use super::routes;

/// Languages whose graph reflects what uses what. For the others `deps`
/// reads declarations that do not amount to usage, and a radius drawn on
/// them would be made up.
const RELIABLE: &[&str] = &[
    "Elixir",
    "Elixir Script",
    "JavaScript",
    "TypeScript",
    "JSX",
    "TSX",
    "Kaikai",
    "Rust",
];

/// Whether the graph of the language of `path` can be trusted for a radius.
pub fn is_reliable(path: &Path) -> bool {
    graphed_language(path).is_some_and(|language| RELIABLE.contains(&language))
}

/// How a file that uses a changed one stands to the change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Exposure {
    /// It calls a function that changed, or the change could not be
    /// narrowed to functions and any use counts.
    Calls,
    /// It refers to the changed module without calling it: a struct, an
    /// `import`, a `use`. Whether the change concerns it is not known.
    Refers,
    /// It only calls functions the change leaves alone.
    Elsewhere,
}

/// A source file that uses a changed one directly.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Dependent {
    pub file: PathBuf,
    pub exposure: Exposure,
    /// The calls that expose it: to the functions that changed when those
    /// are known, to the changed files otherwise.
    pub calls: Vec<String>,
    /// How tests protect it.
    pub protection: Protection,
    /// Test files that refer to it, or that mirror its path.
    pub tests: usize,
    /// How many of those tests are part of the change.
    pub tests_in_diff: usize,
    /// Whether it is run rather than used: a command-line task, a script.
    pub entry_point: bool,
    /// Whether a source file uses it.
    #[serde(skip)]
    pub used: bool,
}

/// What a change comes to in one changed file: the public functions it
/// affects, or why they cannot be told and every use of the file counts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Narrowing {
    pub file: PathBuf,
    pub functions: Option<Vec<String>>,
    /// Why not, when `functions` is `None`.
    pub reason: Option<String>,
}

/// Why the change to `source` cannot be narrowed, in words.
fn reason(stat: &FileDiffStat, why: Unnarrowed) -> String {
    match (stat.kind, why) {
        (ChangeKind::Added, _) => "new file".to_string(),
        (_, Unnarrowed::Language) => "its language does not tell functions".to_string(),
        (_, Unnarrowed::NoLines) => "no changed line is known".to_string(),
        (_, Unnarrowed::Outside(line)) => format!("line {line} is outside every function"),
    }
}

/// A changed file, and where the change touches it.
pub type Changed<'a> = &'a FileDiffStat;

/// The structural radius of a change.
#[derive(Debug, Default, PartialEq)]
pub struct Structural {
    /// Changed source files the radius starts from.
    pub origins: Vec<PathBuf>,
    /// Public functions the change affects, over the origins that tell
    /// them. `None` when no origin could be narrowed to functions.
    pub functions: Option<Vec<String>>,
    /// What the change comes to in each origin.
    pub narrowing: Vec<Narrowing>,
    /// Source files measured: the ones a file can be reached among.
    pub source_files: usize,
    /// Files that use an origin directly, most exposed first.
    pub direct: Vec<Dependent>,
    /// Files reached at each distance, starting at 1 with the files that
    /// call what changed. What only passes through a file that calls
    /// elsewhere, or that merely refers to the module, is left out.
    pub by_distance: Vec<usize>,
    /// The files behind `by_distance`, distance by distance.
    pub reached_files: Vec<Vec<PathBuf>>,
    /// Files reached when every use of a changed file counts, whatever the
    /// function: what the radius cannot exceed.
    pub upper_bound: usize,
    /// Changed files in languages whose graph is not reliable, by language.
    pub unavailable: Vec<(String, usize)>,
    /// Elixir module references left out for naming several files.
    pub ambiguous: usize,
}

impl Structural {
    /// Source files reached, at any distance.
    pub fn reached(&self) -> usize {
        self.by_distance.iter().sum()
    }

    /// Direct dependents the change concerns: all but the ones that only
    /// call functions it leaves alone.
    pub fn exposed(&self) -> impl Iterator<Item = &Dependent> {
        self.direct
            .iter()
            .filter(|d| d.exposure != Exposure::Elsewhere)
    }

    /// Dependents that call what changed and that no test reaches.
    fn untested(&self) -> impl Iterator<Item = &Dependent> {
        self.direct
            .iter()
            .filter(|d| d.exposure == Exposure::Calls && d.protection == Protection::None)
    }

    /// Dependents that call what changed and that no test reaches, not even
    /// through the files that use them. Entry points are told apart:
    /// nothing uses them, so no test of something else could reach them.
    pub fn unprotected(&self) -> impl Iterator<Item = &Dependent> {
        self.untested().filter(|d| !d.entry_point)
    }

    /// Entry points that call what changed and have no test.
    pub fn untested_entry_points(&self) -> impl Iterator<Item = &Dependent> {
        self.untested().filter(|d| d.entry_point)
    }

    /// Mark the entry points among the dependents: those `declared` so,
    /// and those at a `conventional` place that no source file uses.
    pub fn mark_entry_points(
        &mut self,
        conventional: impl Fn(&Path) -> bool,
        declared: impl Fn(&Path) -> bool,
    ) {
        for d in &mut self.direct {
            d.entry_point = declared(&d.file) || (!d.used && conventional(&d.file));
        }
    }
}

/// Changed files in languages that have a graph, but not a reliable one.
fn unavailable(changed: &[&Path]) -> Vec<(String, usize)> {
    let mut by_language: BTreeMap<&str, usize> = BTreeMap::new();
    for path in changed {
        if let Some(language) = graphed_language(path).filter(|l| !RELIABLE.contains(l)) {
            *by_language.entry(language).or_default() += 1;
        }
    }
    by_language
        .into_iter()
        .map(|(language, files)| (language.to_string(), files))
        .collect()
}

/// The graph read backwards: for each file, the sources and the tests that
/// use it.
struct Reverse {
    sources: Vec<Vec<usize>>,
    tests: Tests,
}

impl Reverse {
    /// `referring` holds, for each file, the tests known to reach it by
    /// other means than naming it.
    fn of(graph: &FileGraph, roles: &[Role], mut referring: Vec<Vec<usize>>) -> Self {
        let mut sources = vec![Vec::new(); graph.files.len()];
        for (file, uses) in graph.uses.iter().enumerate() {
            let into = match roles[file] {
                Role::Source => &mut sources,
                Role::Test => &mut referring,
                Role::Other => continue,
            };
            for used in uses {
                into[used.to].push(file);
            }
        }
        role::own_tests(graph, roles, &mut referring);
        let tests = Tests::of(
            &graph.files,
            referring,
            |file| roles[file] == Role::Test,
            |file| roles[file] == Role::Source,
        );
        Self { sources, tests }
    }

    /// The sources that use `frontier`, then those that use them, and so
    /// on, each group one step further. Files in `seen` are not reached again.
    fn levels_beyond(&self, frontier: &[usize], mut seen: HashSet<usize>) -> Vec<Vec<usize>> {
        let mut levels = Vec::new();
        let mut frontier = frontier.to_vec();
        loop {
            let mut next: Vec<usize> = frontier
                .iter()
                .flat_map(|&file| &self.sources[file])
                .copied()
                .filter(|file| seen.insert(*file))
                .collect();
            if next.is_empty() {
                return levels;
            }
            next.sort_unstable();
            levels.push(next.clone());
            frontier = next;
        }
    }

    /// The sources reached from `origins`, grouped by distance from 1.
    fn levels(&self, origins: &[usize]) -> Vec<Vec<usize>> {
        self.levels_beyond(origins, origins.iter().copied().collect())
    }

    /// The reach that starts at `calling`, the files that call what changed,
    /// at distance 1, without going back to the `origins`.
    fn levels_through(&self, origins: &[usize], calling: &[usize]) -> Vec<Vec<usize>> {
        if calling.is_empty() {
            return Vec::new();
        }
        let seen = origins.iter().chain(calling).copied().collect();
        let mut levels = vec![calling.to_vec()];
        levels.extend(self.levels_beyond(calling, seen));
        levels
    }
}

/// `Tip.apply` for `Booking.Tip.apply`: the module's own name is enough
/// next to the file it is in.
fn short_call(call: &str) -> String {
    let mut parts: Vec<&str> = call.rsplitn(3, '.').collect();
    parts.truncate(2);
    parts.reverse();
    parts.join(".")
}

/// The function a call names: `apply` in `Booking.Tip.apply`.
fn function_of(call: &str) -> &str {
    call.rsplit('.').next().unwrap_or(call)
}

/// How one file's use of an origin stands to what changed in that origin,
/// and the calls that tell.
fn exposure_of(calls: &[String], changed: Option<&BTreeSet<String>>) -> (Exposure, Vec<String>) {
    let Some(changed) = changed else {
        return (Exposure::Calls, calls.to_vec());
    };
    let hits: Vec<String> = calls
        .iter()
        .filter(|call| changed.contains(function_of(call)))
        .cloned()
        .collect();
    match (hits.is_empty(), calls.is_empty()) {
        (false, _) => (Exposure::Calls, hits),
        (true, true) => (Exposure::Refers, Vec::new()),
        (true, false) => (Exposure::Elsewhere, Vec::new()),
    }
}

/// Measure the structural radius of `changed` over `sources`.
pub fn compute(
    sources: Vec<(PathBuf, String)>,
    changed: &[Changed<'_>],
    related: Related,
) -> Structural {
    let (sources, graph) = reading::read(sources, related);
    let roles = roles_of(&graph);
    let requesting = routes::requesting(&sources, &graph, |f| roles[f] == Role::Test, related);
    let reverse = Reverse::of(&graph, &roles, requesting);

    let stat_of: HashMap<&Path, Changed> = changed.iter().map(|c| (c.path.as_path(), *c)).collect();
    let in_diff = |file: usize| stat_of.contains_key(graph.files[file].as_path());
    // Each origin with the functions the change affects in it, if known.
    let origins: BTreeMap<usize, Result<BTreeSet<String>, Unnarrowed>> = (0..graph.files.len())
        .filter(|&f| roles[f] == Role::Source && in_diff(f))
        .map(|f| {
            let lines = stat_of[graph.files[f].as_path()].lines_in(&sources[f].text);
            (f, sources[f].changed_functions(lines))
        })
        .collect();
    let starts: Vec<usize> = origins.keys().copied().collect();
    let levels = reverse.levels(&starts);

    let dependent = |&file: &usize| {
        let mut exposure = Exposure::Elsewhere;
        let mut calls = BTreeSet::new();
        for used in &graph.uses[file] {
            let Some(changed) = origins.get(&used.to) else {
                continue;
            };
            let (how, telling) = exposure_of(&used.calls, changed.as_ref().ok());
            exposure = exposure.min(how);
            calls.extend(telling.iter().map(|c| short_call(c)));
        }
        let tests = reverse.tests.direct(file);
        Dependent {
            file: graph.files[file].clone(),
            exposure,
            calls: calls.into_iter().collect(),
            protection: reverse.tests.protection(file, &reverse.sources[file]),
            tests: tests.len(),
            tests_in_diff: reverse.tests.own(file).filter(|&t| in_diff(t)).count(),
            entry_point: false,
            used: !reverse.sources[file].is_empty(),
        }
    };
    let mut direct: Vec<(usize, Dependent)> = levels
        .first()
        .into_iter()
        .flatten()
        .map(|file| (*file, dependent(file)))
        .collect();
    direct.sort_by(|(_, a), (_, b)| {
        // Most exposed first, and among those the least protected.
        (a.exposure, std::cmp::Reverse(a.protection), &a.file).cmp(&(
            b.exposure,
            std::cmp::Reverse(b.protection),
            &b.file,
        ))
    });

    let mut narrowing: Vec<Narrowing> = origins
        .iter()
        .map(|(&file, told)| {
            let path = &graph.files[file];
            Narrowing {
                file: path.clone(),
                functions: told.as_ref().ok().map(|f| f.iter().cloned().collect()),
                reason: told
                    .as_ref()
                    .err()
                    .map(|&why| reason(stat_of[path.as_path()], why)),
            }
        })
        .collect();
    narrowing.sort_by(|a, b| a.file.cmp(&b.file));
    let narrowed: Vec<&Vec<String>> = narrowing
        .iter()
        .filter_map(|n| n.functions.as_ref())
        .collect();
    let functions: Option<Vec<String>> = (!narrowed.is_empty()).then(|| {
        let all: BTreeSet<&String> = narrowed.into_iter().flatten().collect();
        all.into_iter().cloned().collect()
    });

    // The radius goes through the files that call what changed. One that
    // only calls functions the change leaves alone carries it no further,
    // and neither does one that merely refers to the module: a schema is
    // named by half a project, and following all of that says nothing.
    // Where an origin could not be narrowed, every use of it is a call.
    let calling: Vec<usize> = direct
        .iter()
        .filter(|(_, d)| d.exposure == Exposure::Calls)
        .map(|(file, _)| *file)
        .collect();
    let through = reverse.levels_through(&starts, &calling);

    Structural {
        origins: narrowing.iter().map(|n| n.file.clone()).collect(),
        functions,
        narrowing,
        source_files: roles.iter().filter(|r| **r == Role::Source).count(),
        direct: direct.into_iter().map(|(_, d)| d).collect(),
        by_distance: through.iter().map(Vec::len).collect(),
        reached_files: through
            .iter()
            .map(|level| level.iter().map(|&f| graph.files[f].clone()).collect())
            .collect(),
        upper_bound: levels.iter().map(Vec::len).sum(),
        unavailable: unavailable(&changed.iter().map(|c| c.path.as_path()).collect::<Vec<_>>()),
        ambiguous: graph.ambiguous,
    }
}

#[cfg(test)]
#[path = "structural_test.rs"]
mod tests;
