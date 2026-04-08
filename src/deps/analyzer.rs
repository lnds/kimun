/// Dependency graph builder and analyzer.
///
/// Builds a directed graph of internal file dependencies, computes per-file
/// fan-in (importers) and fan-out (imports), and detects cycles using
/// Tarjan's strongly-connected components (SCC) algorithm.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// A single file's dependency metrics.
pub struct DepEntry {
    pub path: PathBuf,
    pub language: String,
    /// Number of project files that import this file.
    pub fan_in: usize,
    /// Number of project files this file imports.
    pub fan_out: usize,
    /// True if this file participates in a dependency cycle.
    pub in_cycle: bool,
}

/// Full dependency analysis result.
pub struct DepResult {
    pub entries: Vec<DepEntry>,
    /// Each inner Vec is a set of files forming a cycle (SCC size > 1).
    pub cycles: Vec<Vec<PathBuf>>,
}

/// Build the dependency graph from raw edges and compute metrics.
///
/// `edges` maps each file to the list of files it imports.
/// Only files present as keys in `edges` are included as nodes (even if they
/// have no outgoing edges, they must be keyed with an empty vec).
pub fn build_graph(
    files: &[(PathBuf, String)], // (path, language) for all project files
    edges: &HashMap<PathBuf, Vec<PathBuf>>,
) -> DepResult {
    // Index files: path → index
    let index: HashMap<&PathBuf, usize> =
        files.iter().enumerate().map(|(i, (p, _))| (p, i)).collect();

    let n = files.len();

    // Build adjacency list using indices
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    for (src, dsts) in edges {
        if let Some(&si) = index.get(src) {
            for dst in dsts {
                if let Some(&di) = index.get(dst) {
                    adj[si].push(di);
                }
            }
        }
    }

    // Fan-out: number of outgoing edges per node
    let fan_out: Vec<usize> = adj.iter().map(|v| v.len()).collect();

    // Fan-in: number of incoming edges per node
    let mut fan_in = vec![0usize; n];
    for neighbors in &adj {
        for &dst in neighbors {
            fan_in[dst] += 1;
        }
    }

    // Tarjan's SCC to find cycles
    let sccs = tarjan_scc(n, &adj);

    // Mark nodes that are part of a non-trivial SCC (size > 1)
    let mut in_cycle = vec![false; n];
    let mut cycles: Vec<Vec<PathBuf>> = Vec::new();
    for scc in &sccs {
        if scc.len() > 1 {
            for &i in scc {
                in_cycle[i] = true;
            }
            let mut cycle_paths: Vec<PathBuf> = scc.iter().map(|&i| files[i].0.clone()).collect();
            cycle_paths.sort();
            cycles.push(cycle_paths);
        }
    }
    cycles.sort();

    let entries = files
        .iter()
        .enumerate()
        .map(|(i, (path, lang))| DepEntry {
            path: path.clone(),
            language: lang.clone(),
            fan_in: fan_in[i],
            fan_out: fan_out[i],
            in_cycle: in_cycle[i],
        })
        .collect();

    DepResult { entries, cycles }
}

/// Tarjan's strongly-connected components algorithm (iterative to avoid stack overflow).
fn tarjan_scc(n: usize, adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut index_counter = 0usize;
    let mut stack: Vec<usize> = Vec::new();
    let mut on_stack = vec![false; n];
    let mut index: Vec<Option<usize>> = vec![None; n];
    let mut lowlink = vec![0usize; n];
    let mut sccs: Vec<Vec<usize>> = Vec::new();

    // Iterative DFS using an explicit work stack.
    // Each entry: (node, iterator position in adj[node])
    let mut work: Vec<(usize, usize)> = Vec::new();

    for start in 0..n {
        if index[start].is_some() {
            continue;
        }

        work.push((start, 0));

        while let Some((v, ei)) = work.last_mut() {
            let v = *v;
            if index[v].is_none() {
                // First visit
                index[v] = Some(index_counter);
                lowlink[v] = index_counter;
                index_counter += 1;
                stack.push(v);
                on_stack[v] = true;
            }

            let neighbors = &adj[v];
            if *ei < neighbors.len() {
                let w = neighbors[*ei];
                *ei += 1;
                if index[w].is_none() {
                    work.push((w, 0));
                } else if on_stack[w] {
                    lowlink[v] = lowlink[v].min(index[w].unwrap());
                }
            } else {
                // Done with v's neighbors — pop and update parent's lowlink
                work.pop();
                if let Some((parent, _)) = work.last() {
                    let parent = *parent;
                    lowlink[parent] = lowlink[parent].min(lowlink[v]);
                }
                // Check if v is an SCC root
                if lowlink[v] == index[v].unwrap() {
                    let mut scc = Vec::new();
                    loop {
                        let w = stack.pop().unwrap();
                        on_stack[w] = false;
                        scc.push(w);
                        if w == v {
                            break;
                        }
                    }
                    sccs.push(scc);
                }
            }
        }
    }
    sccs
}

