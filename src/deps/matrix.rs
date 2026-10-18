//! A dependency structure matrix: rows and columns are the same directories
//! and files, in the order of `matrix_tree`, and a cell counts the files of
//! its column that use files of its row. A directory shown collapsed counts
//! for everything under it.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use super::edges::Edge;
use super::graph::EdgeKind;
use super::matrix_tree::{Node, ROOT, Tree};

/// Which directories are shown expanded.
pub enum Expand {
    /// Those less than this many levels below the root.
    Depth(usize),
    /// This one and the ones above it.
    Focus(PathBuf),
}

/// A line of the matrix: a directory shown expanded, which heads the rows
/// of what it holds, or a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    Group { node: usize, level: usize },
    Row { node: usize, level: usize },
}

/// How a cell stands out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// One of its uses is part of a cycle between files.
    Cycle,
    /// Above the diagonal: the row is used by a column it comes before.
    Layering,
    /// Only uses that make no cycle: type-only or deferred imports.
    Weak,
    Plain,
}

#[derive(Debug, Default)]
pub struct Cell {
    pub count: usize,
    /// Some use loads or compiles the file used.
    strong: bool,
    cycle: bool,
    /// The uses behind the count, by index into `Matrix::edges`.
    pub edges: Vec<usize>,
}

impl Cell {
    pub fn mark(&self, above: bool) -> Mark {
        match () {
            _ if self.cycle => Mark::Cycle,
            _ if !self.strong => Mark::Weak,
            _ if above => Mark::Layering,
            _ => Mark::Plain,
        }
    }
}

pub struct Matrix {
    pub tree: Tree,
    pub lines: Vec<Line>,
    /// The node of each row.
    pub rows: Vec<usize>,
    /// By (row, column): the row is used, the column uses it.
    pub cells: BTreeMap<(usize, usize), Cell>,
    pub edges: Vec<Edge>,
}

impl Expand {
    /// What `--depth` and `--focus` ask for; one level when neither is given.
    pub fn of(depth: Option<u32>, focus: Option<&str>) -> Expand {
        match focus {
            Some(dir) => Expand::Focus(PathBuf::from(
                dir.trim_start_matches("./").trim_end_matches('/'),
            )),
            None => Expand::Depth(depth.unwrap_or(1) as usize),
        }
    }
}

fn strong(kind: EdgeKind) -> bool {
    kind <= EdgeKind::Load
}

impl Matrix {
    /// The matrix of `files`, with the edges between them and the cycles
    /// they form.
    pub fn build(
        files: &[PathBuf],
        edges: &[Edge],
        cycles: &[Vec<PathBuf>],
        expand: &Expand,
    ) -> Result<Matrix, String> {
        let index: HashMap<&Path, usize> = files
            .iter()
            .enumerate()
            .map(|(i, f)| (f.as_path(), i))
            .collect();
        let ends = |e: &Edge| Some((*index.get(e.from.as_path())?, *index.get(e.to.as_path())?));
        let (edges, ends): (Vec<Edge>, Vec<(usize, usize)>) = edges
            .iter()
            .filter_map(|e| Some((e.clone(), ends(e)?)))
            .unzip();
        let mut tree = Tree::of(files);
        let uses = edges.iter().zip(&ends).filter(|(e, _)| strong(e.kind));
        tree.order(&uses.map(|(_, &pair)| pair).collect::<Vec<_>>());

        let lines = visible(&tree, expand)?;
        let rows: Vec<usize> = lines
            .iter()
            .filter_map(|line| match line {
                Line::Row { node, .. } => Some(*node),
                Line::Group { .. } => None,
            })
            .collect();
        let mut row_of = vec![0; files.len()];
        for (r, &node) in rows.iter().enumerate() {
            files_under(&tree, node)
                .into_iter()
                .for_each(|f| row_of[f] = r);
        }
        let ends: Vec<(usize, usize)> = ends.iter().map(|&(f, t)| (row_of[f], row_of[t])).collect();
        let cells = cells(&edges, &ends, cycles);
        Ok(Matrix {
            tree,
            lines,
            rows,
            cells,
            edges,
        })
    }

