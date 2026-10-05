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

use crate::deps::graph::{FileGraph, Related, Source};
use crate::git::FileDiffStat;
use crate::loc::language::detect;
use crate::walk::{TEST_DIRS, is_test_file};

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
];

/// The language of a file, when it has a graph.
pub fn graphed_language(path: &Path) -> Option<&'static str> {
    detect(path)
        .map(|spec| spec.name)
        .filter(|name| crate::deps::is_supported(name))
}

/// Whether the graph of the language of `path` can be trusted for a radius.
pub fn is_reliable(path: &Path) -> bool {
    graphed_language(path).is_some_and(|language| RELIABLE.contains(&language))
}

/// What a file is to the radius.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// Code that ships: it can change, and it can be reached.
    Source,
    /// A test: it protects the sources it refers to.
    Test,
    /// Test support, configuration and scripts. A factory or a case
    /// template refers to every schema; counting it as a test would make
    /// everything look protected.
    Other,
}

fn role(path: &Path) -> Role {
    let in_test_dir = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .any(|c| TEST_DIRS.contains(&c));
    let is_script = graphed_language(path) == Some("Elixir Script");
    if is_test_file(path) {
        Role::Test
    } else if in_test_dir || is_script {
        Role::Other
    } else {
        Role::Source
    }
}

/// Directories that separate sources from their tests without being part
/// of what a file is: `lib/a/b.ex` and `test/a/b_test.exs` are one place.
const LAYOUT_DIRS: &[&str] = &["lib", "src", "test", "tests", "spec", "__tests__"];

/// What marks a file name as a test, around the name of what it tests.
const TEST_AFFIXES: &[&str] = &["_test", ".test", "_spec", ".spec", "Test", "Spec"];

/// The place of a file in its project, whatever side of the source and test
/// layout it is on: its directories without the layout ones, and its name
/// without extension or test mark. A test mirrors a source when both are at
/// the same place. That is how a test protects code it never names, as a
/// controller test that only issues requests.
fn place(path: &Path) -> Vec<String> {
    let mut place: Vec<String> = path
        .parent()
        .into_iter()
        .flat_map(|dir| dir.components())
        .filter_map(|c| c.as_os_str().to_str())
        .filter(|c| !LAYOUT_DIRS.contains(c))
        .map(str::to_string)
        .collect();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let name = TEST_AFFIXES
        .iter()
        .find_map(|affix| stem.strip_suffix(affix))
        .or_else(|| stem.strip_prefix("test_"))
        .unwrap_or(stem);
    place.push(name.to_string());
    place
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
    /// Test files that refer to it, or that mirror its path.
    pub tests: usize,
    /// How many of those tests are part of the change.
    pub tests_in_diff: usize,
}

/// A changed file, and where the change touches it.
pub type Changed<'a> = &'a FileDiffStat;

/// The structural radius of a change.
#[derive(Debug, Default, PartialEq)]
pub struct Structural {
    /// Changed source files the radius starts from.
    pub origins: Vec<PathBuf>,
    /// Public functions the change affects, when every origin tells them.
    /// `None` when some change could not be narrowed to functions.
    pub functions: Option<Vec<String>>,
    /// Source files measured: the ones a file can be reached among.
    pub source_files: usize,
    /// Files that use an origin directly, most exposed first.
    pub direct: Vec<Dependent>,
    /// Files reached at each distance, starting at 1. When the functions
    /// that changed are known, the reach starts at the files the change
    /// concerns and leaves out what only passes through the others.
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

    /// Dependents that call what changed and that no test refers to.
    pub fn unprotected(&self) -> impl Iterator<Item = &Dependent> {
        self.direct
            .iter()
            .filter(|d| d.exposure == Exposure::Calls && d.tests == 0)
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
    tests: Vec<Vec<usize>>,
}

impl Reverse {
    fn of(graph: &FileGraph, roles: &[Role]) -> Self {
        let mut reverse = Self {
            sources: vec![Vec::new(); graph.files.len()],
            tests: vec![Vec::new(); graph.files.len()],
        };
        for (file, uses) in graph.uses.iter().enumerate() {
            let into = match roles[file] {
                Role::Source => &mut reverse.sources,
                Role::Test => &mut reverse.tests,
                Role::Other => continue,
            };
            for used in uses {
                into[used.to].push(file);
            }
        }
        reverse.add_mirrors(graph, roles);
        reverse
    }