/// JSON-serializable dependency entry.
#[derive(Serialize)]
pub struct JsonDepEntry {
    pub path: String,
    pub language: String,
    pub fan_in: usize,
    pub fan_out: usize,
    pub in_cycle: bool,
}

/// JSON-serializable full result.
#[derive(Serialize)]
pub struct JsonDepResult {
    pub files: Vec<JsonDepEntry>,
    pub cycles: Vec<Vec<String>>,
    pub cycle_count: usize,
}

impl From<&DepResult> for JsonDepResult {
    fn from(r: &DepResult) -> Self {
        JsonDepResult {
            files: r
                .entries
                .iter()
                .map(|e| JsonDepEntry {
                    path: e.path.display().to_string(),
                    language: e.language.clone(),
                    fan_in: e.fan_in,
                    fan_out: e.fan_out,
                    in_cycle: e.in_cycle,
                })
                .collect(),
            cycles: r
                .cycles
                .iter()
                .map(|c| c.iter().map(|p| p.display().to_string()).collect())
                .collect(),
            cycle_count: r.cycles.len(),
        }
    }
}

/// Resolve a raw import string to a project-relative path, given the importer's location.
/// Returns `None` if the import cannot be resolved to a known project file.
pub fn resolve_import(
    importer: &Path,  // project-relative path of the importing file
    import_str: &str, // raw import string from extractor
    language: &str,
    file_set: &std::collections::HashSet<PathBuf>,
    go_module: Option<&str>,
) -> Option<PathBuf> {
    let dir = importer.parent().unwrap_or(Path::new(""));
    match language {
        "Rust" => resolve_rust(dir, import_str, file_set),
        "Python" => resolve_python(dir, import_str, file_set),
        "JavaScript" | "TypeScript" | "JSX" | "TSX" => resolve_js(dir, import_str, file_set),
        "Go" => resolve_go(import_str, go_module, file_set),
        _ => None,
    }
}

fn resolve_rust(
    dir: &Path,
    name: &str,
    file_set: &std::collections::HashSet<PathBuf>,
) -> Option<PathBuf> {
    // `mod foo;` → foo.rs or foo/mod.rs relative to current directory
    let as_file = dir.join(format!("{name}.rs"));
    if file_set.contains(&as_file) {
        return Some(as_file);
    }
    let as_mod = dir.join(name).join("mod.rs");
    if file_set.contains(&as_mod) {
        return Some(as_mod);
    }
    // lib.rs files can also declare submodules as name/lib.rs (rare but possible)
    None
}

fn resolve_python(
    dir: &Path,
    import_str: &str,
    file_set: &std::collections::HashSet<PathBuf>,
) -> Option<PathBuf> {
    // Count leading dots for relative level
    let dots = import_str.chars().take_while(|c| *c == '.').count();
    let module = &import_str[dots..]; // module name after dots

    // Navigate up `dots - 1` directories from current dir
    let mut base = dir.to_path_buf();
    for _ in 1..dots {
        base = base.parent().unwrap_or(Path::new("")).to_path_buf();
    }

    if module.is_empty() {
        return None;
    }

    // Convert dotted module to path: foo.bar → foo/bar
    let rel = module.replace('.', "/");
    let as_file = base.join(format!("{rel}.py"));
    if file_set.contains(&as_file) {
        return Some(as_file);
    }
    let as_init = base.join(&rel).join("__init__.py");
    if file_set.contains(&as_init) {
        return Some(as_init);
    }
    None
}

fn resolve_js(
    dir: &Path,
    import_str: &str,
    file_set: &std::collections::HashSet<PathBuf>,
) -> Option<PathBuf> {
    let base = dir.join(import_str);
    // If already has an extension, try directly
    if base.extension().is_some() {
        let p = normalize_path(&base);
        if file_set.contains(&p) {
            return Some(p);
        }
    }
    // Try common JS/TS extensions
    for ext in &["ts", "tsx", "js", "jsx", "mts", "mjs"] {
        let p = normalize_path(&base.with_extension(ext));
        if file_set.contains(&p) {
            return Some(p);
        }
    }
    // Try index file
    for ext in &["ts", "tsx", "js", "jsx"] {
        let p = normalize_path(&base.join(format!("index.{ext}")));
        if file_set.contains(&p) {
            return Some(p);
        }
    }
    None
}

