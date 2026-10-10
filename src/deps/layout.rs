//! What a graph is built with besides its sources.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// What a graph is built with besides the sources.
#[derive(Default)]
pub struct Layout<'a> {
    /// Paths resolvers may look up besides the sources, such as manifests.
    pub known: HashSet<PathBuf>,
    /// The module path `go.mod` declares.
    pub go_module: Option<&'a str>,
    /// The name each Cargo package directory declares.
    pub packages: Vec<(PathBuf, String)>,
    /// The directories each Python project says its packages are imported
    /// from, by the directory of the project.
    pub python_roots: Vec<(PathBuf, Vec<PathBuf>)>,
}

fn is_named(path: &Path, file_name: &str) -> bool {
    path.file_name().is_some_and(|name| name == file_name)
}

/// Whether `path` is a manifest a layout reads: that of a Cargo package or
/// of a Python project.
pub fn is_manifest(path: &Path) -> bool {
    is_named(path, "Cargo.toml") || is_named(path, "pyproject.toml")
}

impl Layout<'_> {
    /// Note what each of the `manifests` declares, given as path and text:
    /// the package of a Cargo one, the import roots of a Python one.
    pub fn with_manifests<'t>(
        mut self,
        manifests: impl IntoIterator<Item = (&'t Path, &'t str)>,
    ) -> Self {
        for (path, text) in manifests {
            let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
            if is_named(path, "pyproject.toml") {
                let roots = crate::projects::python_roots(path, text);
                if !roots.is_empty() {
                    self.python_roots.push((dir, roots));
                }
            } else if let Some(package) = crate::projects::cargo_package(text) {
                self.packages.push((dir, package));
            }
        }
        self
    }
}
