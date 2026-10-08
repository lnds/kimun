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
}

/// Whether `path` is the manifest of a Cargo package.
pub fn is_cargo_manifest(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "Cargo.toml")
}

impl Layout<'_> {
    /// Note the package each of the Cargo `manifests` declares, given as
    /// path and text.
    pub fn with_cargo<'t>(
        mut self,
        manifests: impl IntoIterator<Item = (&'t Path, &'t str)>,
    ) -> Self {
        self.packages = manifests
            .into_iter()
            .filter_map(|(path, text)| {
                let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
                Some((dir, crate::projects::cargo_package(text)?))
            })
            .collect();
        self
    }
}
