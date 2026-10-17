//! What a graph is built with besides its sources.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::javascript_config::{self, Modules};

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
    /// The aliases and packages of JavaScript.
    pub javascript: Modules,
}

/// The manifests a layout reads by name, besides those of JavaScript: of a
/// Cargo package, a Python project and a Go module.
const MANIFESTS: [&str; 3] = ["Cargo.toml", "pyproject.toml", "go.mod"];

fn file_name(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

/// Whether `path` is a manifest a layout reads, or a config with import
/// aliases.
pub fn is_manifest(path: &Path) -> bool {
    MANIFESTS.contains(&file_name(path)) || javascript_config::is_manifest(path)
}

impl Layout {
    /// Note what each of the `manifests` declares, given as path and text:
    /// the package of a Cargo or JavaScript one, the import roots of a
    /// Python one, the module path of a Go one, the aliases of a config.
    pub fn with_manifests<'t>(
        mut self,
        manifests: impl IntoIterator<Item = (&'t Path, &'t str)>,
    ) -> Self {
        let (javascript, manifests): (Vec<_>, Vec<_>) = manifests
            .into_iter()
            .partition(|(path, _)| javascript_config::is_manifest(path));
        for (path, text) in manifests {
            let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
            match file_name(path) {
                "pyproject.toml" => {
                    let roots = crate::projects::python_roots(path, text);
                    let declared = !roots.is_empty();
                    self.python_roots.extend(declared.then_some((dir, roots)));
                }
                "go.mod" => {
                    let module = crate::projects::go_module(text);
                    self.go_modules.extend(module.map(|module| (dir, module)));
                }
                _ => {
                    let package = crate::projects::cargo_package(text);
                    self.packages.extend(package.map(|package| (dir, package)));
                }
            }
        }
        self.javascript = javascript_config::read(javascript);
        self
    }
}
