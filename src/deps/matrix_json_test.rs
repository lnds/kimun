use std::path::PathBuf;

use super::*;
use crate::deps::edges::Edge;
use crate::deps::graph::EdgeKind;
use crate::deps::matrix::Expand;

fn matrix(expand: Expand) -> Matrix {
    let files: Vec<PathBuf> = ["main.rs", "core/a.rs", "core/b.rs"]
        .iter()
        .map(PathBuf::from)
        .collect();
    let edge = |from: &str, to: &str| Edge {
        from: PathBuf::from(from),
        to: PathBuf::from(to),
        kind: EdgeKind::Load,
    };
    let edges = vec![
        edge("main.rs", "core/a.rs"),
        edge("core/a.rs", "core/b.rs"),
        edge("core/b.rs", "core/a.rs"),
    ];
    let cycle = vec![PathBuf::from("core/a.rs"), PathBuf::from("core/b.rs")];
    Matrix::build(&files, &edges, &[cycle], &expand).unwrap()
}

#[test]
fn rows_are_numbered_and_directories_end_in_a_slash() {
    let json = serde_json::to_value(of(Some(&matrix(Expand::Depth(1))), &[])).unwrap();
    assert_eq!(
        json["rows"][0],
        serde_json::json!({"number": 1, "path": "main.rs", "level": 0})
    );
    assert_eq!(json["rows"][1]["path"], "core/");
    assert_eq!(json["groups"], serde_json::json!([]));
    assert_eq!(json["above_diagonal"], serde_json::json!([]));
}

#[test]
fn cells_carry_their_mark_and_the_uses_behind_them() {
    let json = serde_json::to_value(of(Some(&matrix(Expand::Depth(2))), &[])).unwrap();
    assert_eq!(json["groups"], serde_json::json!(["core/"]));
    assert_eq!(json["above_diagonal"], serde_json::json!([[2, 3]]));
    let back = json["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["row"] == 2 && c["column"] == 3)
        .unwrap();
    assert_eq!(back["mark"], "cycle");
    assert_eq!(
        back["uses"],
        serde_json::json!([{"from": "core/b.rs", "to": "core/a.rs", "kind": "load", "cycle": true}])
    );
}

#[test]
fn no_files_give_an_empty_matrix() {
    let json = serde_json::to_value(of(None, &[])).unwrap();
    assert_eq!(json["rows"], serde_json::json!([]));
    assert_eq!(json["cells"], serde_json::json!([]));
}