    /// Count as a test of each source the test files at the same place.
    fn add_mirrors(&mut self, graph: &FileGraph, roles: &[Role]) {
        let mut tests_at: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
        for (file, path) in graph.files.iter().enumerate() {
            if roles[file] == Role::Test {
                tests_at.entry(place(path)).or_default().push(file);
            }
        }
        for (file, path) in graph.files.iter().enumerate() {
            let mirrors = tests_at.get(&place(path)).into_iter().flatten();
            if roles[file] == Role::Source {
                let tests = &mut self.tests[file];
                tests.extend(mirrors);
                tests.sort_unstable();
                tests.dedup();
            }
        }
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

    /// The reach that starts at `exposed`, the files a change concerns at
    /// distance 1, without going back to the `origins`.
    fn levels_through(&self, origins: &[usize], exposed: &[usize]) -> Vec<Vec<usize>> {
        if exposed.is_empty() {
            return Vec::new();
        }
        let seen = origins.iter().chain(exposed).copied().collect();
        let mut levels = vec![exposed.to_vec()];
        levels.extend(self.levels_beyond(exposed, seen));
        levels
    }
}

/// `Tax.apply` for `Billing.Tax.apply`: the module's own name is enough
/// next to the file it is in.
fn short_call(call: &str) -> String {
    let mut parts: Vec<&str> = call.rsplitn(3, '.').collect();
    parts.truncate(2);
    parts.reverse();
    parts.join(".")
}

/// The function a call names: `apply` in `Billing.Tax.apply`.
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
    let sources: Vec<Source> = sources
        .into_iter()
        .filter_map(|(path, text)| {
            let language = graphed_language(&path)?.to_string();
            Some(Source {
                path,
                language,
                text,
            })
        })
        .collect();
    let known: HashSet<PathBuf> = sources.iter().map(|s| s.path.clone()).collect();
    let graph = FileGraph::build(&sources, &known, None, related);
    let roles: Vec<Role> = graph.files.iter().map(|f| role(f)).collect();
    let reverse = Reverse::of(&graph, &roles);

