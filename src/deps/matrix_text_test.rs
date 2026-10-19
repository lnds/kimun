use std::path::PathBuf;

use super::*;
use crate::deps::edges::Edge;
use crate::deps::graph::EdgeKind;
use crate::deps::matrix::Expand;

fn edge(from: &str, to: &str) -> Edge {
    Edge {
        from: PathBuf::from(from),
        to: PathBuf::from(to),
        kind: EdgeKind::Load,
    }
}

fn matrix(expand: Expand) -> Matrix {
    let files: Vec<PathBuf> = ["main.rs", "core/a.rs", "core/b.rs", "web/c.rs"]
        .iter()
        .map(PathBuf::from)
        .collect();
    let edges = vec![
        edge("main.rs", "web/c.rs"),
        edge("web/c.rs", "core/a.rs"),
        edge("core/a.rs", "core/b.rs"),
        edge("core/b.rs", "core/a.rs"),
    ];
    let cycle = vec![PathBuf::from("core/a.rs"), PathBuf::from("core/b.rs")];
    Matrix::build(&files, &edges, &[cycle], &expand).unwrap()
}

#[test]
fn collapsed_matrix_reads_as_rows_and_numbered_columns() {
    let text = render(&matrix(Expand::Depth(1)), false);
    let expected = [
        "Dependency Structure Matrix",
        "A cell counts the files of its column that use files of its row; ! marks a cycle.",
        "",
        "           1 2 3",
        " 1 main.rs ·",
        " 2 ▸ web/  1 ·",
        " 3 ▸ core/   1 ·",
        "",
        "Nothing above the diagonal.",
        "",
    ]
    .join("\n");
    assert_eq!(text, expected);
}

#[test]
fn expanded_matrix_lists_what_is_above_the_diagonal() {
    let text = render(&matrix(Expand::Depth(2)), false);
    let expected = [
        "Dependency Structure Matrix",
        "A cell counts the files of its column that use files of its row; ! marks a cycle.",
        "",
        "            1  2  3  4",
        " 1 main.rs  ·",
        "   ▾ web/",
        " 2   c.rs   1  ·",
        "   ▾ core/",
        " 3   a.rs      1  · 1!",
        " 4   b.rs        1!  ·",
        "",
        "Above the diagonal: 1 cell",
        "  core/b.rs → core/a.rs: 1, in a cycle",
        "    core/b.rs → core/a.rs",
        "",
    ]
    .join("\n");
    assert_eq!(text, expected);
}

#[test]
fn colour_marks_cycles_in_red() {
    let text = render(&matrix(Expand::Depth(2)), true);
    assert!(text.contains("\x1b[31m1!\x1b[0m"));
    assert!(!render(&matrix(Expand::Depth(2)), false).contains('\x1b'));
}

#[test]
fn width_counts_the_label_and_every_column() {
    let m = matrix(Expand::Depth(1));
    let widest = render(&m, false)
        .lines()
        .skip(3)
        .take(4)
        .map(|l| l.width())
        .max()
        .unwrap();
    assert!(width(&m) >= widest);
    assert_eq!(width(&m), 10 + 3 * 2);
}
