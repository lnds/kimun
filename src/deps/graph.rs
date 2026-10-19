//! The dependency graph between files, built from sources given as text, so
//! they can come from disk or from the tree of a commit.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use super::analyzer::resolve_import;
use super::apart::Apart;
use super::elixir::{self, ElixirFile};
use super::elixir_using;
use super::extractor::extract_imports;
use super::javascript;
use super::layout::Layout;
use super::python_names;
use super::rust_crates;

/// A source file to place in the graph.
pub struct Source {
    /// Path relative to the root the graph is built for.
    pub path: PathBuf,
    /// Language name, as `LanguageSpec` gives it.
    pub language: String,
    pub text: String,
    /// Templates the file renders that are kept apart from it.
    pub templates: Vec<String>,
}

/// How an import makes one file depend on another when the program runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum EdgeKind {
    /// The importer needs the file to compile: an Elixir `use`, or a macro
    /// called outside a `quote` block.
    Compile,
    /// The importer loads the file when it loads itself.
    #[default]
    Load,
    /// The file is loaded later, when the code that imports it runs: a
    /// Python import inside a function, a dynamic `import()`.
    Deferred,
    /// Only types are taken, which the compiler erases: a TypeScript
    /// `import type`, a Python import under `if TYPE_CHECKING:`.
    Type,
}

impl EdgeKind {
    /// The kind of two imports of the same file: the one that loads soonest.
    pub fn strongest(self, other: Self) -> Self {
        self.min(other)
    }

    /// How the kind is written in JSON.
    pub fn name(self) -> &'static str {
        match self {
            EdgeKind::Compile => "compile",
            EdgeKind::Load => "load",
            EdgeKind::Deferred => "deferred",
            EdgeKind::Type => "type",
        }
    }
}

/// One file's use of another.
#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    /// Index of the file used.
    pub to: usize,
    /// Functions called on it, as `Module.function`, when the language tells.
    pub calls: Vec<String>,
    /// Whether it may reach any function of the file without naming it:
    /// an Elixir `import` without `only:`, a `use`, a protocol implemented.
    pub whole: bool,
    pub kind: EdgeKind,
}

/// What the imports of a source were found to name.
#[derive(Debug, Default)]
pub struct Found {
    /// The files used, each with the functions called there.
    pub uses: BTreeMap<usize, BTreeSet<String>>,
    /// The kind of each use, when the language tells; a load otherwise.
    pub kinds: BTreeMap<usize, EdgeKind>,
    /// Imports that name a file of the project by their form, but resolve
    /// to none: a quoted include, a relative specifier.
    pub unresolved: BTreeSet<String>,
    /// The files used whole, as `Use.whole` tells.
    pub whole: BTreeSet<usize>,
}

impl Found {
    /// Note a use of `to` of the given kind, and give the calls made there.
    pub fn add(&mut self, to: usize, kind: EdgeKind) -> &mut BTreeSet<String> {
        self.kinds
            .entry(to)
            .and_modify(|k| *k = k.strongest(kind))
            .or_insert(kind);
        self.uses.entry(to).or_default()
    }

    /// The uses `file` makes, without those of itself.
    fn uses_from(&self, file: usize) -> Vec<Use> {
        let made = self.uses.iter().filter(|(to, _)| **to != file);
        made.map(|(&to, calls)| Use {
            to,
            calls: calls.iter().cloned().collect(),
            whole: self.whole.contains(&to),
            kind: self.kinds.get(&to).copied().unwrap_or_default(),
        })
        .collect()
    }
}

impl FromIterator<(usize, BTreeSet<String>)> for Found {
    fn from_iter<I: IntoIterator<Item = (usize, BTreeSet<String>)>>(uses: I) -> Self {
        Found::from(uses.into_iter().collect::<BTreeMap<_, _>>())
    }
}

impl From<BTreeMap<usize, BTreeSet<String>>> for Found {
    fn from(uses: BTreeMap<usize, BTreeSet<String>>) -> Self {
        Found {
            uses,
            ..Self::default()
        }
    }
}

/// Why a change to a file cannot be narrowed to functions, so that every
/// use of the file has to count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unnarrowed {
    /// Functions are not told for the language of the file.
    Language,
    /// The lines the change touches are not known.
    NoLines,
    /// The change touches this line, which may concern every function.
    Outside(usize),
}

/// Tells the functions of a source a change to some lines affects, or the
/// line that may concern all of them.
type Teller = fn(&str, &[usize]) -> Result<BTreeSet<String>, usize>;

/// What tells the changed functions of a language, if anything does.
fn teller(language: &str) -> Option<Teller> {
    match language {
        "Python" => Some(python_names::changed),
        "Elixir" | "Elixir Script" => Some(super::elixir_functions::changed_functions),
        _ => None,
    }
}

impl Source {
    /// The public functions of this file a change to `lines` affects, by
    /// name, or why they cannot be told.
    pub fn changed_functions(&self, lines: &[usize]) -> Result<BTreeSet<String>, Unnarrowed> {
        let told = teller(&self.language).ok_or(Unnarrowed::Language)?;
        if lines.is_empty() {
            return Err(Unnarrowed::NoLines);
        }
        told(&self.text, lines).map_err(Unnarrowed::Outside)
    }