    let stat_of: HashMap<&Path, Changed> = changed.iter().map(|c| (c.path.as_path(), *c)).collect();
    let in_diff = |file: usize| stat_of.contains_key(graph.files[file].as_path());
    // Each origin with the functions the change affects in it, if known.
    let origins: BTreeMap<usize, Option<BTreeSet<String>>> = (0..graph.files.len())
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
            let (how, telling) = exposure_of(&used.calls, changed.as_ref());
            exposure = exposure.min(how);
            calls.extend(telling.iter().map(|c| short_call(c)));
        }
        let tests = &reverse.tests[file];
        Dependent {
            file: graph.files[file].clone(),
            exposure,
            calls: calls.into_iter().collect(),
            tests: tests.len(),
            tests_in_diff: tests.iter().filter(|&&t| in_diff(t)).count(),
        }
    };
    let mut direct: Vec<(usize, Dependent)> = levels
        .first()
        .into_iter()
        .flatten()
        .map(|file| (*file, dependent(file)))
        .collect();
    direct.sort_by(|(_, a), (_, b)| {
        (a.exposure, a.tests > 0, &a.file).cmp(&(b.exposure, b.tests > 0, &b.file))
    });

    let functions: Option<BTreeSet<String>> = origins
        .values()
        .cloned()
        .try_fold(BTreeSet::new(), |mut all, f| {
            all.extend(f?);
            Some(all)
        })
        .filter(|_| !origins.is_empty());

    // With the functions known, the radius goes through the files the
    // change concerns; a file that calls elsewhere carries it no further.
    let exposed: Vec<usize> = direct
        .iter()
        .filter(|(_, d)| d.exposure != Exposure::Elsewhere)
        .map(|(file, _)| *file)
        .collect();
    let through = match functions {
        Some(_) => reverse.levels_through(&starts, &exposed),
        None => levels.clone(),
    };

    Structural {
        origins: starts.iter().map(|&f| graph.files[f].clone()).collect(),
        functions: functions.map(|f| f.into_iter().collect()),
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

const TITLE: &str = "Structural radius — source files that use what changed";

fn names(paths: impl Iterator<Item = impl AsRef<Path>>, limit: usize) -> String {
    let all: Vec<String> = paths.map(|p| p.as_ref().display().to_string()).collect();
    let shown = all[..all.len().min(limit)].join(", ");
    match all.len().saturating_sub(limit) {
        0 => shown,
        more => format!("{shown} and {more} more"),
    }
}

/// The exposed dependents, most exposed first, each with what exposes it.
fn dependent_lines(radius: &Structural, top: usize) -> Vec<String> {
    let exposed: Vec<&Dependent> = radius.exposed().collect();
    let mut lines = vec![" Tests  Dependent".to_string()];
    for dependent in exposed.iter().take(top) {
        let tests = match dependent.tests {
            0 => "none".to_string(),
            n => n.to_string(),
        };
        lines.push(format!(" {tests:>5}  {}", dependent.file.display()));
        if dependent.exposure == Exposure::Refers {
            lines.push("            refers to the module without calling it".to_string());
        } else if !dependent.calls.is_empty() {
            lines.push(format!(
                "            calls {}",
                names(dependent.calls.iter(), 3)
            ));
        }
    }
    if exposed.len() > top {
        lines.push(format!(" {} files ({top} shown).", exposed.len()));
    }
    lines
}

/// How far the change reaches when every use of a changed file counts.
fn reach_line(radius: &Structural) -> String {
    let near: Vec<String> = radius
        .by_distance
        .iter()
        .take(3)
        .enumerate()
        .map(|(i, files)| format!("{} at distance {}", files, i + 1))
        .collect();
    let further: usize = radius.by_distance.iter().skip(3).sum();
    match further {
        0 => near.join(", "),
        _ => format!(
            "{}, {further} further (up to distance {})",
            near.join(", "),
            radius.by_distance.len()
        ),
    }
}

/// The lines that sum the radius up, above the list of dependents.
fn summary(radius: &Structural) -> Vec<String> {
    let total = radius.source_files.max(1) as f64;
    // A share too small to round to 1% is still not zero.
    let share = |files: usize| match 100.0 * files as f64 / total {
        pct if pct > 0.0 && pct < 0.5 => "<1%".to_string(),
        pct => format!("{pct:.0}%"),
    };
    let reached = radius.reached();
    let changed = format!(" Changed: {}", names(radius.origins.iter(), 5));
    let unprotected = radius.unprotected().count();
    let Some(functions) = &radius.functions else {
        return vec![
            format!(
                " {reached} of {} source files reached ({}), {} direct, {unprotected} of them with no test",
                radius.source_files,
                share(reached),
                radius.direct.len()
            ),
            changed,
            " The change touches code outside functions: every use of the file counts.".to_string(),
            format!(" Reach: {}", reach_line(radius)),
        ];
    };
    let calling = radius
        .direct
        .iter()
        .filter(|d| d.exposure == Exposure::Calls)
        .count();
    let radius_line = match reached {
        0 => format!(" Radius: 0 of {} source files", radius.source_files),
        _ => format!(
            " Radius: {reached} of {} source files ({}): {}",
            radius.source_files,
            share(reached),
            reach_line(radius)
        ),
    };
    vec![
        format!(
            " {}, {unprotected} of them with no test",
            match calling {
                1 => "1 file calls what changed".to_string(),
                n => format!("{n} files call what changed"),
            }
        ),
        changed,
        format!(" Functions: {}", names(functions.iter(), 8)),
        radius_line,
        format!(
            " Upper bound, whatever the function: {} files ({})",
            radius.upper_bound,
            share(radius.upper_bound)
        ),
    ]
}

/// What the radius comes to.
fn body(radius: &Structural, top: usize) -> Vec<String> {
    if radius.origins.is_empty() {
        return vec![
            " No changed source file in a language with a reliable graph".to_string(),
            " (Elixir, JavaScript/TypeScript, Kaikai).".to_string(),
        ];
    }
    if radius.direct.is_empty() {
        return vec![
            format!(" 0 of {} source files reached.", radius.source_files),
            format!(" Changed: {}", names(radius.origins.iter(), 5)),
            " No source file measured uses what changed.".to_string(),
        ];
    }
    let mut lines = summary(radius);
    if radius.exposed().next().is_none() {
        lines.push(" No other file calls the functions that changed.".to_string());
        return lines;
    }
    lines.push(String::new());
    lines.extend(dependent_lines(radius, top));
    lines
}

/// The warnings the radius is measured for.
fn warnings(radius: &Structural) -> Vec<String> {
    let mut lines = Vec::new();
    let unprotected: Vec<&Path> = radius.unprotected().map(|d| d.file.as_path()).collect();
    if !unprotected.is_empty() {
        lines.push(format!(
            "No test refers to {} of the files that call what changed; an integration test is probably missing:",
            unprotected.len()
        ));
        lines.extend(
            unprotected
                .iter()
                .take(5)
                .map(|p| format!("  {}", p.display())),
        );
        if unprotected.len() > 5 {
            lines.push(format!("  and {} more", unprotected.len() - 5));
        }
    }
    let exercised = radius.exposed().any(|d| d.tests_in_diff > 0);
    if radius.exposed().next().is_some() && !exercised {
        lines.push("No test in the change exercises a file that uses what changed.".to_string());
    }
    let unknown = radius
        .direct
        .iter()
        .filter(|d| d.exposure == Exposure::Refers && d.tests == 0)
        .count();
    if unknown > 0 {
        lines.push(format!(
            "{unknown} more with no test use the module without a call that tells whether the change concerns them."
        ));
    }
    lines.extend(radius.unavailable.iter().map(|(language, files)| {
        format!(
            "Not measured for {language} ({files} changed): its graph does not reflect usage yet."
        )
    }));
    if radius.ambiguous > 0 {
        lines.push(format!(
            "Module references left out for naming more than one file: {}",
            radius.ambiguous
        ));
    }
    lines
}

/// The structural block of the report.
pub fn render(radius: &Structural, top: usize) -> Vec<String> {
    let sep = crate::report_helpers::separator(78);
    let mut lines = vec![TITLE.to_string(), sep.clone()];
    lines.extend(body(radius, top));
    lines.push(sep);
    lines.extend(warnings(radius));
    lines
}

#[derive(Serialize)]
struct JsonUnavailable<'a> {
    language: &'a str,
    files: usize,
}

