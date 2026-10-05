//! Blast radius at project level: the projects of the repository that depend,
//! directly or through others, on the projects a diff touches.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::projects::{ProjectGraph, Reach};
use crate::report_helpers;

/// Scope of a project reached because its workspace changed, next to the
/// labels of `Scope`.
const WORKSPACE: &str = "workspace";

/// A project reached by the change, with the path that explains it.
#[derive(Debug, Clone, PartialEq)]
pub struct ReachRow {
    pub project: String,
    /// The changed project or workspace root the reach starts from.
    pub origin: String,
    pub distance: usize,
    /// The project in between, for a reach that is not direct.
    pub via: Option<String>,
    /// Why it is reached: empty for a dependency that ships, `dev`, `build`,
    /// `optional`, or `workspace` for a member of a changed workspace.
    pub scope: &'static str,
}

/// The reach of a diff over the projects of the repository.
#[derive(Debug, Default, PartialEq)]
pub struct ProjectRadius {
    /// Every project in the repository.
    pub all: Vec<String>,
    /// Projects holding a changed file.
    pub changed: Vec<String>,
    /// Projects reached from what changed, nearest first.
    pub reached: Vec<ReachRow>,
    /// Changed files that belong to no project: their reach is unknown.
    pub outside: Vec<PathBuf>,
    /// Manifests with local dependencies that could not be read.
    pub unread: Vec<(PathBuf, usize)>,
}

impl ProjectRadius {
    /// Projects in the repository.
    pub fn total(&self) -> usize {
        self.all.len()
    }

    /// Projects that depend on what changed without intermediaries.
    pub fn direct(&self) -> usize {
        self.reached.iter().filter(|r| r.distance == 1).count()
    }

    /// The projects whose builds and tests the diff calls for, sorted: the
    /// changed and the reached ones. When a changed file belongs to no
    /// project its reach is unknown, and every project is affected: running
    /// too much is the safe side of not knowing.
    pub fn affected(&self) -> Vec<&str> {
        if !self.outside.is_empty() {
            return self.all.iter().map(String::as_str).collect();
        }
        let affected: BTreeSet<&str> = self
            .changed
            .iter()
            .map(String::as_str)
            .chain(self.reached.iter().map(|r| r.project.as_str()))
            .collect();
        affected.into_iter().collect()
    }
}

fn display(dir: &Path) -> String {
    if dir.as_os_str().is_empty() {
        ".".to_string()
    } else {
        dir.display().to_string()
    }
}

/// Where each changed file lands: in a project, on a workspace as a whole,
/// or nowhere.
#[derive(Default)]
struct Landing<'a> {
    projects: BTreeSet<usize>,
    workspaces: BTreeSet<&'a Path>,
    outside: BTreeSet<PathBuf>,
}

impl<'a> Landing<'a> {
    fn of(graph: &'a ProjectGraph, files: impl Iterator<Item = &'a Path>) -> Self {
        let mut landing = Self::default();
        for file in files {
            let workspace = graph.workspace_of(file);
            let project = graph.owner(file);
            landing.workspaces.extend(workspace);
            landing.projects.extend(project);
            if workspace.is_none() && project.is_none() {
                landing.outside.insert(file.to_path_buf());
            }
        }
        landing
    }

    /// The unchanged projects under a changed workspace, each with the
    /// workspace that reaches it.
    fn members(&self, graph: &ProjectGraph) -> BTreeMap<usize, &'a Path> {
        let mut members = BTreeMap::new();
        for &workspace in &self.workspaces {
            for project in graph.members(workspace) {
                if !self.projects.contains(&project) {
                    members.entry(project).or_insert(workspace);
                }
            }
        }
        members
    }
}

/// Measure the reach of `changed_files` over the projects of `graph`.
///
/// The manifest or the lock file of a workspace root reaches every project
/// under it at distance 1, and from them on like any other change.
pub fn compute<'a>(
    graph: &'a ProjectGraph,
    changed_files: impl Iterator<Item = &'a Path>,
) -> ProjectRadius {
    let landing = Landing::of(graph, changed_files);
    let members = landing.members(graph);
    let root = |project: usize| graph.projects[project].display_root();

    let seeds: BTreeSet<usize> = landing
        .projects
        .iter()
        .chain(members.keys())
        .copied()
        .collect();
    // A reach that starts at a member starts, in fact, at its workspace.
    let through = |r: Reach| match members.get(&r.origin) {
        Some(workspace) => ReachRow {
            project: root(r.project),
            origin: display(workspace),
            distance: r.distance + 1,
            via: Some(root(r.via)),
            scope: r.scope.label(),
        },
        None => ReachRow {
            project: root(r.project),
            origin: root(r.origin),
            distance: r.distance,
            via: (r.via != r.origin).then(|| root(r.via)),
            scope: r.scope.label(),
        },
    };
    let mut reached: Vec<ReachRow> = members
        .iter()
        .map(|(&project, workspace)| ReachRow {
            project: root(project),
            origin: display(workspace),
            distance: 1,
            via: None,
            scope: WORKSPACE,
        })
        .chain(graph.radius(&seeds).into_iter().map(through))
        .collect();
    reached.sort_by(|a, b| (a.distance, &a.project).cmp(&(b.distance, &b.project)));

    let mut all: Vec<String> = (0..graph.projects.len()).map(root).collect();
    all.sort();
    ProjectRadius {
        all,
        changed: landing.projects.into_iter().map(root).collect(),
        reached,
        outside: landing.outside.into_iter().collect(),
        unread: graph.unread.clone(),
    }
}

