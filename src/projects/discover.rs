//! Finding the manifests of a repository and resolving what they declare
//! into projects and the edges between them.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use ignore::WalkBuilder;

use super::{DepTarget, Ecosystem, Edge, Manifest, Project, ProjectGraph, Scope, reader};
use crate::walk::TEST_DIRS;

/// Directories that hold fetched dependencies or build output. Their
/// manifests belong to other people's projects.
const SKIPPED_DIRS: &[&str] = &[
    "testdata",
    ".git",
    "node_modules",
    "deps",
    "_build",
    "target",
    "vendor",
];

/// Directories of sample projects that tests read: manifests, but nobody's
/// project. A whole test directory is not skipped, since an end-to-end suite
/// with its own manifest is a project that depends on what it exercises.
const FIXTURE_DIRS: &[&str] = &["fixtures", "__fixtures__"];

/// Whether a search for manifests should stay out of the directory `path`,
/// given relative to the repository.
pub fn is_skipped(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let in_tests = || {
        path.components()
            .filter_map(|c| c.as_os_str().to_str())
            .any(|c| TEST_DIRS.contains(&c))
    };
    SKIPPED_DIRS.contains(&name) || (FIXTURE_DIRS.contains(&name) && in_tests())
}

/// One manifest found in the walk.
struct Found {
    dir: PathBuf,
    path: PathBuf,
    ecosystem: Ecosystem,
    manifest: Manifest,
}

/// Resolve `.` and `..` without touching the filesystem. `None` when the
/// path leaves the repository.
fn normalize(path: &Path) -> Option<PathBuf> {
    let mut parts: Vec<Component> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.iter().collect())
}

/// Whether a file is a manifest some reader knows, by its name.
pub fn is_manifest(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| reader(name).is_some())
}

/// What the manifest at `path`, relative to the repository, declares.
fn read_manifest(path: &Path, source: &str) -> Option<Found> {
    let (ecosystem, read) = reader(path.file_name()?.to_str()?)?;
    Some(Found {
        dir: path.parent().unwrap_or(Path::new("")).to_path_buf(),
        path: path.to_path_buf(),
        ecosystem,
        manifest: read(source),
    })
}

/// The text files under `root` on disk that `wanted` picks, outside the
/// skipped directories, each with its path relative to `root`.
pub fn files_on_disk(root: &Path, wanted: impl Fn(&Path) -> bool) -> Vec<(PathBuf, String)> {
    let relative = |path: &Path| path.strip_prefix(root).unwrap_or(path).to_path_buf();
    let walk = WalkBuilder::new(root)
        .hidden(false)
        .filter_entry({
            let root = root.to_path_buf();
            move |entry| !is_skipped(entry.path().strip_prefix(&root).unwrap_or(entry.path()))
        })
        .build();

    walk.flatten()
        .map(|entry| relative(entry.path()))
        .filter(|path| wanted(path))
        .filter_map(|path| {
            let text = std::fs::read_to_string(root.join(&path)).ok()?;
            Some((path, text))
        })
        .collect()
}

/// The name of a directory, for a package that gives itself none.
fn dir_name(dir: &Path) -> String {
    dir.file_name()
        .map_or_else(|| "root".to_string(), |n| n.to_string_lossy().into_owned())
}

/// A dependency resolved against the projects of the repository.
enum Resolved {
    Project(usize),
    /// Not a project of this repository.
    External,
    /// Meant to be local, pointing at no project.
    Unread,
}

/// The projects of the repository, and the tables that resolve what a
/// manifest declares into one of them.
#[derive(Default)]
struct Resolver {
    projects: Vec<Project>,
    by_root: HashMap<PathBuf, usize>,
    by_name: HashMap<(Ecosystem, String), usize>,
    /// Workspace root directory → dependency name → project directory.
    workspaces: HashMap<PathBuf, HashMap<String, PathBuf>>,
}

impl Resolver {
    /// Register the workspace `found` is the root of, with the dependencies
    /// it offers its members.
    fn register_workspace(&mut self, found: &Found) {
        if !found.manifest.is_workspace_root {
            return;
        }
        let offered = found
            .manifest
            .workspace_paths
            .iter()
            .filter_map(|(name, p)| Some((name.clone(), normalize(&found.dir.join(p))?)));
        self.workspaces
            .entry(found.dir.clone())
            .or_default()
            .extend(offered);
    }

