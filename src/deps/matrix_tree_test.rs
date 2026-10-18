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

#[test]
fn users_come_before_what_they_use() {
    let mut tree = Tree::of(&paths(&["a.rs", "b.rs", "c.rs"]));
    // c uses b, b uses a.
    tree.order(&[(2, 1), (1, 0)]);
    assert_eq!(names(&tree, ROOT), vec!["c.rs", "b.rs", "a.rs"]);
}

#[test]
fn uses_between_directories_order_the_directories() {
    let mut tree = Tree::of(&paths(&["core/x.rs", "web/y.rs", "web/z.rs"]));
    // web/z uses core/x, web/z uses web/y.
    tree.order(&[(2, 0), (2, 1)]);
    assert_eq!(names(&tree, ROOT), vec!["web/", "core/"]);
    let web = tree.nodes[ROOT].children[0];
    assert_eq!(names(&tree, web), vec!["z.rs", "y.rs"]);
}

#[test]
fn a_cycle_stays_together_and_the_more_using_member_goes_first() {
    let mut tree = Tree::of(&paths(&["a.rs", "b.rs", "c.rs", "d.rs"]));
    // a, b and c form a cycle in which c uses two and is used by one; d
    // uses c.
    tree.order(&[(0, 1), (1, 2), (2, 0), (2, 1), (3, 2)]);
    assert_eq!(names(&tree, ROOT), vec!["d.rs", "c.rs", "a.rs", "b.rs"]);
}

#[test]
fn files_without_uses_keep_the_order_of_their_names() {
    let mut tree = Tree::of(&paths(&["b.rs", "a.rs", "c.rs"]));
    tree.order(&[]);
    assert_eq!(names(&tree, ROOT), vec!["a.rs", "b.rs", "c.rs"]);
}
