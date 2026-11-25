//! The uses a matrix hides: those between files of one collapsed directory,
//! which fall on the diagonal. When no cell shows a use, the text names the
//! directories that hold them and the `--focus` that opens each.

use unicode_width::UnicodeWidthStr;

use super::matrix::Matrix;
use super::matrix_text::row_path;
use crate::file_tree::Tree;

/// The uses with both ends inside each row that is a directory, given the
/// rows of the ends of every use: row and uses, most first, then in the
/// order of the rows.
pub fn of(tree: &Tree, rows: &[usize], between: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut uses = vec![0; rows.len()];
    for &(from, to) in between {
        if from == to && tree.nodes[rows[from]].is_dir() {
            uses[from] += 1;
        }
    }
    let mut inside: Vec<(usize, usize)> = uses
        .into_iter()
        .enumerate()
        .filter(|&(_, n)| n > 0)
        .collect();
    inside.sort_by_key(|&(row, n)| (std::cmp::Reverse(n), row));
    inside
}

fn noun(n: usize) -> &'static str {
    if n == 1 { "use" } else { "uses" }
}

/// What the text says below the matrix: where the uses are when no cell
/// shows one and some directory holds them, else what `otherwise` says.
pub fn below(m: &Matrix, otherwise: fn(&Matrix) -> Vec<String>) -> Vec<String> {
    if m.cells.is_empty() && !m.inside.is_empty() {
        hint(m)
    } else {
        otherwise(m)
    }
}

fn hint(m: &Matrix) -> Vec<String> {
    let total: usize = m.inside.iter().map(|&(_, n)| n).sum();
    let head = format!(
        "No dependencies between these rows; {total} {} inside them.",
        noun(total)
    );
    let paths: Vec<String> = m.inside.iter().map(|&(row, _)| row_path(m, row)).collect();
    let path_width = paths.iter().map(|p| p.width()).max().unwrap_or(0);
    let count_width = m.inside.iter().map(|&(_, n)| n.to_string().len()).max();
    let count_width = count_width.unwrap_or(1);
    let rows = paths.iter().zip(&m.inside).map(|(path, &(row, n))| {
        let pad = " ".repeat(path_width - path.width());
        let focus = m.node(row).path.display();
        format!(
            "  {path}{pad}  {n:>count_width$} {:<4}  --focus {focus}",
            noun(n)
        )
    });
    std::iter::once(head).chain(rows).collect()
}
