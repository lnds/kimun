//! Terraform in the graph: a file that calls a local module uses every
//! `.tf` file of the module's directory, as a Go import uses the files of
//! a package. Read when `--kind infra` asks for it.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use super::graph::{EdgeKind, Found, Source};

/// The Terraform files of a graph, by directory.
pub struct Modules {
    by_dir: HashMap<PathBuf, Vec<usize>>,
}

impl Modules {
    pub fn of(sources: &[Source]) -> Self {
        let mut by_dir: BTreeMap<PathBuf, Vec<usize>> = BTreeMap::new();
        for (at, source) in sources.iter().enumerate() {
            if source.language == "Terraform" {
                let dir = source.path.parent().unwrap_or(Path::new(""));
                by_dir.entry(dir.to_path_buf()).or_default().push(at);
            }
        }
        Modules {
            by_dir: by_dir.into_iter().collect(),
        }
    }

    /// The files of the modules a Terraform source calls; a local source
    /// that leads to no module is unresolved. `None` for any other source.
    pub fn uses(&self, source: &Source, by_path: &HashMap<&Path, usize>) -> Option<Found> {
        if source.language != "Terraform" {
            return None;
        }
        let from = *by_path.get(source.path.as_path())?;
        let mut found = Found::default();
        for (written, dir) in crate::projects::terraform_modules(&source.path, &source.text) {
            match dir.and_then(|d| self.by_dir.get(&d)) {
                Some(files) => {
                    for &to in files.iter().filter(|&&to| to != from) {
                        found.add(to, EdgeKind::Load);
                    }
                }
                None => {
                    found.unresolved.insert(written);
                }
            }
        }
        Some(found)
    }
}
