/// Dependency graph builder and analyzer.
///
/// Builds a directed graph of internal file dependencies, computes per-file
/// fan-in (importers) and fan-out (imports), and detects cycles using
/// Tarjan's strongly-connected components (SCC) algorithm. Fan-in and
/// fan-out count every import; cycles only those that load the module with
/// the importer or compile with it, since a type-only or deferred import
/// never makes one module wait for another to load.
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::graph::EdgeKind;
use super::{javascript, kaikai};

/// A single file's dependency metrics.
#[derive(Clone, Default, Serialize)]
pub struct DepEntry {
    /// Shown in JSON as `JsonDepEntry` writes it.
    #[serde(skip)]
    pub path: PathBuf,
    pub language: String,
    /// Number of project files that import this file.
    pub fan_in: usize,
    /// Number of project files this file imports.
    pub fan_out: usize,
    /// Of `fan_out`, the files imported only for their types.
    pub type_fan_out: usize,
    /// Of `fan_out`, the files imported only where the code runs later.
    pub deferred_fan_out: usize,
    /// Of `fan_out`, the files needed to compile this one.
    pub compile_fan_out: usize,
    /// True if this file participates in a dependency cycle.
    pub in_cycle: bool,
    /// Imports that look internal but name no file of the project.
    pub unresolved: Vec<String>,
}

/// Files left out of the graph because imports are not extracted for their language.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UnsupportedLanguage {
    pub language: String,
    pub files: usize,
}

/// Full dependency analysis result.
#[derive(Default)]
pub struct DepResult {
    pub entries: Vec<DepEntry>,
    /// Each inner Vec is a set of files forming a cycle (SCC size > 1).
    pub cycles: Vec<Vec<PathBuf>>,
    /// Languages found in the project whose files were not analysed.
    pub unsupported: Vec<UnsupportedLanguage>,
    /// Totals over every file, shown or not.
    pub totals: Totals,
}

/// What the whole graph holds besides its files and cycles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Totals {
    /// Imports that take only types, left out of cycles.
    pub type_edges: usize,
    /// Imports that load their module later, left out of cycles.
    pub deferred_edges: usize,
    /// Uses needed to compile the user, in cycles.
    pub compile_edges: usize,
    /// Imports that look internal but name no file.
    pub unresolved: usize,
    /// Files with at least one of those.
    pub files_with_unresolved: usize,
}

impl Totals {
    pub fn of(entries: &[DepEntry]) -> Self {
        let sum = |f: fn(&DepEntry) -> usize| entries.iter().map(f).sum();
        Totals {
            type_edges: sum(|e| e.type_fan_out),
            deferred_edges: sum(|e| e.deferred_fan_out),
            compile_edges: sum(|e| e.compile_fan_out),
            unresolved: sum(|e| e.unresolved.len()),
            files_with_unresolved: sum(|e| usize::from(!e.unresolved.is_empty())),
        }
    }
}

/// Build the dependency graph from raw edges and compute metrics.
///
/// `edges` maps each file to the list of files it imports.
/// Only files present as keys in `edges` are included as nodes (even if they
/// have no outgoing edges, they must be keyed with an empty vec).
pub fn build_graph(
    files: &[(PathBuf, String)], // (path, language) for all project files
    edges: &HashMap<PathBuf, Vec<(PathBuf, EdgeKind)>>,
) -> DepResult {
    // Index files: path → index
    let index: HashMap<&PathBuf, usize> =
        files.iter().enumerate().map(|(i, (p, _))| (p, i)).collect();

    let n = files.len();

    let adj = adjacency(n, &index, edges);
    let loading: Vec<Vec<usize>> = adj
        .iter()
        .map(|out| {
            let mut to = of_kind(out, EdgeKind::Compile);
            to.extend(of_kind(out, EdgeKind::Load));
            to
        })
        .collect();
    let count = |i: usize, kind: EdgeKind| of_kind(&adj[i], kind).len();

    let mut fan_in = vec![0usize; n];
    for &(dst, _) in adj.iter().flatten() {
        fan_in[dst] += 1;
    }

    // A strongly connected component of more than one file is a cycle.
    let sccs: Vec<Vec<usize>> = tarjan_scc(n, &loading)
        .into_iter()
        .filter(|scc| scc.len() > 1)
        .collect();
    let mut in_cycle = vec![false; n];
    for &i in sccs.iter().flatten() {
        in_cycle[i] = true;
    }
    let mut cycles: Vec<Vec<PathBuf>> = sccs
        .iter()
        .map(|scc| {
            let mut paths: Vec<PathBuf> = scc.iter().map(|&i| files[i].0.clone()).collect();
            paths.sort();
            paths
        })
        .collect();
    cycles.sort();

    let entries = files
        .iter()
        .enumerate()
        .map(|(i, (path, lang))| DepEntry {
            path: path.clone(),
            language: lang.clone(),
            fan_in: fan_in[i],
            fan_out: adj[i].len(),
            type_fan_out: count(i, EdgeKind::Type),
            deferred_fan_out: count(i, EdgeKind::Deferred),
            compile_fan_out: count(i, EdgeKind::Compile),
            in_cycle: in_cycle[i],
            unresolved: Vec::new(),
        })
        .collect::<Vec<_>>();

    DepResult {
        totals: Totals::of(&entries),
        entries,
        cycles,
        unsupported: Vec::new(),
    }
}

