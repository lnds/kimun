//! Which project owns a file, and which projects a change reaches.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::{ProjectGraph, Reach, Scope};

/// Files that govern every project of the workspace they are at the root
/// of: its manifest, and the lock file that pins what all of them build from.
const WORKSPACE_FILES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "package.json",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "bun.lock",
    "bun.lockb",
    "mix.exs",
    "mix.lock",
];

impl ProjectGraph {
    /// The workspace root `file` governs, when it is the manifest or the
    /// lock file of one.
    pub fn workspace_of(&self, file: &Path) -> Option<&Path> {
        let name = file.file_name()?.to_str()?;
        let dir = file.parent()?;
        WORKSPACE_FILES
            .contains(&name)
            .then(|| self.workspaces.iter().find(|w| w.as_path() == dir))
            .flatten()
            .map(PathBuf::as_path)
    }

    /// Whether the project owning `from` is the one owning `to`, or declares
    /// a dependency on it.
    pub fn may_use(&self, from: &Path, to: &Path) -> bool {
        match (self.owner(from), self.owner(to)) {
            (Some(a), Some(b)) => a == b || self.edges.iter().any(|e| e.from == a && e.to == b),
            _ => false,
        }
    }

    /// The projects under a workspace root.
    pub fn members(&self, workspace: &Path) -> impl Iterator<Item = usize> + '_ {
        let workspace = workspace.to_path_buf();
        self.projects
            .iter()
            .enumerate()
            .filter(move |(_, p)| p.root.starts_with(&workspace))
            .map(|(i, _)| i)
    }

    /// The project a file belongs to: the nearest one above it.
    pub fn owner(&self, file: &Path) -> Option<usize> {
        file.ancestors()
            .skip(1)
            .find_map(|dir| self.projects.iter().position(|p| p.root == dir))
    }

    /// The dependents of `carrier.project` not seen yet, through dev
    /// dependencies or through the others, as reaches one step further.
    fn step(&self, carrier: &Reach, dev: bool, seen: &mut BTreeSet<usize>) -> Vec<Reach> {
        self.edges
            .iter()
            .filter(|e| e.to == carrier.project && (e.scope == Scope::Dev) == dev)
            .filter(|e| seen.insert(e.from))
            .map(|e| Reach {
                project: e.from,
                origin: carrier.origin,
                distance: carrier.distance + 1,
                via: carrier.project,
                scope: e.scope,
            })
            .collect()
    }

    /// The projects reached by a change to `changed`, nearest first.
    ///
    /// A dev dependency reaches the dependent, whose tests or tooling use the
    /// changed project, and stops there: the changed project is not part of
    /// what the dependent delivers, so its own dependents are not reached.
    pub fn radius(&self, changed: &BTreeSet<usize>) -> Vec<Reach> {
        let mut seen = changed.clone();
        // What a change travels through: the changed projects themselves,
        // then every project reached through a dependency that ships.
        let mut carriers: Vec<Reach> = changed
            .iter()
            .map(|&project| Reach {
                project,
                origin: project,
                distance: 0,
                via: project,
                scope: Scope::Runtime,
            })
            .collect();
        let mut next = 0;
        while next < carriers.len() {
            let reached = self.step(&carriers[next], false, &mut seen);
            carriers.extend(reached);
            next += 1;
        }

        // Dev dependents come last, so that a shorter dev path never hides
        // one that ships.
        let dev: Vec<Reach> = carriers
            .iter()
            .flat_map(|carrier| self.step(carrier, true, &mut seen))
            .collect();

        let mut reached: Vec<Reach> = carriers
            .into_iter()
            .skip(changed.len())
            .chain(dev)
            .collect();
        reached.sort_by_key(|r| (r.distance, r.project));
        reached
    }
}
