//! Go, where an import names a package and a package is a directory.
//!
//! A package is every `.go` file directly inside its directory, so an
//! import leads to all of them: which one holds what was used is not read.
//! Test files are not part of what a package offers. The files of one
//! package use each other without importing, so the graph has no edges
//! between them. Build constraints are not read.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use super::golang_source;
use super::graph::Source;
use super::layout::Layout;

/// The packages of a graph, by directory, and the modules that name them.
pub struct Packages {
    /// For each directory, the files that make its package.
    by_dir: HashMap<PathBuf, Vec<usize>>,
    /// Each module as its directory and its path, the longest path first:
    /// a module nested in another has the longer one.
    modules: Vec<(PathBuf, String)>,
}

/// Whether a file is part of what its package offers to importers: not a
/// test, and not one the toolchain ignores (`_x.go`, `.x.go`).
fn offered(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    !name.ends_with("_test.go") && !name.starts_with(['_', '.'])
}

impl Packages {
    pub fn of(sources: &[Source], layout: &Layout) -> Self {
        let mut by_dir: HashMap<PathBuf, Vec<usize>> = HashMap::new();
        for (file, source) in sources.iter().enumerate() {
            if source.language == "Go" && offered(&source.path) {
                let dir = source.path.parent().unwrap_or(Path::new(""));
                by_dir.entry(dir.to_path_buf()).or_default().push(file);
            }
        }
        let mut modules = layout.go_modules.clone();
        modules.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.cmp(b)));
        Packages { by_dir, modules }
    }

    /// The directory the import path names, when a module of the
    /// repository holds it.
    fn directory(&self, import: &str) -> Option<PathBuf> {
        self.modules.iter().find_map(|(dir, module)| {
            let rest = import.strip_prefix(module.as_str())?;
            match rest.strip_prefix('/') {
                Some(package) => Some(dir.join(package)),
                None => rest.is_empty().then(|| dir.clone()),
            }
        })
    }

    /// The files `source` imports: those of each package it names. `None`
    /// when it is not Go. A package cannot import itself, so an import of
    /// its own directory is left out, unless the file is an external test.
    pub fn uses(&self, source: &Source) -> Option<BTreeMap<usize, BTreeSet<String>>> {
        (source.language == "Go").then(|| {
            let file = golang_source::read(&source.text);
            let own = source.path.parent().unwrap_or(Path::new(""));
            file.imports
                .iter()
                .filter_map(|import| self.directory(import))
                .filter(|dir| file.is_external_test() || dir != own)
                .filter_map(|dir| self.by_dir.get(&dir))
                .flatten()
                .map(|&file| (file, BTreeSet::new()))
                .collect()
        })
    }
}

#[cfg(test)]
#[path = "golang_test.rs"]
mod tests;