/// For each of the `n` files by index, the known files it uses, with the
/// kind of use.
fn adjacency(
    n: usize,
    index: &HashMap<&PathBuf, usize>,
    edges: &HashMap<PathBuf, Vec<(PathBuf, EdgeKind)>>,
) -> Vec<Vec<(usize, EdgeKind)>> {
    let mut adj = vec![vec![]; n];
    for (src, dsts) in edges {
        let Some(&si) = index.get(src) else { continue };
        let known = dsts
            .iter()
            .filter_map(|(dst, kind)| Some((*index.get(dst)?, *kind)));
        adj[si].extend(known);
    }
    adj
}

/// The files of `out` used with `kind`.
fn of_kind(out: &[(usize, EdgeKind)], kind: EdgeKind) -> Vec<usize> {
    out.iter()
        .filter(|(_, k)| *k == kind)
        .map(|(to, _)| *to)
        .collect()
}

/// Tarjan's strongly-connected components algorithm (iterative to avoid stack overflow).
fn tarjan_scc(n: usize, adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut tarjan = Tarjan {
        adj,
        counter: 0,
        stack: Vec::new(),
        on_stack: vec![false; n],
        index: vec![None; n],
        lowlink: vec![0; n],
        sccs: Vec::new(),
    };
    for start in 0..n {
        if tarjan.index[start].is_none() {
            tarjan.search(start);
        }
    }
    tarjan.sccs
}

/// The state of Tarjan's algorithm over a graph.
struct Tarjan<'a> {
    adj: &'a [Vec<usize>],
    counter: usize,
    stack: Vec<usize>,
    on_stack: Vec<bool>,
    index: Vec<Option<usize>>,
    lowlink: Vec<usize>,
    sccs: Vec<Vec<usize>>,
}

impl Tarjan<'_> {
    /// Depth first from `start`, with an explicit stack of each node and
    /// the next of its neighbors to visit.
    fn search(&mut self, start: usize) {
        self.enter(start);
        let mut work = vec![(start, 0usize)];
        while let Some(&(v, next)) = work.last() {
            let Some(&w) = self.adj[v].get(next) else {
                work.pop();
                self.leave(v, work.last().map(|&(parent, _)| parent));
                continue;
            };
            if let Some(top) = work.last_mut() {
                top.1 += 1;
            }
            match self.index[w] {
                None => {
                    self.enter(w);
                    work.push((w, 0));
                }
                Some(index) if self.on_stack[w] => self.lowlink[v] = self.lowlink[v].min(index),
                Some(_) => {}
            }
        }
    }

    fn enter(&mut self, v: usize) {
        self.index[v] = Some(self.counter);
        self.lowlink[v] = self.counter;
        self.counter += 1;
        self.stack.push(v);
        self.on_stack[v] = true;
    }

    /// Done with `v`: its parent learns its lowlink, and a root takes its
    /// component off the stack.
    fn leave(&mut self, v: usize, parent: Option<usize>) {
        if let Some(parent) = parent {
            self.lowlink[parent] = self.lowlink[parent].min(self.lowlink[v]);
        }
        if self.index[v] != Some(self.lowlink[v]) {
            return;
        }
        let mut scc = Vec::new();
        while let Some(w) = self.stack.pop() {
            self.on_stack[w] = false;
            scc.push(w);
            if w == v {
                break;
            }
        }
        self.sccs.push(scc);
    }
}

