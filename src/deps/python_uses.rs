//! What a Python file uses: the files its imports name, each with the
//! names it takes there.

use std::collections::HashMap;
use std::path::Path;

use super::graph::{Found, Source};
use super::layout::Layout;
use super::{python, python_names};

/// A file of a graph as a Python module.
struct Module {
    /// The name its calls are told by: that of the file, or of its
    /// directory when it is a package.
    label: String,
    /// The names it defines, for an import that does not say which ones
    /// it takes.
    names: Vec<String>,
}

/// The files of a graph as Python modules. What is not Python defines
/// nothing.
pub struct Modules(Vec<Module>);

fn label(path: &Path) -> String {
    let named = match path.file_stem() {
        Some(stem) if stem == "__init__" => path.parent().and_then(Path::file_name),
        stem => stem,
    };
    named.map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

impl Modules {
    pub fn of(sources: &[Source]) -> Self {
        let module = |source: &Source| Module {
            label: label(&source.path),
            names: match source.language.as_str() {
                "Python" => python_names::defined(&source.text),
                _ => Vec::new(),
            },
        };
        Modules(sources.iter().map(module).collect())
    }

    /// The files `source` imports, each with the names it takes there and
    /// when the import runs, and the relative imports that name no file.
    /// `None` when it is not Python.
    pub fn uses(
        &self,
        source: &Source,
        layout: &Layout,
        by_path: &HashMap<&Path, usize>,
    ) -> Option<Found> {
        (source.language == "Python").then(|| self.uses_of(source, layout, by_path))
    }

    fn uses_of(&self, source: &Source, layout: &Layout, by_path: &HashMap<&Path, usize>) -> Found {
        let resolve = |import: &str| {
            let path = python::resolve(&source.path, import, &layout.known, &layout.python_roots)?;
            by_path.get(path.as_path()).copied()
        };
        let read = python::read(&source.text);

        let mut found = Found::default();
        for binding in &read.bindings {
            // A name taken from a package may be one of its modules.
            let submodule = binding.submodule().and_then(|module| resolve(&module));
            let Some(to) = submodule.or_else(|| resolve(&binding.module)) else {
                found.unresolved.extend(unresolved(binding));
                continue;
            };
            let module = &self.0[to];
            let taken = module.taken(binding, submodule.is_some(), &read.code);
            let calls = taken.iter().map(|name| format!("{}.{name}", module.label));
            found.add(to, binding.kind).extend(calls);
        }
        found
    }
}

/// A relative import that names no file, as written: only those are known
/// to be the project's. `from . import a` is `.a`.
fn unresolved(binding: &python::Binding) -> Option<String> {
    if !binding.module.starts_with('.') {
        return None;
    }
    if binding.module.trim_start_matches('.').is_empty() {
        binding.submodule()
    } else {
        Some(binding.module.clone())
    }
}

impl Module {
    /// The names `binding` takes from this module in a file with `code`.
    /// `is_submodule` says the binding names the module itself, though
    /// written as a name taken from its package.
    fn taken(&self, binding: &python::Binding, is_submodule: bool, code: &[String]) -> Vec<String> {
        let mentioned = |name: &&String| code.iter().any(|line| python_names::mentions(line, name));
        match (&binding.bound, &binding.name) {
            // `*` takes the names the file goes on to mention.
            (None, _) => self.names.iter().filter(mentioned).cloned().collect(),
            (Some(_), Some(name)) if !is_submodule => vec![name.clone()],
            (Some(bound), _) => {
                let read = python_names::taken(code, bound);
                // Used as a value, the module may give any of its names.
                let whole = read.whole.then_some(&self.names).into_iter().flatten();
                read.names.iter().chain(whole).cloned().collect()
            }
        }
    }
}
