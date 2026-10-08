use super::*;
use crate::deps::rust::parse;

/// The files each file uses, by path, for sources given as `(path, text)`.
fn graph(sources: &[(&str, &str)], packages: &[(&str, &str)]) -> Vec<(String, Vec<String>)> {
    let files: Vec<PathBuf> = sources.iter().map(|(p, _)| PathBuf::from(p)).collect();
    let parsed: Vec<Option<RustFile>> = sources.iter().map(|(_, t)| Some(parse(t))).collect();
    let packages: Vec<(PathBuf, String)> = packages
        .iter()
        .map(|(dir, name)| (PathBuf::from(dir), name.to_string()))
        .collect();
    uses(&files, &parsed, &packages)
        .iter()
        .enumerate()
        .map(|(file, used)| {
            let used = used
                .iter()
                .filter(|&&to| to != file)
                .map(|&to| sources[to].0.to_string())
                .collect();
            (sources[file].0.to_string(), used)
        })
        .collect()
}

fn used_by<'a>(graph: &'a [(String, Vec<String>)], file: &str) -> Vec<&'a str> {
    let (_, used) = graph
        .iter()
        .find(|(f, _)| f == file)
        .expect("file in graph");
    used.iter().map(String::as_str).collect()
}

#[test]
fn a_path_leads_to_the_file_of_the_deepest_module_it_names() {
    let g = graph(
        &[
            (
                "src/main.rs",
                "mod cli;\nmod git;\nmod util;\nfn main() { cli::run() }",
            ),
            (
                "src/cli.rs",
                "use crate::git::changeset::Stats;\nuse crate::util::parse;",
            ),
            (
                "src/git/mod.rs",
                "mod changeset;\npub use changeset::Stats;",
            ),
            (
                "src/git/changeset.rs",
                "pub struct Stats;\nfn f() { super::open(); crate::util::parse(); }",
            ),
            ("src/util.rs", "pub fn parse() {}"),
        ],
        &[],
    );
    // Declaring a module is not using it: only `cli` is called.
    assert_eq!(used_by(&g, "src/main.rs"), ["src/cli.rs"]);
    assert_eq!(
        used_by(&g, "src/cli.rs"),
        ["src/git/changeset.rs", "src/util.rs"]
    );
    assert_eq!(used_by(&g, "src/git/mod.rs"), ["src/git/changeset.rs"]);
    assert_eq!(
        used_by(&g, "src/git/changeset.rs"),
        ["src/git/mod.rs", "src/util.rs"]
    );
    assert!(used_by(&g, "src/util.rs").is_empty());
}

#[test]
fn a_file_that_is_not_mod_rs_keeps_its_modules_in_its_own_directory() {
    let g = graph(
        &[
            ("src/lib.rs", "mod a;\nmod b;"),
            ("src/a.rs", "mod b;\nfn f() { b::inner() }"),
            ("src/a/b.rs", "pub fn inner() { crate::b::outer() }"),
            ("src/b.rs", "pub fn outer() {}"),
        ],
        &[],
    );
    assert_eq!(used_by(&g, "src/a.rs"), ["src/a/b.rs"]);
    assert_eq!(used_by(&g, "src/a/b.rs"), ["src/b.rs"]);
}

#[test]
fn a_test_module_kept_in_another_file_uses_the_file_that_declares_it() {
    let g = graph(
        &[
            ("src/main.rs", "mod deps;"),
            (
                "src/deps/mod.rs",
                "mod graph;\n#[cfg(test)]\n#[path = \"mod_test.rs\"]\nmod tests;",
            ),
            (
                "src/deps/graph.rs",
                "pub fn build() {}\n#[cfg(test)]\n#[path = \"graph_test.rs\"]\nmod tests;",
            ),
            (
                "src/deps/graph_test.rs",
                "use super::*;\n#[test]\nfn t() { build() }",
            ),
            (
                "src/deps/mod_test.rs",
                "use super::*;\nuse super::graph::build;",
            ),
        ],
        &[],
    );
    // `super` is the file with the declaration, not the `mod.rs` of the directory.
    assert_eq!(used_by(&g, "src/deps/graph_test.rs"), ["src/deps/graph.rs"]);
    assert_eq!(
        used_by(&g, "src/deps/mod_test.rs"),
        ["src/deps/mod.rs", "src/deps/graph.rs"]
    );
}

#[test]
fn inline_tests_name_their_own_file() {
    let g = graph(
        &[
            ("src/lib.rs", "pub mod a;\npub mod b;"),
            (
                "src/a.rs",
                "pub fn f() {}\n#[cfg(test)]\nmod tests {\n    use super::*;\n    use crate::b::helper;\n}",
            ),
            ("src/b.rs", "pub fn helper() {}"),
        ],
        &[],
    );
    // `super` inside the inline module is `a` itself, not the crate root.
    assert_eq!(used_by(&g, "src/a.rs"), ["src/b.rs"]);
}