/// JSON-serializable dependency entry.
#[derive(Serialize)]
pub struct JsonDepEntry<'a> {
    pub path: String,
    #[serde(flatten)]
    pub entry: &'a DepEntry,
}

/// JSON-serializable full result.
#[derive(Serialize)]
pub struct JsonDepResult<'a> {
    pub files: Vec<JsonDepEntry<'a>>,
    pub cycles: Vec<Vec<String>>,
    pub cycle_count: usize,
    pub type_edges: usize,
    pub deferred_edges: usize,
    pub compile_edges: usize,
    pub unresolved_count: usize,
    pub unsupported: Vec<UnsupportedLanguage>,
}

impl<'a> From<&'a DepResult> for JsonDepResult<'a> {
    fn from(r: &'a DepResult) -> Self {
        JsonDepResult {
            files: r
                .entries
                .iter()
                .map(|entry| JsonDepEntry {
                    path: entry.path.display().to_string(),
                    entry,
                })
                .collect(),
            cycles: r
                .cycles
                .iter()
                .map(|c| c.iter().map(|p| p.display().to_string()).collect())
                .collect(),
            cycle_count: r.cycles.len(),
            type_edges: r.totals.type_edges,
            deferred_edges: r.totals.deferred_edges,
            compile_edges: r.totals.compile_edges,
            unresolved_count: r.totals.unresolved,
            unsupported: r.unsupported.clone(),
        }
    }
}

/// Resolve a raw import string to the project-relative files it names, given the
/// importer's location. Empty if the import names no known project file.
/// Only a Kaikai package import can name more than one file. Python and Go
/// are resolved apart, since they depend on the layout.
pub fn resolve_import(
    importer: &Path,  // project-relative path of the importing file
    import_str: &str, // raw import string from extractor
    language: &str,
    file_set: &HashSet<PathBuf>,
) -> Vec<PathBuf> {
    let dir = importer.parent().unwrap_or(Path::new(""));
    let single = match language {
        "JavaScript" | "TypeScript" | "JSX" => javascript::relative(dir, import_str, file_set),
        "Kaikai" => return kaikai::resolve(importer, import_str, file_set),
        _ => None,
    };
    single.into_iter().collect()
}

