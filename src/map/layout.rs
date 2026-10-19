//! The map laid out: every directory a circle holding the circles of what
//! it holds, a file's area proportional to its lines of code. Children are
//! packed largest first, then by name, so the layout is the same on every
//! run; the root fills a circle of `SIZE` across.

use std::path::PathBuf;

use serde::Serialize;

use super::metric::Shade;
use super::pack;
use crate::file_tree::{ROOT, Tree};

/// The diameter of the whole map, in the units of the page.
pub const SIZE: f64 = 1000.0;

/// Room around the children inside a directory, in units of a leaf radius.
const PADDING: f64 = 2.0;

#[derive(Debug, Serialize)]
pub struct Placed {
    /// As shown: `deps/`, `lib/core/`, `graph.rs`.
    pub name: String,
    pub path: String,
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub depth: usize,
    /// The index of the directory holding it; none for the root.
    pub parent: Option<usize>,
    pub dir: bool,
    /// Code lines, of the file or of everything under the directory.
    pub lines: usize,
    #[serde(flatten)]
    pub shade: Option<Shade>,
}

/// Lay out `files` (paths relative to the analysed directory, with their
/// code lines and shades), the root first and every node after its parent.
pub fn layout(files: &[(PathBuf, usize, Option<Shade>)]) -> Vec<Placed> {
    let paths: Vec<PathBuf> = files.iter().map(|(p, _, _)| p.clone()).collect();
    let tree = Tree::of(&paths);
    let mut lines = vec![0usize; tree.nodes.len()];
    let mut radius = vec![0f64; tree.nodes.len()];
    // Relative to the parent's centre, before scaling.
    let mut offset = vec![(0f64, 0f64); tree.nodes.len()];
    let order = post_order(&tree);
    for &node in &order {
        let n = &tree.nodes[node];
        if let Some(file) = n.file {
            lines[node] = files[file].1;
            radius[node] = (files[file].1.max(1) as f64).sqrt();
            continue;
        }
        let mut children = n.children.clone();
        children.sort_by(|&a, &b| {
            radius[b]
                .total_cmp(&radius[a])
                .then(tree.nodes[a].name.cmp(&tree.nodes[b].name))
        });
        let padded: Vec<f64> = children.iter().map(|&c| radius[c] + PADDING).collect();
        let (circles, r) = pack::siblings(&padded);
        for (&c, circle) in children.iter().zip(circles) {
            offset[c] = (circle.x, circle.y);
        }
        radius[node] = r + PADDING;
        lines[node] = n.children.iter().map(|&c| lines[c]).sum();
    }
    let scale = SIZE / 2.0 / radius[ROOT].max(f64::MIN_POSITIVE);
    let mut placed = Vec::with_capacity(order.len());
    // Each node with its parent's index in `placed`, its centre and depth.
    let mut pending = vec![(ROOT, None, (SIZE / 2.0, SIZE / 2.0), 0usize)];
    while let Some((node, parent, (x, y), depth)) = pending.pop() {
        let n = &tree.nodes[node];
        let index = placed.len();
        placed.push(Placed {
            name: n.name.clone(),
            path: n.path.display().to_string(),
            x,
            y,
            r: radius[node] * scale,
            depth,
            parent,
            dir: n.is_dir(),
            lines: lines[node],
            shade: n.file.and_then(|f| files[f].2.clone()),
        });
        for &c in n.children.iter().rev() {
            let (dx, dy) = offset[c];
            pending.push((c, Some(index), (x + dx * scale, y + dy * scale), depth + 1));
        }
    }
    placed
}

/// The nodes reachable from the root, children before their parent.
fn post_order(tree: &Tree) -> Vec<usize> {
    let mut order = Vec::new();
    let mut pending = vec![(ROOT, false)];
    while let Some((node, done)) = pending.pop() {
        if done {
            order.push(node);
            continue;
        }
        pending.push((node, true));
        pending.extend(tree.nodes[node].children.iter().map(|&c| (c, false)));
    }
    order
}

#[cfg(test)]
#[path = "layout_test.rs"]
mod tests;