#[test]
fn a_name_brought_by_use_leads_where_the_use_does() {
    let g = graph(
        &[
            ("src/lib.rs", "mod deps;\nmod run;"),
            ("src/deps/mod.rs", "pub mod graph;"),
            ("src/deps/graph.rs", "pub struct FileGraph;"),
            (
                "src/run.rs",
                "use crate::deps as d;\nfn f() { d::graph::FileGraph::build() }",
            ),
        ],
        &[],
    );
    assert_eq!(
        used_by(&g, "src/run.rs"),
        ["src/deps/mod.rs", "src/deps/graph.rs"]
    );
}

#[test]
fn other_targets_of_a_package_reach_its_library_by_name() {
    let sources = [
        ("app/src/lib.rs", "pub mod orders;"),
        ("app/src/orders.rs", "pub fn total() {}"),
        ("app/src/main.rs", "fn main() { my_app::orders::total() }"),
        (
            "app/tests/orders.rs",
            "mod common;\nuse my_app::orders::total;\n#[test]\nfn t() { common::setup(); total() }",
        ),
        ("app/tests/common/mod.rs", "pub fn setup() {}"),
        ("app/examples/demo.rs", "use my_app::orders;"),
        (
            "tool/src/lib.rs",
            "pub fn go() { my_app::orders::total(); serde::de(); }",
        ),
    ];
    let g = graph(&sources, &[("app", "my-app"), ("tool", "tool")]);
    assert_eq!(used_by(&g, "app/src/main.rs"), ["app/src/orders.rs"]);
    assert_eq!(
        used_by(&g, "app/tests/orders.rs"),
        ["app/src/orders.rs", "app/tests/common/mod.rs"]
    );
    assert_eq!(used_by(&g, "app/examples/demo.rs"), ["app/src/orders.rs"]);
    // Another crate of the workspace; a crate from outside names nothing.
    assert_eq!(used_by(&g, "tool/src/lib.rs"), ["app/src/orders.rs"]);

    // With no manifest read, a package answers to the name of its directory.
    let g = graph(&sources[..3], &[]);
    assert!(used_by(&g, "app/src/main.rs").is_empty());
    let renamed: Vec<(&str, &str)> = sources[..3]
        .iter()
        .map(|(p, t)| (p.strip_prefix("app/").unwrap(), *t))
        .collect();
    assert!(used_by(&graph(&renamed, &[]), "src/main.rs").is_empty());
    let g = graph(
        &[
            ("my-app/src/lib.rs", "pub mod orders;"),
            ("my-app/src/orders.rs", ""),
            ("my-app/src/main.rs", "use my_app::orders;"),
        ],
        &[],
    );
    assert_eq!(used_by(&g, "my-app/src/main.rs"), ["my-app/src/orders.rs"]);
}

#[test]
fn what_no_crate_reaches_and_what_leaves_the_crate_use_nothing() {
    let g = graph(
        &[
            (
                "src/lib.rs",
                "mod a;\nfn f() { super::above(); std::fs::read(); a::x() }",
            ),
            (
                "src/a.rs",
                "use an_import::that::names::itself as an_import;\nfn g() { an_import::go() }",
            ),
            ("src/orphan.rs", "use crate::a::x;"),
        ],
        &[],
    );
    assert_eq!(used_by(&g, "src/lib.rs"), ["src/a.rs"]);
    assert!(used_by(&g, "src/a.rs").is_empty());
    assert!(used_by(&g, "src/orphan.rs").is_empty());
}

#[test]
fn a_type_uses_the_files_that_hold_its_impl_blocks() {
    let g = graph(
        &[
            (
                "src/lib.rs",
                "pub mod git;\nfn f() { git::GitRepo::open().files_at() }",
            ),
            (
                "src/git/mod.rs",
                "mod tree;\npub struct GitRepo;\nimpl GitRepo { pub fn open() -> Self { GitRepo } }",
            ),
            (
                "src/git/tree.rs",
                "use super::GitRepo;\nimpl GitRepo { pub fn files_at(&self) {} }",
            ),
        ],
        &[],
    );
    // Nothing names `tree`, yet whoever holds a `GitRepo` can call into it.
    assert_eq!(used_by(&g, "src/git/mod.rs"), ["src/git/tree.rs"]);
    assert_eq!(used_by(&g, "src/git/tree.rs"), ["src/git/mod.rs"]);
    assert_eq!(used_by(&g, "src/lib.rs"), ["src/git/mod.rs"]);
}

#[test]
fn a_tree_that_starts_inside_src_still_has_its_root() {
    let g = graph(
        &[
            ("main.rs", "mod util;\nmod loc;\nfn main() { loc::run() }"),
            ("util.rs", "pub fn parse() {}"),
            ("loc/mod.rs", "pub fn run() { crate::util::parse() }"),
        ],
        &[],
    );
    assert_eq!(used_by(&g, "main.rs"), ["loc/mod.rs"]);
    assert_eq!(used_by(&g, "loc/mod.rs"), ["util.rs"]);
}
