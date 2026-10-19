use super::*;

fn paths(files: &[&str]) -> Vec<PathBuf> {
    files.iter().map(PathBuf::from).collect()
}

fn names(tree: &Tree, node: usize) -> Vec<&str> {
    let children = &tree.nodes[node].children;
    children
        .iter()
        .map(|&c| tree.nodes[c].name.as_str())
        .collect()
}

#[test]
fn the_root_is_the_directory_every_file_is_under() {
    let tree = Tree::of(&paths(&["src/a.rs", "src/b/c.rs"]));
    assert_eq!(tree.nodes[ROOT].path, PathBuf::from("src"));
    assert_eq!(names(&tree, ROOT), vec!["a.rs", "b/"]);
}

#[test]
fn a_directory_holding_only_another_one_is_merged_with_it() {
    let tree = Tree::of(&paths(&["a.rs", "lib/core/x.rs", "lib/core/y.rs"]));
    assert_eq!(names(&tree, ROOT), vec!["a.rs", "lib/core/"]);
    let merged = tree.nodes[ROOT].children[1];
    assert_eq!(tree.nodes[merged].path, PathBuf::from("lib/core"));
    assert_eq!(names(&tree, merged), vec!["x.rs", "y.rs"]);
}