fn resolve_go(
    import_str: &str,
    go_module: Option<&str>,
    file_set: &std::collections::HashSet<PathBuf>,
) -> Option<PathBuf> {
    let module = go_module?;
    let rel = import_str.strip_prefix(module)?.trim_start_matches('/');
    if rel.is_empty() {
        return None;
    }
    // Find any .go file in that directory
    let dir = PathBuf::from(rel);
    file_set
        .iter()
        .find(|p| p.starts_with(&dir) && p.extension().is_some_and(|e| e == "go"))
        .cloned()
}

/// Normalize a path by resolving `..` components (without touching the filesystem).
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
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

    fn edges_map(pairs: &[(&str, &[&str])]) -> HashMap<PathBuf, Vec<PathBuf>> {
        pairs
            .iter()
            .map(|(k, vs)| {
                (
                    PathBuf::from(k),
                    vs.iter().map(|v| PathBuf::from(v)).collect(),
                )
            })
            .collect()
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
    fn resolve_rust_file() {
        let mut file_set = std::collections::HashSet::new();
        file_set.insert(PathBuf::from("src/foo.rs"));
        let result = resolve_import(Path::new("src/main.rs"), "foo", "Rust", &file_set, None);
        assert_eq!(result, Some(PathBuf::from("src/foo.rs")));
    }

    #[test]
    fn resolve_rust_mod_dir() {
        let mut file_set = std::collections::HashSet::new();
        file_set.insert(PathBuf::from("src/bar/mod.rs"));
        let result = resolve_import(Path::new("src/main.rs"), "bar", "Rust", &file_set, None);
        assert_eq!(result, Some(PathBuf::from("src/bar/mod.rs")));
    }

    #[test]
    fn resolve_js_relative() {
        let mut file_set = std::collections::HashSet::new();
        file_set.insert(PathBuf::from("src/utils.ts"));
        let result = resolve_import(
            Path::new("src/components/App.tsx"),
            "../utils",
            "TypeScript",
            &file_set,
            None,
        );
        assert_eq!(result, Some(PathBuf::from("src/utils.ts")));
    }

    #[test]
    fn normalize_dotdot() {
        let p = normalize_path(Path::new("src/foo/../bar.rs"));
        assert_eq!(p, PathBuf::from("src/bar.rs"));
    }

    // ── Python resolution ──────────────────────────────────────────────────

    #[test]
    fn resolve_python_relative_as_file() {
        let mut file_set = std::collections::HashSet::new();
        file_set.insert(PathBuf::from("src/utils.py"));
        let result = resolve_import(
            Path::new("src/main.py"),
            ".utils",
            "Python",
            &file_set,
            None,
        );
        assert_eq!(result, Some(PathBuf::from("src/utils.py")));
    }

    #[test]
    fn resolve_python_relative_as_package() {
        let mut file_set = std::collections::HashSet::new();
        file_set.insert(PathBuf::from("src/utils/__init__.py"));
        let result = resolve_import(
            Path::new("src/main.py"),
            ".utils",
            "Python",
            &file_set,
            None,
        );
        assert_eq!(result, Some(PathBuf::from("src/utils/__init__.py")));
    }

    #[test]
    fn resolve_python_double_dot_goes_up() {
        let mut file_set = std::collections::HashSet::new();
        file_set.insert(PathBuf::from("common.py"));
        // From src/sub/main.py, ..common → src/common.py? No, two dots goes up two levels
        // importer: src/sub/main.py → dir = src/sub
        // dots=2 → base goes up 1 level (for _ in 1..2) → base = src
        // module = "common" → src/common.py
        file_set.insert(PathBuf::from("src/common.py"));
        let result = resolve_import(
            Path::new("src/sub/main.py"),
            "..common",
            "Python",
            &file_set,
            None,
        );
        assert_eq!(result, Some(PathBuf::from("src/common.py")));
    }

    #[test]
    fn resolve_python_dotted_module_path() {
        let mut file_set = std::collections::HashSet::new();
        file_set.insert(PathBuf::from("src/foo/bar.py"));
        let result = resolve_import(
            Path::new("src/main.py"),
            ".foo.bar",
            "Python",
            &file_set,
            None,
        );
        assert_eq!(result, Some(PathBuf::from("src/foo/bar.py")));
    }

    #[test]
    fn resolve_python_module_only_dots_returns_none() {
        // Single dot with no module name → skip
        let file_set = std::collections::HashSet::new();
        let result = resolve_import(Path::new("src/main.py"), ".", "Python", &file_set, None);
        assert!(result.is_none());
    }

    #[test]
    fn resolve_python_not_found_returns_none() {
        let file_set = std::collections::HashSet::new();
        let result = resolve_import(
            Path::new("src/main.py"),
            ".missing",
            "Python",
            &file_set,
            None,
        );
        assert!(result.is_none());
    }

    // ── JavaScript/TypeScript resolution ──────────────────────────────────

    #[test]
    fn resolve_js_direct_extension() {
        let mut file_set = std::collections::HashSet::new();
        file_set.insert(PathBuf::from("src/utils.js"));
        let result = resolve_import(
            Path::new("src/app.js"),
            "./utils.js",
            "JavaScript",
            &file_set,
            None,
        );
        assert_eq!(result, Some(PathBuf::from("src/utils.js")));
    }

    #[test]
    fn resolve_js_without_extension_ts() {
        let mut file_set = std::collections::HashSet::new();
        file_set.insert(PathBuf::from("src/utils.ts"));
        let result = resolve_import(
            Path::new("src/app.ts"),
            "./utils",
            "TypeScript",
            &file_set,
            None,
        );
        assert_eq!(result, Some(PathBuf::from("src/utils.ts")));
    }

    #[test]
    fn resolve_js_index_file() {
        let mut file_set = std::collections::HashSet::new();
        file_set.insert(PathBuf::from("src/components/index.tsx"));
        let result = resolve_import(
            Path::new("src/app.tsx"),
            "./components",
            "TSX",
            &file_set,
            None,
        );
        assert_eq!(result, Some(PathBuf::from("src/components/index.tsx")));
    }

    #[test]
    fn resolve_js_jsx_extension() {
        let mut file_set = std::collections::HashSet::new();
        file_set.insert(PathBuf::from("src/Button.jsx"));
        let result = resolve_import(Path::new("src/App.jsx"), "./Button", "JSX", &file_set, None);
        assert_eq!(result, Some(PathBuf::from("src/Button.jsx")));
    }

    #[test]
    fn resolve_js_not_found_returns_none() {
        let file_set = std::collections::HashSet::new();
        let result = resolve_import(
            Path::new("src/app.ts"),
            "./missing",
            "TypeScript",
            &file_set,
            None,
        );
        assert!(result.is_none());
    }

    #[test]
    fn resolve_js_direct_extension_not_found_returns_none() {
        // Has extension but file doesn't exist in file_set
        let file_set = std::collections::HashSet::new();
        let result = resolve_import(
            Path::new("src/app.ts"),
            "./nonexistent.ts",
            "TypeScript",
            &file_set,
            None,
        );
        assert!(result.is_none());
    }

    // ── Go resolution ──────────────────────────────────────────────────────

    #[test]
    fn resolve_go_with_module() {
        let mut file_set = std::collections::HashSet::new();
        file_set.insert(PathBuf::from("pkg/foo/main.go"));
        let result = resolve_import(
            Path::new("main.go"),
            "github.com/user/proj/pkg/foo",
            "Go",
            &file_set,
            Some("github.com/user/proj"),
        );
        assert_eq!(result, Some(PathBuf::from("pkg/foo/main.go")));
    }

    #[test]
    fn resolve_go_no_module_returns_none() {
        let file_set = std::collections::HashSet::new();
        let result = resolve_import(
            Path::new("main.go"),
            "github.com/user/proj/pkg/foo",
            "Go",
            &file_set,
            None, // no go_module
        );
        assert!(result.is_none());
    }

    #[test]
    fn resolve_go_external_package_returns_none() {
        // Import doesn't start with module prefix
        let file_set = std::collections::HashSet::new();
        let result = resolve_import(
            Path::new("main.go"),
            "fmt",
            "Go",
            &file_set,
            Some("github.com/user/proj"),
        );
        assert!(result.is_none());
    }

    #[test]
    fn resolve_go_root_module_path_returns_none() {
        // Strip prefix leaves empty string
        let file_set = std::collections::HashSet::new();
        let result = resolve_import(
            Path::new("main.go"),
            "github.com/user/proj",
            "Go",
            &file_set,
            Some("github.com/user/proj"),
        );
        assert!(result.is_none());
    }

    #[test]
    fn resolve_unknown_language_returns_none() {
        let file_set = std::collections::HashSet::new();
        let result = resolve_import(Path::new("main.sh"), "utils", "Bash", &file_set, None);
        assert!(result.is_none());
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
        assert!(json.files.iter().all(|f| f.in_cycle));
    }
}