    /// For each public function, the functions of other modules it calls,
    /// when the language tells them: Elixir.
    pub fn function_calls(&self) -> Option<super::elixir_calls::Calls> {
        matches!(self.language.as_str(), "Elixir" | "Elixir Script")
            .then(|| super::elixir_calls::calls(&self.text))
    }
}

#[derive(Debug, Default)]
pub struct FileGraph {
    pub files: Vec<PathBuf>,
    /// For each file, the files it uses.
    pub uses: Vec<Vec<Use>>,
    /// For each file, the files it uses without naming them: the sources
    /// behind the headers it includes. Not in `edges`, which is what the
    /// code writes, nor so in its cycles.
    pub implied: Vec<Vec<Use>>,
    /// For each file, the imports that look internal but resolve to no
    /// file.
    pub unresolved: Vec<Vec<String>>,
    /// References left out because several files define the module named
    /// and none is nearer than the others.
    pub ambiguous: usize,
    /// For each file, whether it holds tests of its own, when the language
    /// keeps them beside the code.
    pub own_tests: Vec<bool>,
    modules: ModuleIndex,
}

impl Use {
    /// A use that tells no calls.
    pub fn plain(to: usize) -> Self {
        Use {
            to,
            calls: Vec::new(),
            whole: false,
            kind: EdgeKind::Load,
        }
    }
}

/// A use that tells no calls.
fn no_calls(to: usize) -> (usize, BTreeSet<String>) {
    (to, BTreeSet::new())
}

/// The files a source imports, for the languages whose imports resolve
/// one by one from where the importer is, or are read apart.
fn imports(
    source: &Source,
    layout: &Layout,
    by_path: &HashMap<&Path, usize>,
    apart: &Apart,
) -> Found {
    if let Some(found) = apart.uses(source) {
        return found;
    }
    if matches!(
        source.language.as_str(),
        "JavaScript" | "TypeScript" | "JSX"
    ) {
        return javascript::found(source, layout, by_path);
    }
    let known = &layout.known;
    extract_imports(&source.language, &source.text)
        .iter()
        .flat_map(|import| resolve_import(&source.path, import, &source.language, known))
        .filter_map(|path| by_path.get(path.as_path()).copied())
        .map(no_calls)
        .collect()
}

/// Whether a source enters the graph of Elixir modules: build
/// configuration neither defines nor uses any of them.
fn is_elixir_code(source: &Source) -> bool {
    matches!(source.language.as_str(), "Elixir" | "Elixir Script")
        && !elixir::is_build_config(&source.path)
}

/// How many leading directories two paths share.
pub fn shared_depth(a: &Path, b: &Path) -> usize {
    a.components()
        .zip(b.components())
        .take_while(|(x, y)| x == y)
        .count()
}

/// The one file of `pool` deepest by `depth`, or `Err` when none or
/// several are.
pub fn nearest(pool: &[usize], depth: impl Fn(usize) -> usize) -> Result<usize, ()> {
    let deepest = pool.iter().map(|&f| depth(f)).max().ok_or(())?;
    let mut at_deepest = pool.iter().filter(|&&f| depth(f) == deepest);
    match (at_deepest.next(), at_deepest.next()) {
        (Some(&only), None) => Ok(only),
        _ => Err(()),
    }
}

/// Tells whether the file at the first path may use the one at the second
/// by what their projects declare: same project, or a declared dependency.
pub type Related<'a> = &'a dyn Fn(&Path, &Path) -> bool;

/// Where each Elixir module is defined. Projects of one repository made
/// from the same template may define modules of the same name.
#[derive(Debug, Default)]
struct ModuleIndex(HashMap<String, Vec<usize>>);

impl ModuleIndex {
    /// The file that defines `module` for a reference made from `from`.
    ///
    /// When several do, those the projects declare as usable come first:
    /// the same project, or one it depends on. Among them, or among all
    /// when nothing is declared, the one deepest in the same directories
    /// wins. `Err` when that does not single one out.
    fn resolve(
        &self,
        module: &str,
        from: &Path,
        files: &[PathBuf],
        related: Related,
    ) -> Result<Option<usize>, ()> {
        let Some(candidates) = self.0.get(module) else {
            return Ok(None);
        };
        // What the projects declare comes first: the same project, or one
        // it depends on. Where nothing is declared, every definition counts.
        let declared: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|&c| related(from, &files[c]))
            .collect();
        let pool = if declared.is_empty() {
            candidates
        } else {
            &declared
        };

        nearest(pool, |file| shared_depth(&files[file], from)).map(Some)
    }
}

