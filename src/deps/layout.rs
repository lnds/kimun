//! What a graph is built with besides its sources.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// What a graph is built with besides the sources.
#[derive(Default)]
pub struct Layout {
    /// Paths resolvers may look up besides the sources, such as manifests.
    pub known: HashSet<PathBuf>,
    /// Each Go module, as its directory and the path its `go.mod` declares.
    pub go_modules: Vec<(PathBuf, String)>,
    /// The name each Cargo package directory declares.
    pub packages: Vec<(PathBuf, String)>,
    /// The directories each Python project says its packages are imported
    /// from, by the directory of the project.
    pub python_roots: Vec<(PathBuf, Vec<PathBuf>)>,
}

fn is_named(path: &Path, file_name: &str) -> bool {
    path.file_name().is_some_and(|name| name == file_name)
}

/// Whether `path` is a manifest a layout reads: that of a Cargo package,
/// of a Python project or of a Go module.
pub fn is_manifest(path: &Path) -> bool {
    ["Cargo.toml", "pyproject.toml", "go.mod"]
        .iter()
        .any(|name| is_named(path, name))
}

impl Layout {
    /// Note what each of the `manifests` declares, given as path and text:
    /// the package of a Cargo one, the import roots of a Python one, the
    /// module path of a Go one.
    pub fn with_manifests<'t>(
        mut self,
        manifests: impl IntoIterator<Item = (&'t Path, &'t str)>,
    ) -> Self {
        for (path, text) in manifests {
            let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
            match path.file_name().and_then(|name| name.to_str()) {
                Some("pyproject.toml") => {
                    let roots = crate::projects::python_roots(path, text);
                    let declared = !roots.is_empty();
                    self.python_roots.extend(declared.then_some((dir, roots)));
                }
                Some("go.mod") => {
                    let module = crate::projects::go_module(text);
                    self.go_modules.extend(module.map(|module| (dir, module)));
                }
                _ => {
                    let package = crate::projects::cargo_package(text);
                    self.packages.extend(package.map(|package| (dir, package)));
                }
            }
        }
        self
    }
}
