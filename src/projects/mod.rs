//! Projects of a repository and the dependencies between them, read from
//! their manifests.
//!
//! A project is a directory with a manifest. An edge is a dependency of one
//! project on another project of the same repository; dependencies on
//! registries or other repositories are external and ignored. Each ecosystem
//! contributes a reader that turns a manifest into a [`Manifest`]; discovery,
//! resolution and the reach of a change are shared.

mod cargo;
mod discover;
mod gomod;
mod mix;
mod npm;
mod pyproject;
mod radius;

use std::path::{Path, PathBuf};

pub use discover::{files_on_disk, is_manifest, is_skipped};

/// The name of the package a `Cargo.toml` declares, if it declares one.
pub fn cargo_package(manifest: &str) -> Option<String> {
    cargo::read(manifest).name
}

/// The directories the `pyproject.toml` at `manifest` says its packages are
/// imported from, relative to the repository like `manifest` itself.
pub fn python_roots(manifest: &Path, text: &str) -> Vec<PathBuf> {
    let dir = manifest.parent().unwrap_or(Path::new(""));
    pyproject::import_roots(text)
        .iter()
        .filter_map(|root| discover::normalize(&dir.join(root)))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ecosystem {
    Cargo,
    Npm,
    Mix,
    Go,
    Python,
}

impl Ecosystem {
    /// The form of a project name that names are compared by. A Python
    /// package is the same whatever the case of its name and whichever of
    /// `-`, `_` and `.` separates its parts.
    fn key(self, name: &str) -> String {
        if self != Ecosystem::Python {
            return name.to_string();
        }
        name.split(['-', '_', '.'])
            .filter(|part| !part.is_empty())
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
            .join("-")
    }
}

/// What a dependency is needed for, strongest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    Runtime,
    Build,
    Optional,
    /// Needed only to develop or test the dependent.
    Dev,
}

impl Scope {
    /// Label for reports; a runtime dependency needs none.
    pub fn label(self) -> &'static str {
        match self {
            Scope::Runtime => "",
            Scope::Build => "build",
            Scope::Optional => "optional",
            Scope::Dev => "dev",
        }
    }
}

/// How a manifest names a dependency that may be local.
#[derive(Debug, Clone, PartialEq)]
pub enum DepTarget {
    /// A directory, relative to the manifest.
    Path(PathBuf),
    /// A project name, local only if a project of the same ecosystem has it.
    Name(String),
    /// An entry of the dependency table of the enclosing workspace.
    Workspace(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawDep {
    pub target: DepTarget,
    pub scope: Scope,
}

/// What a reader found in one manifest.
#[derive(Debug, Default, PartialEq)]
pub struct Manifest {
    /// The project name, or `None` when the manifest declares no project
    /// (a workspace or umbrella root).
    pub name: Option<String>,
    pub deps: Vec<RawDep>,
    /// Dependencies a workspace root offers to its members, by name, with
    /// their directory relative to the root.
    pub workspace_paths: Vec<(String, PathBuf)>,
    pub is_workspace_root: bool,
    /// Local dependencies that could not be read, such as a path built at
    /// run time.
    pub unread: usize,
}

/// Turns the text of a manifest into what it declares.
type Reader = fn(&str) -> Manifest;

/// A `pnpm-workspace.yaml` says only that its directory is a workspace root.
fn pnpm_workspace(_source: &str) -> Manifest {
    Manifest {
        is_workspace_root: true,
        ..Manifest::default()
    }
}

/// The reader for a manifest file name.
fn reader(file_name: &str) -> Option<(Ecosystem, Reader)> {
    match file_name {
        "Cargo.toml" => Some((Ecosystem::Cargo, cargo::read)),
        "package.json" => Some((Ecosystem::Npm, npm::read)),
        "pnpm-workspace.yaml" => Some((Ecosystem::Npm, pnpm_workspace)),
        "mix.exs" => Some((Ecosystem::Mix, mix::read)),
        "go.mod" => Some((Ecosystem::Go, gomod::read)),
        "go.work" => Some((Ecosystem::Go, gomod::read_work)),
        "pyproject.toml" => Some((Ecosystem::Python, pyproject::read)),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    /// Directory of the project, relative to the repository; empty at the root.
    pub root: PathBuf,
    pub name: String,
}

impl Project {
    /// The root as shown in reports: `.` for the repository root.
    pub fn display_root(&self) -> String {
        if self.root.as_os_str().is_empty() {
            ".".to_string()
        } else {
            self.root.display().to_string()
        }
    }
}

/// `from` depends on `to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub scope: Scope,
}

/// A project reached by a change to another one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reach {
    pub project: usize,
    /// The changed project the reach starts from.
    pub origin: usize,
    pub distance: usize,
    /// The project it depends on along the way: the origin at distance 1.
    pub via: usize,
    /// Scope of the last dependency of the path.
    pub scope: Scope,
}

#[derive(Debug, Default)]
pub struct ProjectGraph {
    pub projects: Vec<Project>,
    pub edges: Vec<Edge>,
    /// Directories that are the root of a workspace or an umbrella. Their
    /// manifest and lock file govern every project below them.
    pub workspaces: Vec<PathBuf>,
    /// Manifests with local dependencies that could not be attributed to a
    /// project, and how many.
    pub unread: Vec<(PathBuf, usize)>,
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
