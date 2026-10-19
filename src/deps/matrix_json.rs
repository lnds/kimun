//! The dependency matrix as JSON, for tools and models: the rows in their
//! order, every cell with its mark and the uses behind it, and the cells
//! above the diagonal.

use serde::Serialize;

use super::analyzer::UnsupportedLanguage;
use super::matrix::{Line, Mark, Matrix};

#[derive(Serialize)]
pub struct JsonMatrix {
    /// Directories shown expanded, which head the rows under them.
    pub groups: Vec<String>,
    pub rows: Vec<JsonRow>,
    pub cells: Vec<JsonCell>,
    /// The cells above the diagonal, as `[row, column]`.
    pub above_diagonal: Vec<(usize, usize)>,
    pub unsupported: Vec<UnsupportedLanguage>,
}

#[derive(Serialize)]
pub struct JsonRow {
    /// From 1, as the text numbers it; the column of the row has it too.
    pub number: usize,
    /// A directory ends in `/` and counts for every file under it.
    pub path: String,
    pub level: usize,
}

#[derive(Serialize)]
pub struct JsonCell {
    /// The row used.
    pub row: usize,
    /// The column using it.
    pub column: usize,
    pub count: usize,
    /// `cycle`, `layering` (against it), `weak` (type-only or deferred
    /// uses alone) or `plain`.
    pub mark: &'static str,
    pub uses: Vec<JsonUse>,
}

#[derive(Serialize)]
pub struct JsonUse {
    pub from: String,
    pub to: String,
    pub kind: &'static str,
    pub cycle: bool,
}

fn mark_name(mark: Mark) -> &'static str {
    match mark {
        Mark::Cycle => "cycle",
        Mark::Layering => "layering",
        Mark::Weak => "weak",
        Mark::Plain => "plain",
    }
}

fn shown_path(m: &Matrix, node: usize) -> String {
    let n = &m.tree.nodes[node];
    let slash = if n.is_dir() { "/" } else { "" };
    format!("{}{slash}", n.path.display())
}

/// The JSON of `m`, or an empty matrix when no file was analysed.
pub fn of(m: Option<&Matrix>, unsupported: &[UnsupportedLanguage]) -> JsonMatrix {
    let mut json = JsonMatrix {
        groups: vec![],
        rows: vec![],
        cells: vec![],
        above_diagonal: vec![],
        unsupported: unsupported.to_vec(),
    };
    let Some(m) = m else { return json };
    for line in &m.lines {
        match *line {
            Line::Group { node, .. } => json.groups.push(shown_path(m, node)),
            Line::Row { node, level } => json.rows.push(JsonRow {
                number: json.rows.len() + 1,
                path: shown_path(m, node),
                level,
            }),
        }
    }
    for (&(row, col), cell) in &m.cells {
        let uses = cell.edges.iter().map(|&e| JsonUse {
            from: m.edges[e].from.display().to_string(),
            to: m.edges[e].to.display().to_string(),
            kind: m.edges[e].kind.name(),
            cycle: m.cyclic[e],
        });
        json.cells.push(JsonCell {
            row: row + 1,
            column: col + 1,
            count: cell.count,
            mark: mark_name(cell.mark(row < col)),
            uses: uses.collect(),
        });
    }
    json.above_diagonal = m.above().map(|(row, col, _)| (row + 1, col + 1)).collect();
    json
}

#[cfg(test)]
#[path = "matrix_json_test.rs"]
mod tests;