const TITLE: &str = "Blast radius — projects reached through their manifests";
const HEADERS: [&str; 5] = ["Changed", "Reaches", "Distance", "Scope", "Via"];

/// The cells of one table row.
fn cells(row: &ReachRow) -> [String; 5] {
    [
        row.origin.clone(),
        row.project.clone(),
        row.distance.to_string(),
        row.scope.to_string(),
        row.via.clone().unwrap_or_default(),
    ]
}

/// The table of reached projects, with each column as wide as its content.
fn table(rows: &[ReachRow]) -> Vec<String> {
    let rows: Vec<[String; 5]> = std::iter::once(HEADERS.map(str::to_string))
        .chain(rows.iter().map(cells))
        .collect();
    let width = |col: usize| {
        rows.iter()
            .map(|r| report_helpers::display_width(&r[col]))
            .max()
            .unwrap_or(0)
    };
    let widths = [width(0), width(1), width(2), width(3)];
    rows.iter()
        .map(|[origin, project, distance, scope, via]| {
            format!(
                " {}  {}  {distance:>dw$}  {}  {via}",
                report_helpers::pad_to(origin, widths[0]),
                report_helpers::pad_to(project, widths[1]),
                report_helpers::pad_to(scope, widths[3]),
                dw = widths[2],
            )
            .trim_end()
            .to_string()
        })
        .collect()
}

/// What the radius comes to, in one or more lines.
fn body(radius: &ProjectRadius) -> Vec<String> {
    let total = radius.total();
    let one_line = |line: String| vec![line];
    if total == 0 {
        return one_line(
            " No project manifests found (Cargo.toml, package.json, mix.exs).".to_string(),
        );
    }
    if total == 1 {
        return one_line(format!(
            " Single project ({}): no other project to reach; this level does not apply.",
            radius.all[0]
        ));
    }
    if radius.reached.is_empty() && radius.changed.is_empty() {
        return one_line(" No project changed.".to_string());
    }
    if radius.reached.is_empty() {
        return one_line(format!(
            " 0 of {total} projects reached: nothing depends on {}.",
            radius.changed.join(", ")
        ));
    }
    let share = 100.0 * radius.reached.len() as f64 / total as f64;
    let mut lines = vec![
        format!(
            " {} of {total} projects reached ({share:.0}%), {} direct",
            radius.reached.len(),
            radius.direct()
        ),
        String::new(),
    ];
    lines.extend(table(&radius.reached));
    lines
}

/// Notes on what the manifests could not say.
fn notes(radius: &ProjectRadius) -> Vec<String> {
    let mut notes = Vec::new();
    if !radius.outside.is_empty() {
        let files: Vec<String> = radius
            .outside
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        notes.push(format!(
            "Changed files outside every project (reach unknown): {}",
            files.join(", ")
        ));
    }
    if !radius.unread.is_empty() {
        let manifests: Vec<String> = radius
            .unread
            .iter()
            .map(|(path, count)| format!("{} ({count})", path.display()))
            .collect();
        notes.push(format!(
            "Local dependencies that could not be read: {}",
            manifests.join(", ")
        ));
    }
    notes
}

/// The project block of the report.
pub fn render(radius: &ProjectRadius) -> Vec<String> {
    let body = body(radius);
    let width = body
        .iter()
        .map(|l| report_helpers::display_width(l))
        .max()
        .unwrap_or(0)
        .max(78);
    let sep = report_helpers::separator(width);

    let mut lines = vec![TITLE.to_string(), sep.clone()];
    lines.extend(body);
    lines.push(sep);
    lines.extend(notes(radius));
    lines
}

#[derive(Serialize)]
struct JsonReach {
    project: String,
    origin: String,
    distance: usize,
    via: Option<String>,
    scope: &'static str,
}

#[derive(Serialize)]
struct JsonUnread {
    manifest: String,
    dependencies: usize,
}

/// The project block of the JSON report.
#[derive(Serialize)]
pub struct JsonProjects {
    total: usize,
    changed: Vec<String>,
    reached: Vec<JsonReach>,
    direct: usize,
    affected: Vec<String>,
    outside: Vec<String>,
    unread: Vec<JsonUnread>,
}

impl From<&ProjectRadius> for JsonProjects {
    fn from(radius: &ProjectRadius) -> Self {
        Self {
            total: radius.total(),
            changed: radius.changed.clone(),
            reached: radius
                .reached
                .iter()
                .map(|r| JsonReach {
                    project: r.project.clone(),
                    origin: r.origin.clone(),
                    distance: r.distance,
                    via: r.via.clone(),
                    scope: if r.scope.is_empty() {
                        "runtime"
                    } else {
                        r.scope
                    },
                })
                .collect(),
            direct: radius.direct(),
            affected: radius.affected().into_iter().map(str::to_string).collect(),
            outside: radius
                .outside
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
            unread: radius
                .unread
                .iter()
                .map(|(path, count)| JsonUnread {
                    manifest: path.display().to_string(),
                    dependencies: *count,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
#[path = "projects_test.rs"]
mod tests;
