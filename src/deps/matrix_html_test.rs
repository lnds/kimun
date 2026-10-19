use std::path::PathBuf;

use super::*;
use crate::deps::edges::Edge;
use crate::deps::graph::EdgeKind;
use crate::deps::matrix::Expand;

fn matrix() -> Matrix {
    let files: Vec<PathBuf> = ["main.rs", "core/a.rs", "core/b.rs"]
        .iter()
        .map(PathBuf::from)
        .collect();
    let edge = |from: &str, to: &str, kind| Edge {
        from: PathBuf::from(from),
        to: PathBuf::from(to),
        kind,
    };
    let edges = vec![
        edge("main.rs", "core/a.rs", EdgeKind::Load),
        edge("core/a.rs", "core/b.rs", EdgeKind::Load),
        edge("core/b.rs", "core/a.rs", EdgeKind::Type),
    ];
    Matrix::build(&files, &edges, &[], &Expand::Depth(2)).unwrap()
}

fn embedded(html: &str) -> serde_json::Value {
    let start = html.find(r#"id="data">"#).unwrap() + r#"id="data">"#.len();
    let end = start + html[start..].find("</script>").unwrap();
    serde_json::from_str(&html[start..end]).unwrap()
}

#[test]
fn the_page_carries_the_tree_the_files_and_the_edges() {
    let html = render(&matrix(), "shop").unwrap();
    let data = embedded(&html);
    assert_eq!(
        data["files"],
        serde_json::json!(["main.rs", "core/a.rs", "core/b.rs"])
    );
    assert_eq!(data["nodes"][0]["children"].as_array().unwrap().len(), 2);
    // main.rs uses core/a.rs; the type-only use is not strong.
    assert_eq!(data["edges"][0], serde_json::json!([0, 1, true, false]));
    assert_eq!(data["edges"][2], serde_json::json!([2, 1, false, false]));
}

#[test]
fn the_page_opens_with_the_directories_the_text_expands() {
    let m = matrix();
    let data = embedded(&render(&m, "shop").unwrap());
    let core = m.tree.nodes[0]
        .children
        .iter()
        .copied()
        .find(|&n| m.tree.nodes[n].is_dir());
    assert_eq!(data["expanded"], serde_json::json!([core.unwrap()]));
}

#[test]
fn the_page_is_titled_after_the_directory_and_needs_nothing_else() {
    let html = render(&matrix(), "<shop>").unwrap();
    assert!(html.contains("<title>Dependency matrix · &lt;shop&gt;</title>"));
    assert!(!html.contains("src=\"http") && !html.contains("href=\"http"));
}