/// The Elixir files of `sources`, read with what the modules they `use`
/// inject, and where each module is defined.
fn read_elixir(
    sources: &[Source],
    files: &[PathBuf],
    related: Related,
) -> (Vec<Option<ElixirFile>>, ModuleIndex) {
    let mut parsed: Vec<Option<ElixirFile>> = sources
        .iter()
        .map(|s| is_elixir_code(s).then(|| elixir::parse_with(&s.text, &s.templates)))
        .collect();
    let mut modules = ModuleIndex::default();
    for (file, parsed) in parsed.iter().enumerate() {
        for module in parsed.iter().flat_map(|p| &p.defines) {
            modules.0.entry(module.clone()).or_default().push(file);
        }
    }
    let resolve = |from: usize, module: &str| {
        let found = modules.resolve(module, &files[from], files, related);
        found.ok().flatten()
    };
    elixir_using::complete(&mut parsed, sources, &resolve);
    (parsed, modules)
}

impl FileGraph {
    /// Build the graph of `sources`.
    pub fn build(sources: &[Source], layout: &Layout, related: Related) -> Self {
        let files: Vec<PathBuf> = sources.iter().map(|s| s.path.clone()).collect();
        let by_path: HashMap<&Path, usize> = sources
            .iter()
            .enumerate()
            .map(|(i, s)| (s.path.as_path(), i))
            .collect();

        let (parsed, modules) = read_elixir(sources, &files, related);

        let rust: Vec<Option<&str>> = sources
            .iter()
            .map(|s| (s.language == "Rust").then_some(s.text.as_str()))
            .collect();
        let (mut rust_uses, own_tests) = rust_crates::read(&files, &rust, &layout.packages);
        let apart = Apart::of(sources, layout, &by_path);

        let mut graph = FileGraph {
            files,
            modules,
            own_tests,
            ..Self::default()
        };
        let modules = std::mem::take(&mut graph.modules);
        for (file, source) in sources.iter().enumerate() {
            let found = match (&parsed[file], rust_uses[file].take()) {
                (Some(_), _) => graph.elixir_uses(file, &parsed, &modules, related),
                (None, Some(used)) => used.into_iter().map(no_calls).collect(),
                (None, None) => imports(source, layout, &by_path, &apart),
            };
            graph.uses.push(found.uses_from(file));
            graph
                .unresolved
                .push(found.unresolved.into_iter().collect());
        }
        graph.ambiguous += apart.ambiguous();
        graph.implied = apart.implied(sources, &graph.uses);
        graph.modules = modules;
        graph
    }

    /// The file that defines the Elixir `module`, as a reference made from
    /// the file `from` resolves it.
    pub fn defining(&self, module: &str, from: usize, related: Related) -> Option<usize> {
        let resolved = self
            .modules
            .resolve(module, &self.files[from], &self.files, related);
        resolved.ok().flatten()
    }

    /// The files an Elixir file uses, each with the functions it calls
    /// there, and those it may reach whole.
    fn elixir_uses(
        &mut self,
        file: usize,
        parsed: &[Option<ElixirFile>],
        modules: &ModuleIndex,
        related: Related,
    ) -> Found {
        let mut found = Found::default();
        let mut ambiguous = BTreeSet::new();
        let Some(own) = &parsed[file] else {
            return found;
        };
        for reference in &own.refs {
            match modules.resolve(&reference.module, &self.files[file], &self.files, related) {
                Ok(Some(to)) => {
                    found.add(to, EdgeKind::Load).extend(
                        reference
                            .function
                            .iter()
                            .map(|f| format!("{}.{f}", reference.module)),
                    );
                }
                Ok(None) => {}
                Err(()) => {
                    ambiguous.insert(&reference.module);
                }
            }
        }
        self.ambiguous += ambiguous.len();
        let from = &self.files[file];
        let resolved = |m: &String| {
            modules
                .resolve(m, from, &self.files, related)
                .ok()
                .flatten()
        };
        let macros = |to: usize| parsed[to].as_ref().map_or(&[][..], |p| p.macros.as_slice());
        for to in own.compile.needed(&own.whole, &resolved, &macros) {
            found.add(to, EdgeKind::Compile);
        }
        found.whole = own.whole.iter().filter_map(resolved).collect();
        found
    }

    /// What `file` uses, named or implied.
    pub fn uses_of(&self, file: usize) -> impl Iterator<Item = &Use> {
        self.uses[file]
            .iter()
            .chain(self.implied.get(file).into_iter().flatten())
    }

    /// The graph as the paths each file uses.
    #[cfg(test)]
    pub fn edges(&self) -> HashMap<PathBuf, Vec<PathBuf>> {
        self.kinded_edges()
            .into_iter()
            .map(|(file, used)| (file, used.into_iter().map(|(to, _)| to).collect()))
            .collect()
    }

    /// The graph as the paths each file uses, each with the kind of use.
    pub fn kinded_edges(&self) -> HashMap<PathBuf, Vec<(PathBuf, EdgeKind)>> {
        self.files
            .iter()
            .zip(&self.uses)
            .map(|(file, uses)| {
                let used = uses
                    .iter()
                    .map(|u| (self.files[u.to].clone(), u.kind))
                    .collect();
                (file.clone(), used)
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "graph_test.rs"]
mod tests;