/// Normalize a path by resolving `..` components (without touching the filesystem).
#[cfg(test)]
fn normalize_path(path: &Path) -> PathBuf {
    javascript::normalize(path).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(items: &[&str]) -> Vec<(PathBuf, String)> {
        items
            .iter()
            .map(|s| (PathBuf::from(s), "Rust".to_string()))
            .collect()
    }

    fn edges_map(pairs: &[(&str, &[&str])]) -> HashMap<PathBuf, Vec<(PathBuf, EdgeKind)>> {
        pairs
            .iter()
            .map(|(k, vs)| {
                (
                    PathBuf::from(k),
                    vs.iter()
                        .map(|v| (PathBuf::from(v), EdgeKind::Load))
                        .collect(),
                )
            })
            .collect()
    }

    #[test]
    fn imports_that_do_not_load_make_no_cycle_but_count_in_fan_out() {
        let files = paths(&["a.ts", "b.ts", "c.py"]);
        let edges: HashMap<PathBuf, Vec<(PathBuf, EdgeKind)>> = [
            (
                "a.ts",
                vec![("b.ts", EdgeKind::Type), ("c.py", EdgeKind::Deferred)],
            ),
            ("b.ts", vec![("a.ts", EdgeKind::Load)]),
            ("c.py", vec![("a.ts", EdgeKind::Load)]),
        ]
        .into_iter()
        .map(|(from, to)| {
            let to = to.into_iter().map(|(p, k)| (PathBuf::from(p), k)).collect();
            (PathBuf::from(from), to)
        })
        .collect();
        let result = build_graph(&files, &edges);
        assert!(result.cycles.is_empty());
        let a = &result.entries[0];
        assert_eq!((a.fan_out, a.type_fan_out, a.deferred_fan_out), (2, 1, 1));
        assert_eq!(a.fan_in, 2);
        assert_eq!(result.totals.type_edges, 1);
        assert_eq!(result.totals.deferred_edges, 1);
    }

    #[test]
    fn uses_needed_to_compile_make_cycles_and_are_counted() {
        let files = paths(&["a.ex", "b.ex"]);
        let edges: HashMap<PathBuf, Vec<(PathBuf, EdgeKind)>> = [
            ("a.ex", ("b.ex", EdgeKind::Compile)),
            ("b.ex", ("a.ex", EdgeKind::Load)),
        ]
        .into_iter()
        .map(|(from, (to, kind))| (PathBuf::from(from), vec![(PathBuf::from(to), kind)]))
        .collect();
        let result = build_graph(&files, &edges);
        assert_eq!(result.cycles.len(), 1);
        assert_eq!(result.entries[0].compile_fan_out, 1);
        assert_eq!(result.totals.compile_edges, 1);
    }

    #[test]
    fn fan_in_fan_out() {
        // a → b → c  (linear chain)
        let files = paths(&["a.rs", "b.rs", "c.rs"]);
        let edges = edges_map(&[("a.rs", &["b.rs"]), ("b.rs", &["c.rs"]), ("c.rs", &[])]);
        let result = build_graph(&files, &edges);
        let by_path: HashMap<_, _> = result
            .entries
            .iter()
            .map(|e| (e.path.as_path(), e))
            .collect();
        assert_eq!(by_path[Path::new("a.rs")].fan_out, 1);
        assert_eq!(by_path[Path::new("a.rs")].fan_in, 0);
        assert_eq!(by_path[Path::new("b.rs")].fan_in, 1);
        assert_eq!(by_path[Path::new("b.rs")].fan_out, 1);
        assert_eq!(by_path[Path::new("c.rs")].fan_in, 1);
        assert_eq!(by_path[Path::new("c.rs")].fan_out, 0);
    }

    #[test]
    fn no_cycles_in_linear_chain() {
        let files = paths(&["a.rs", "b.rs", "c.rs"]);
        let edges = edges_map(&[("a.rs", &["b.rs"]), ("b.rs", &["c.rs"]), ("c.rs", &[])]);
        let result = build_graph(&files, &edges);
        assert!(result.cycles.is_empty());
        assert!(result.entries.iter().all(|e| !e.in_cycle));
    }

    #[test]
    fn detects_simple_cycle() {
        // a → b → a
        let files = paths(&["a.rs", "b.rs"]);
        let edges = edges_map(&[("a.rs", &["b.rs"]), ("b.rs", &["a.rs"])]);
        let result = build_graph(&files, &edges);
        assert_eq!(result.cycles.len(), 1);
        assert_eq!(result.cycles[0].len(), 2);
        assert!(result.entries.iter().all(|e| e.in_cycle));
    }

    #[test]
    fn self_loop_is_not_a_cycle() {
        // a → a (self-loop) — Tarjan SCC of size 1, not reported as cycle
        let files = paths(&["a.rs"]);
        let edges = edges_map(&[("a.rs", &["a.rs"])]);
        let result = build_graph(&files, &edges);
        assert!(result.cycles.is_empty());
    }

    #[test]
    fn three_node_cycle() {
        // a → b → c → a
        let files = paths(&["a.rs", "b.rs", "c.rs"]);
        let edges = edges_map(&[
            ("a.rs", &["b.rs"]),
            ("b.rs", &["c.rs"]),
            ("c.rs", &["a.rs"]),
        ]);
        let result = build_graph(&files, &edges);
        assert_eq!(result.cycles.len(), 1);
        assert_eq!(result.cycles[0].len(), 3);
    }

    #[test]
    fn resolve_js_relative() {
        let mut file_set = HashSet::new();
        file_set.insert(PathBuf::from("src/utils.ts"));
        let result = resolve_import(
            Path::new("src/components/App.tsx"),
            "../utils",
            "TypeScript",
            &file_set,
        );
        assert_eq!(result, vec![PathBuf::from("src/utils.ts")]);
    }

    #[test]
    fn normalize_dotdot() {
        let p = normalize_path(Path::new("src/foo/../bar.rs"));
        assert_eq!(p, PathBuf::from("src/bar.rs"));
    }

    // ── JavaScript/TypeScript resolution ──────────────────────────────────

    #[test]
    fn resolve_js_direct_extension() {
        let mut file_set = HashSet::new();
        file_set.insert(PathBuf::from("src/utils.js"));
        let result = resolve_import(
            Path::new("src/app.js"),
            "./utils.js",
            "JavaScript",
            &file_set,
        );
        assert_eq!(result, vec![PathBuf::from("src/utils.js")]);
    }

    #[test]
    fn resolve_js_without_extension_ts() {
        let mut file_set = HashSet::new();
        file_set.insert(PathBuf::from("src/utils.ts"));
        let result = resolve_import(Path::new("src/app.ts"), "./utils", "TypeScript", &file_set);
        assert_eq!(result, vec![PathBuf::from("src/utils.ts")]);
    }

    #[test]
    fn resolve_js_index_file() {
        let mut file_set = HashSet::new();
        file_set.insert(PathBuf::from("src/components/index.tsx"));
        let result = resolve_import(
            Path::new("src/app.tsx"),
            "./components",
            "TypeScript",
            &file_set,
        );
        assert_eq!(result, vec![PathBuf::from("src/components/index.tsx")]);
    }

    #[test]
    fn resolve_js_jsx_extension() {
        let mut file_set = HashSet::new();
        file_set.insert(PathBuf::from("src/Button.jsx"));
        let result = resolve_import(Path::new("src/App.jsx"), "./Button", "JSX", &file_set);
        assert_eq!(result, vec![PathBuf::from("src/Button.jsx")]);
    }

    #[test]
    fn resolve_js_not_found_returns_none() {
        let file_set = HashSet::new();
        let result = resolve_import(
            Path::new("src/app.ts"),
            "./missing",
            "TypeScript",
            &file_set,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn resolve_js_direct_extension_not_found_returns_none() {
        // Has extension but file doesn't exist in file_set
        let file_set = HashSet::new();
        let result = resolve_import(
            Path::new("src/app.ts"),
            "./nonexistent.ts",
            "TypeScript",
            &file_set,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn resolve_unknown_language_returns_none() {
        let file_set = HashSet::new();
        let result = resolve_import(Path::new("main.sh"), "utils", "Bash", &file_set);
        assert!(result.is_empty());
    }

    #[test]
    fn resolve_kaikai_through_the_dispatcher() {
        let mut file_set = HashSet::new();
        file_set.insert(PathBuf::from("app/b.kai"));
        let result = resolve_import(Path::new("app/main.kai"), "app.b", "Kaikai", &file_set);
        assert_eq!(result, vec![PathBuf::from("app/b.kai")]);
    }

    // ── normalize_path ─────────────────────────────────────────────────────

    #[test]
    fn normalize_cur_dir_component() {
        let p = normalize_path(Path::new("./src/foo.rs"));
        assert_eq!(p, PathBuf::from("src/foo.rs"));
    }

    #[test]
    fn normalize_already_clean() {
        let p = normalize_path(Path::new("src/foo.rs"));
        assert_eq!(p, PathBuf::from("src/foo.rs"));
    }

    // ── JsonDepResult conversion ───────────────────────────────────────────

    #[test]
    fn json_dep_result_conversion() {
        let files = paths(&["a.rs", "b.rs"]);
        let edges = edges_map(&[("a.rs", &["b.rs"]), ("b.rs", &[])]);
        let result = build_graph(&files, &edges);
        let json: JsonDepResult = (&result).into();
        assert_eq!(json.files.len(), 2);
        assert_eq!(json.cycles.len(), 0);
        assert_eq!(json.cycle_count, 0);
    }

    #[test]
    fn json_dep_result_with_cycle() {
        let files = paths(&["a.rs", "b.rs"]);
        let edges = edges_map(&[("a.rs", &["b.rs"]), ("b.rs", &["a.rs"])]);
        let result = build_graph(&files, &edges);
        let json: JsonDepResult = (&result).into();
        assert_eq!(json.cycle_count, 1);
        assert_eq!(json.cycles.len(), 1);
        assert!(json.files.iter().all(|f| f.entry.in_cycle));
    }
}
