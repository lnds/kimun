//! The dependency matrix as text: rows numbered and named, columns by
//! number only, so the matrix stays narrow, and below it the cells above
//! the diagonal with the uses behind each.

use unicode_width::UnicodeWidthStr;

use super::matrix::{Cell, Line, Mark, Matrix};

const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

/// The uses listed under each cell above the diagonal.
const USES_LISTED: usize = 5;

fn digits(n: usize) -> usize {
    n.max(1).ilog10() as usize + 1
}

/// The text of a cell, `!` after the count when it is part of a cycle.
fn cell_text(cell: &Cell) -> String {
    let cycle = cell.mark(false) == Mark::Cycle;
    format!("{}{}", cell.count, if cycle { "!" } else { "" })
}

fn paint(text: &str, mark: Mark, color: bool) -> String {
    let code = match mark {
        _ if !color => return text.to_string(),
        Mark::Cycle => RED,
        Mark::Layering => YELLOW,
        Mark::Weak => DIM,
        Mark::Plain => return text.to_string(),
    };
    format!("{code}{text}{RESET}")
}

/// The label of each line: number and name, indented by level.
fn labels(m: &Matrix) -> Vec<String> {
    let number_width = digits(m.rows.len());
    let mut row = 0;
    m.lines
        .iter()
        .map(|line| match *line {
            Line::Group { node, level } => {
                let name = &m.tree.nodes[node].name;
                format!(" {:number_width$} {}▾ {name}", "", "  ".repeat(level))
            }
            Line::Row { node, level } => {
                row += 1;
                let n = &m.tree.nodes[node];
                let marker = if n.is_dir() { "▸ " } else { "" };
                format!(
                    " {row:>number_width$} {}{marker}{}",
                    "  ".repeat(level),
                    n.name
                )
            }
        })
        .collect()
}

/// The widths of the label column and of each cell.
fn widths(m: &Matrix, labels: &[String]) -> (usize, usize) {
    let label_width = labels.iter().map(|l| l.width()).max().unwrap_or(0);
    let widest = m
        .cells
        .values()
        .map(|c| cell_text(c).len())
        .max()
        .unwrap_or(1);
    (label_width, widest.max(digits(m.rows.len())))
}

/// The columns the matrix takes on screen.
pub fn width(m: &Matrix) -> usize {
    let (label_width, cw) = widths(m, &labels(m));
    label_width + m.rows.len() * (cw + 1)
}

/// The matrix and the cells above its diagonal, with ANSI colours when
/// `color`.
pub fn render(m: &Matrix, color: bool) -> String {
    let labels = labels(m);
    let (label_width, cw) = widths(m, &labels);
    let pad = |label: &str| format!("{label}{}", " ".repeat(label_width - label.width()));

    let mut out = vec![
        "Dependency Structure Matrix".to_string(),
        "A cell counts the files of its column that use files of its row; ! marks a cycle."
            .to_string(),
        String::new(),
    ];
    let header: String = (1..=m.rows.len()).map(|c| format!(" {c:>cw$}")).collect();
    out.push(format!("{}{header}", pad("")));
    let mut row = 0;
    for (line, label) in m.lines.iter().zip(&labels) {
        if let Line::Group { .. } = line {
            out.push(label.clone());
            continue;
        }
        let cells: String = (0..m.rows.len())
            .map(|col| {
                let text = match m.cells.get(&(row, col)) {
                    _ if col == row => "·".to_string(),
                    Some(cell) => {
                        let text = format!("{:>cw$}", cell_text(cell));
                        return format!(" {}", paint(&text, cell.mark(row < col), color));
                    }
                    None => String::new(),
                };
                format!(" {text:>cw$}")
            })
            .collect();
        out.push(format!("{}{cells}", pad(label)).trim_end().to_string());
        row += 1;
    }
    out.push(String::new());
    out.extend(above_lines(m));
    out.join("\n") + "\n"
}

/// The name of a row as a path, so it reads without the tree.
fn row_path(m: &Matrix, row: usize) -> String {
    let node = m.node(row);
    let slash = if node.is_dir() { "/" } else { "" };
    format!("{}{slash}", node.path.display())
}

fn above_lines(m: &Matrix) -> Vec<String> {
    let above: Vec<_> = m.above().collect();
    if above.is_empty() {
        return vec!["Nothing above the diagonal.".to_string()];
    }
    let noun = if above.len() == 1 { "cell" } else { "cells" };
    let mut lines = vec![format!("Above the diagonal: {} {noun}", above.len())];
    for (row, col, cell) in above {
        let why = match cell.mark(true) {
            Mark::Cycle => "in a cycle",
            Mark::Weak => "type-only or deferred",
            _ => "against the layering",
        };
        lines.push(format!(
            "  {} → {}: {}, {why}",
            row_path(m, col),
            row_path(m, row),
            cell.count
        ));
        for &e in cell.edges.iter().take(USES_LISTED) {
            let edge = &m.edges[e];
            lines.push(format!(
                "    {} → {}",
                edge.from.display(),
                edge.to.display()
            ));
        }
        if cell.edges.len() > USES_LISTED {
            lines.push(format!("    … and {} more", cell.edges.len() - USES_LISTED));
        }
    }
    lines
}

#[cfg(test)]
#[path = "matrix_text_test.rs"]
mod tests;
