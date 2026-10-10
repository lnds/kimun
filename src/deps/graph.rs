//! The dependency graph between files, built from sources given as text, so
//! they can come from disk or from the tree of a commit.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use super::analyzer::resolve_import;
use super::elixir::{self, ElixirFile};
use super::extractor::extract_imports;
use super::layout::Layout;
use super::python;
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

/// One file's use of another.
#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    /// Index of the file used.
    pub to: usize,
    /// Functions called on it, as `Module.function`, when the language tells.
    pub calls: Vec<String>,
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

impl Source {
    /// The public functions of this file a change to `lines` affects, by
    /// name, or why they cannot be told.
    pub fn changed_functions(&self, lines: &[usize]) -> Result<BTreeSet<String>, Unnarrowed> {
        if !is_elixir(&self.language) {
            return Err(Unnarrowed::Language);
        }
        if lines.is_empty() {
            return Err(Unnarrowed::NoLines);
        }
        super::elixir_functions::changed_functions(&self.text, lines).map_err(Unnarrowed::Outside)
    }
}

#[derive(Debug, Default)]
pub struct FileGraph {
    pub files: Vec<PathBuf>,
    /// For each file, the files it uses.
    pub uses: Vec<Vec<Use>>,
    /// References left out because several files define the module named
    /// and none is nearer than the others.
    pub ambiguous: usize,
    /// For each file, whether it holds tests of its own, when the language
    /// keeps them beside the code.
    pub own_tests: Vec<bool>,
    modules: ModuleIndex,
}

/// A use that tells no calls.
fn no_calls(to: usize) -> (usize, BTreeSet<String>) {
    (to, BTreeSet::new())
}

/// The files a source imports, for the languages whose imports resolve
/// one by one from where the importer is.
fn imports(
    source: &Source,
    layout: &Layout,
    by_path: &HashMap<&Path, usize>,
) -> BTreeMap<usize, BTreeSet<String>> {
    let (known, go) = (&layout.known, layout.go_module);
    let resolve = |import: &String| match source.language.as_str() {
        "Python" => python::resolve(&source.path, import, known, &layout.python_roots)
            .into_iter()
            .collect(),
        language => resolve_import(&source.path, import, language, known, go),
    };
    extract_imports(&source.language, &source.text)
        .iter()
        .flat_map(resolve)
        .filter_map(|path| by_path.get(path.as_path()).copied())
        .map(no_calls)
        .collect()
}

fn is_elixir(language: &str) -> bool {
    matches!(language, "Elixir" | "Elixir Script")
}

/// How many leading directories two paths share.
pub fn shared_depth(a: &Path, b: &Path) -> usize {
    a.components()
        .zip(b.components())
        .take_while(|(x, y)| x == y)
        .count()
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

        let depth = |&file: &usize| shared_depth(&files[file], from);
        let nearest = pool.iter().map(depth).max().unwrap_or(0);
        let mut at_nearest = pool.iter().filter(|c| depth(c) == nearest);
        match (at_nearest.next(), at_nearest.next()) {
            (Some(&only), None) => Ok(Some(only)),
            _ => Err(()),
        }
    }
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

        let parsed: Vec<Option<ElixirFile>> = sources
            .iter()
            .map(|s| is_elixir(&s.language).then(|| elixir::parse_with(&s.text, &s.templates)))
            .collect();
        let mut modules = ModuleIndex::default();
        for (file, parsed) in parsed.iter().enumerate() {
            for module in parsed.iter().flat_map(|p| &p.defines) {
                modules.0.entry(module.clone()).or_default().push(file);
            }
        }

        let rust: Vec<Option<&str>> = sources
            .iter()
            .map(|s| (s.language == "Rust").then_some(s.text.as_str()))
            .collect();
        let (mut rust_uses, own_tests) = rust_crates::read(&files, &rust, &layout.packages);

        let mut graph = FileGraph {
            files,
            modules,
            own_tests,
            ..Self::default()
        };
        let modules = std::mem::take(&mut graph.modules);
        for (file, source) in sources.iter().enumerate() {
            let uses = match (&parsed[file], rust_uses[file].take()) {
                (Some(parsed), _) => graph.elixir_uses(file, parsed, &modules, related),
                (None, Some(used)) => used.into_iter().map(no_calls).collect(),
                (None, None) => imports(source, layout, &by_path),
            };
            graph.uses.push(
                uses.into_iter()
                    .filter(|(to, _)| *to != file)
                    .map(|(to, calls)| Use {
                        to,
                        calls: calls.into_iter().collect(),
                    })
                    .collect(),
            );
        }
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

    /// The files an Elixir file uses, each with the functions it calls there.
    fn elixir_uses(
        &mut self,
        file: usize,
        parsed: &ElixirFile,
        modules: &ModuleIndex,
        related: Related,
    ) -> BTreeMap<usize, BTreeSet<String>> {
        let mut uses: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
        let mut ambiguous = BTreeSet::new();
        for reference in &parsed.refs {
            match modules.resolve(&reference.module, &self.files[file], &self.files, related) {
                Ok(Some(to)) => {
                    let calls = uses.entry(to).or_default();
                    calls.extend(
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
        uses
    }

    /// The graph as the paths each file uses.
    pub fn edges(&self) -> HashMap<PathBuf, Vec<PathBuf>> {
        self.files
            .iter()
            .zip(&self.uses)
            .map(|(file, uses)| {
                let used = uses.iter().map(|u| self.files[u.to].clone()).collect();
                (file.clone(), used)
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "graph_test.rs"]
mod tests;