    /// The name of the project `found` declares, if it declares one.
    ///
    /// A `package.json` has no way to say it is only a workspace root, so one
    /// at the root of a workspace is taken as that and nothing else; any
    /// other is a project, even without a name.
    fn project_name(&self, found: &Found) -> Option<String> {
        match found.ecosystem {
            Ecosystem::Npm if self.workspaces.contains_key(&found.dir) => None,
            Ecosystem::Npm if found.path.ends_with("package.json") => Some(
                found
                    .manifest
                    .name
                    .clone()
                    .unwrap_or_else(|| dir_name(&found.dir)),
            ),
            _ => found.manifest.name.clone(),
        }
    }

    /// Register the project `found` declares. A directory is one project,
    /// whatever the number of manifests in it.
    fn register_project(&mut self, found: &Found) {
        let Some(name) = self.project_name(found) else {
            return;
        };
        let next = self.projects.len();
        let index = *self.by_root.entry(found.dir.clone()).or_insert(next);
        if index == next {
            self.projects.push(Project {
                root: found.dir.clone(),
                name: name.clone(),
            });
        }
        self.by_name.insert((found.ecosystem, name), index);
    }

    fn at_path(&self, dir: &Path, relative: &Path) -> Resolved {
        normalize(&dir.join(relative))
            .and_then(|p| self.by_root.get(&p))
            .map_or(Resolved::Unread, |&i| Resolved::Project(i))
    }

    fn resolve(&self, found: &Found, target: &DepTarget) -> Resolved {
        match target {
            DepTarget::Path(relative) => self.at_path(&found.dir, relative),
            DepTarget::Name(name) => self
                .by_name
                .get(&(found.ecosystem, name.clone()))
                .map_or(Resolved::External, |&i| Resolved::Project(i)),
            DepTarget::Workspace(name) => found
                .dir
                .ancestors()
                .find_map(|dir| self.workspaces.get(dir))
                .and_then(|table| table.get(name))
                .map_or(Resolved::External, |dir| self.at_path(Path::new(""), dir)),
        }
    }

    /// The projects `found` depends on, with the scope of each dependency,
    /// and how many of its local dependencies point at no project.
    fn dependencies(&self, found: &Found) -> (Vec<(usize, Scope)>, usize) {
        let mut local = Vec::new();
        let mut unread = found.manifest.unread;
        for dep in &found.manifest.deps {
            match self.resolve(found, &dep.target) {
                Resolved::Project(to) => local.push((to, dep.scope)),
                Resolved::Unread => unread += 1,
                Resolved::External => {}
            }
        }
        (local, unread)
    }
}

impl ProjectGraph {
    /// Read the manifests under `root` and build the graph of its projects.
    pub fn discover(root: &Path) -> Self {
        Self::from_manifests(files_on_disk(root, is_manifest))
    }

    /// Build the graph from manifests given as their path, relative to the
    /// repository, and their text. A file no reader knows is ignored.
    pub fn from_manifests(manifests: Vec<(PathBuf, String)>) -> Self {
        let mut found: Vec<Found> = manifests
            .iter()
            .filter_map(|(path, text)| read_manifest(path, text))
            .collect();
        found.sort_by(|a, b| a.path.cmp(&b.path));
        let mut resolver = Resolver::default();
        // Workspace roots first: whether a manifest is a project depends on them.
        for f in &found {
            resolver.register_workspace(f);
        }
        for f in &found {
            resolver.register_project(f);
        }

        // A dependency declared more than once keeps its strongest scope.
        let mut scopes: HashMap<(usize, usize), Scope> = HashMap::new();
        let mut unread = Vec::new();
        for f in &found {
            let (local, missing) = resolver.dependencies(f);
            if missing > 0 {
                unread.push((f.path.clone(), missing));
            }
            let Some(&from) = resolver.by_root.get(&f.dir) else {
                continue;
            };
            for (to, scope) in local.into_iter().filter(|&(to, _)| to != from) {
                let kept = scopes.entry((from, to)).or_insert(scope);
                *kept = scope.min(*kept);
            }
        }

        let mut edges: Vec<Edge> = scopes
            .into_iter()
            .map(|((from, to), scope)| Edge { from, to, scope })
            .collect();
        edges.sort_by_key(|e| (e.to, e.from));
        let mut workspaces: Vec<PathBuf> = resolver.workspaces.into_keys().collect();
        workspaces.sort();
        ProjectGraph {
            projects: resolver.projects,
            edges,
            workspaces,
            unread,
        }
    }
}
