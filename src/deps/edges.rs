//! The file-level edges of the graph, kept for the outputs that show which
//! file uses which.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::analyzer::DepEntry;
use super::graph::EdgeKind;

/// One file using another.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    pub from: PathBuf,
    pub to: PathBuf,
    pub kind: EdgeKind,
}

/// JSON-serializable edge.
#[derive(Serialize)]
pub struct JsonEdge {
    pub from: String,
    pub to: String,
    pub kind: &'static str,
}

impl From<&Edge> for JsonEdge {
    fn from(edge: &Edge) -> Self {
        JsonEdge {
            from: edge.from.display().to_string(),
            to: edge.to.display().to_string(),
            kind: edge.kind.name(),
        }
    }
}

/// The edges of an adjacency list over `files` by index, sorted.
pub fn of(files: &[(PathBuf, String)], adj: &[Vec<(usize, EdgeKind)>]) -> Vec<Edge> {
    let mut edges: Vec<Edge> = adj
        .iter()
        .enumerate()
        .flat_map(|(from, out)| {
            out.iter().map(move |&(to, kind)| Edge {
                from: files[from].0.clone(),
                to: files[to].0.clone(),
                kind,
            })
        })
        .collect();
    edges.sort();
    edges
}

/// The edges that leave the files listed.
pub fn leaving(edges: &[Edge], entries: &[&DepEntry]) -> Vec<Edge> {
    let listed: HashSet<&Path> = entries.iter().map(|e| e.path.as_path()).collect();
    let from_listed = edges.iter().filter(|e| listed.contains(e.from.as_path()));
    from_listed.cloned().collect()
}