/// The radius as a number: the files the change concerns, over those measured.
#[derive(Serialize)]
struct JsonRadius {
    files: usize,
    source_files: usize,
    /// `files / source_files`, from 0 to 1.
    share: f64,
}

#[derive(Serialize)]
struct JsonReach<'a> {
    distance: usize,
    files: &'a [PathBuf],
}

/// The structural block of the JSON report.
#[derive(Serialize)]
pub struct JsonStructural<'a> {
    source_files: usize,
    origins: Vec<String>,
    /// Public functions the change affects; null when it cannot be narrowed.
    functions: Option<&'a [String]>,
    radius: JsonRadius,
    reached: usize,
    by_distance: &'a [usize],
    /// The files of the radius, distance by distance.
    reach: Vec<JsonReach<'a>>,
    /// Files reached if every use of a changed file counted.
    upper_bound: usize,
    direct: &'a [Dependent],
    /// Files that call what changed and that no test protects.
    unprotected: Vec<String>,
    /// Files that use the changed module without a call that tells whether
    /// the change concerns them, and that no test protects.
    unknown_without_tests: Vec<String>,
    /// Whether a test that is part of the change protects a file that uses
    /// what changed.
    change_tests_a_dependent: bool,
    unavailable: Vec<JsonUnavailable<'a>>,
    ambiguous: usize,
}

impl<'a> From<&'a Structural> for JsonStructural<'a> {
    fn from(radius: &'a Structural) -> Self {
        let text = |p: &PathBuf| p.display().to_string();
        let reached = radius.reached();
        let share = reached as f64 / radius.source_files.max(1) as f64;
        Self {
            source_files: radius.source_files,
            origins: radius.origins.iter().map(text).collect(),
            functions: radius.functions.as_deref(),
            radius: JsonRadius {
                files: reached,
                source_files: radius.source_files,
                share: (share * 10_000.0).round() / 10_000.0,
            },
            reached,
            by_distance: &radius.by_distance,
            reach: radius
                .reached_files
                .iter()
                .enumerate()
                .map(|(i, files)| JsonReach {
                    distance: i + 1,
                    files,
                })
                .collect(),
            upper_bound: radius.upper_bound,
            direct: &radius.direct,
            unprotected: radius.unprotected().map(|d| text(&d.file)).collect(),
            unknown_without_tests: radius
                .direct
                .iter()
                .filter(|d| d.exposure == Exposure::Refers && d.tests == 0)
                .map(|d| text(&d.file))
                .collect(),
            change_tests_a_dependent: radius.exposed().any(|d| d.tests_in_diff > 0),
            unavailable: radius
                .unavailable
                .iter()
                .map(|(language, files)| JsonUnavailable {
                    language,
                    files: *files,
                })
                .collect(),
            ambiguous: radius.ambiguous,
        }
    }
}

#[cfg(test)]
#[path = "structural_test.rs"]
mod tests;
