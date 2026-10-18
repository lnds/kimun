use super::*;

fn paths(files: &[&str]) -> Vec<PathBuf> {
    files.iter().map(PathBuf::from).collect()
}

fn edge(from: &str, to: &str, kind: EdgeKind) -> Edge {
    Edge {
        from: PathBuf::from(from),
        to: PathBuf::from(to),
        kind,
    }
}

fn row_names(m: &Matrix) -> Vec<String> {
    (0..m.rows.len()).map(|r| m.node(r).name.clone()).collect()
}

fn files() -> Vec<PathBuf> {
    paths(&["main.rs", "core/a.rs", "core/b.rs", "web/c.rs"])
}

fn edges() -> Vec<Edge> {
    vec![
        edge("main.rs", "web/c.rs", EdgeKind::Load),
        edge("main.rs", "core/a.rs", EdgeKind::Load),
        edge("web/c.rs", "core/a.rs", EdgeKind::Load),
        edge("web/c.rs", "core/b.rs", EdgeKind::Load),
        edge("core/a.rs", "core/b.rs", EdgeKind::Load),
    ]
}

#[test]
fn collapsed_directories_count_every_use_of_their_files() {
    let m = Matrix::build(&files(), &edges(), &[], &Expand::Depth(1)).unwrap();
    assert_eq!(row_names(&m), vec!["main.rs", "web/", "core/"]);
    // web/ uses core/ twice; main.rs uses core/ once and web/ once.
    assert_eq!(m.cells[&(2, 1)].count, 2);
    assert_eq!(m.cells[&(2, 0)].count, 1);
    assert_eq!(m.cells[&(1, 0)].count, 1);
    assert_eq!(m.above().count(), 0);
}

#[test]
fn uses_inside_a_collapsed_directory_are_not_counted() {
    let m = Matrix::build(&files(), &edges(), &[], &Expand::Depth(1)).unwrap();
    assert!(!m.cells.keys().any(|(row, col)| row == col));
}

#[test]
fn depth_expands_directories_into_groups_and_rows() {
    let m = Matrix::build(&files(), &edges(), &[], &Expand::Depth(2)).unwrap();
    assert_eq!(row_names(&m), vec!["main.rs", "c.rs", "a.rs", "b.rs"]);
    let groups = m
        .lines
        .iter()
        .filter(|l| matches!(l, Line::Group { .. }))
        .count();
    assert_eq!(groups, 2);
}

#[test]
fn focus_expands_one_directory_alone() {
    let focus = Expand::Focus(PathBuf::from("core"));
    let m = Matrix::build(&files(), &edges(), &[], &focus).unwrap();
    assert_eq!(row_names(&m), vec!["main.rs", "web/", "a.rs", "b.rs"]);
}

#[test]
fn focus_on_no_directory_of_the_graph_fails() {
    let focus = Expand::Focus(PathBuf::from("nowhere"));
    let error = Matrix::build(&files(), &edges(), &[], &focus)
        .err()
        .unwrap();
    assert_eq!(error, "--focus nowhere: no directory of the graph");
}

#[test]
fn cells_of_a_cycle_are_marked_and_the_back_use_is_above_the_diagonal() {
    let files = paths(&["a.rs", "b.rs"]);
    let edges = vec![
        edge("a.rs", "b.rs", EdgeKind::Load),
        edge("b.rs", "a.rs", EdgeKind::Load),
    ];
    let m = Matrix::build(&files, &edges, &[files.clone()], &Expand::Depth(1)).unwrap();
    let above: Vec<_> = m.above().collect();
    assert_eq!(above.len(), 1);
    assert_eq!(above[0].2.mark(true), Mark::Cycle);
    assert_eq!(m.cells[&(1, 0)].mark(false), Mark::Cycle);
}

#[test]
fn a_cycle_between_directories_alone_goes_against_the_layering() {
    let files = paths(&["x/a.rs", "x/b.rs", "y/c.rs"]);
    // x/a uses y/c and y/c uses x/b: no cycle between files.
    let edges = vec![
        edge("x/a.rs", "y/c.rs", EdgeKind::Load),
        edge("y/c.rs", "x/b.rs", EdgeKind::Load),
    ];
    let m = Matrix::build(&files, &edges, &[], &Expand::Depth(1)).unwrap();
    let above: Vec<_> = m.above().map(|(_, _, cell)| cell.mark(true)).collect();
    assert_eq!(above, vec![Mark::Layering]);
}

#[test]
fn type_only_uses_are_weak_and_do_not_order() {
    let files = paths(&["a.ts", "b.ts"]);
    let edges = vec![
        edge("a.ts", "b.ts", EdgeKind::Load),
        edge("b.ts", "a.ts", EdgeKind::Type),
    ];
    let m = Matrix::build(&files, &edges, &[], &Expand::Depth(1)).unwrap();
    assert_eq!(row_names(&m), vec!["a.ts", "b.ts"]);
    let above: Vec<_> = m.above().map(|(_, _, cell)| cell.mark(true)).collect();
    assert_eq!(above, vec![Mark::Weak]);
}

#[test]
fn edges_to_files_left_out_are_dropped() {
    let files = paths(&["a.rs", "b.rs"]);
    let edges = vec![edge("a.rs", "c.rs", EdgeKind::Load)];
    let m = Matrix::build(&files, &edges, &[], &Expand::Depth(1)).unwrap();
    assert!(m.cells.is_empty());
    assert!(m.edges.is_empty());
}