    /// The cells above the diagonal, by row and column.
    pub fn above(&self) -> impl Iterator<Item = (usize, usize, &Cell)> {
        self.cells
            .iter()
            .filter(|((row, col), _)| row < col)
            .map(|(&(row, col), cell)| (row, col, cell))
    }

    pub fn node(&self, row: usize) -> &Node {
        &self.tree.nodes[self.rows[row]]
    }
}

/// The cells the edges fall in, given the rows of their ends.
fn cells(
    edges: &[Edge],
    rows: &[(usize, usize)],
    cycles: &[Vec<PathBuf>],
) -> BTreeMap<(usize, usize), Cell> {
    let cycle_of: HashMap<&Path, usize> = cycles
        .iter()
        .enumerate()
        .flat_map(|(c, files)| files.iter().map(move |f| (f.as_path(), c)))
        .collect();
    let of = |p: &PathBuf| cycle_of.get(p.as_path());
    let in_cycle = |e: &Edge| strong(e.kind) && of(&e.from).is_some() && of(&e.from) == of(&e.to);
    let mut cells: BTreeMap<(usize, usize), Cell> = BTreeMap::new();
    let between_rows = edges
        .iter()
        .zip(rows)
        .enumerate()
        .filter(|(_, (_, (f, t)))| f != t);
    for (i, (edge, &(from, to))) in between_rows {
        // The row is the file used, the column the file using it.
        let cell = cells.entry((to, from)).or_default();
        cell.count += 1;
        cell.strong |= strong(edge.kind);
        cell.cycle |= in_cycle(edge);
        cell.edges.push(i);
    }
    cells
}

fn files_under(tree: &Tree, node: usize) -> Vec<usize> {
    let mut found = Vec::new();
    let mut pending = vec![node];
    while let Some(n) = pending.pop() {
        found.extend(tree.nodes[n].file);
        pending.extend(&tree.nodes[n].children);
    }
    found
}

/// Whether a directory is shown expanded, given the path of its parent.
fn expanded(expand: &Expand, node: &Node, level: usize, parent: &Path) -> bool {
    match expand {
        Expand::Depth(depth) => level + 1 < *depth,
        // The focus, the directories above it, and the topmost one under it
        // when the focus lies inside a directory merged with its only child.
        Expand::Focus(focus) => {
            focus.starts_with(&node.path)
                || (node.path.starts_with(focus) && !parent.starts_with(focus))
        }
    }
}

/// Whether `focus` names the root or a directory of the tree, or lies
/// inside a merged one.
fn found(tree: &Tree, focus: &Path) -> bool {
    let is = |n: &Node| n.is_dir() && n.path.starts_with(focus);
    tree.nodes[ROOT].path.starts_with(focus) || tree.nodes.iter().any(is)
}

/// The lines shown, given which directories are expanded.
fn visible(tree: &Tree, expand: &Expand) -> Result<Vec<Line>, String> {
    if let Expand::Focus(focus) = expand
        && !found(tree, focus)
    {
        return Err(format!(
            "--focus {}: no directory of the graph",
            focus.display()
        ));
    }
    let root = tree.nodes[ROOT].path.as_path();
    let mut lines = Vec::new();
    // Each node with its level and the path of its parent.
    let mut pending: Vec<(usize, usize, &Path)> = tree.nodes[ROOT]
        .children
        .iter()
        .rev()
        .map(|&c| (c, 0, root))
        .collect();
    while let Some((node, level, parent)) = pending.pop() {
        let n = &tree.nodes[node];
        if !n.is_dir() || !expanded(expand, n, level, parent) {
            lines.push(Line::Row { node, level });
            continue;
        }
        lines.push(Line::Group { node, level });
        let children = n.children.iter().rev();
        pending.extend(children.map(|&child| (child, level + 1, n.path.as_path())));
    }
    Ok(lines)
}

#[cfg(test)]
#[path = "matrix_test.rs"]
mod tests;
