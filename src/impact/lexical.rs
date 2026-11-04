//! What `km impact` reads from the text of the whole repository, beyond
//! the graph: copies of changed values, and the HTTP level.

use std::collections::HashSet;
use std::error::Error;
use std::path::Path;

use crate::git::{ChangeKind, FileDiffStat, GitRepo};
use crate::projects::ProjectGraph;
use crate::util::is_generated;

use super::copies;
use super::http;
use super::source::Change;
use super::structural::Structural;

/// Whether a file outside a change may hold a copy of its values: one of a
/// known language, not a document nor a generated file.
fn may_hold_copies(path: &Path) -> bool {
    crate::loc::language::detect(path)
        .is_some_and(|spec| crate::kind::Kind::by_name(path, spec) != crate::kind::Kind::Docs)
        && !is_generated(path)
}

/// The groups of values outside the change that hold what it replaced.
/// The rest of the repository is read only when the change modifies such a
/// group.
pub fn copies_of(
    git_repo: &GitRepo,
    change: &Change,
    changes: &[FileDiffStat],
    in_diff: &HashSet<&Path>,
) -> Result<Vec<copies::Copy>, Box<dyn Error>> {
    let kept: Vec<&FileDiffStat> = changes
        .iter()
        .filter(|c| c.kind != ChangeKind::Deleted && may_hold_copies(&c.path))
        .collect();
    let wanted: HashSet<&Path> = kept.iter().map(|c| c.path.as_path()).collect();
    let heads = change.sources(git_repo, |p| wanted.contains(p))?;
    let edits: Vec<copies::Edit> = kept
        .iter()
        .flat_map(|c| {
            let head = heads.iter().find(|(p, _)| *p == c.path);
            head.map(|(_, text)| copies::edits(c, text))
                .unwrap_or_default()
        })
        .collect();
    if edits.is_empty() {
        return Ok(Vec::new());
    }
    let files = change.sources(git_repo, |p| !in_diff.contains(p) && may_hold_copies(p))?;
    Ok(copies::copies(&edits, &files))
}

/// The HTTP level of the change, read over every file that may hold
/// routes or client calls.
pub fn http_level(
    git_repo: &GitRepo,
    change: &Change,
    graph: &ProjectGraph,
    changes: &[FileDiffStat],
    structural: &Structural,
) -> Result<http::Http, Box<dyn Error>> {
    let sources = change.sources(git_repo, |p| http::is_read(p) && !is_generated(p))?;
    let project_of = |p: &Path| graph.owner(p).map(|i| graph.projects[i].name.clone());
    let repository = http::Repository {
        sources: &sources,
        project_of: &project_of,
    };
    Ok(http::measure(&repository, changes, &structural.narrowing))
}
