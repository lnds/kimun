use super::*;

fn path(scope: &[&str], segments: &[&str]) -> PathRef {
    PathRef {
        scope: scope.iter().map(|s| s.to_string()).collect(),
        segments: segments.iter().map(|s| s.to_string()).collect(),
    }
}

fn paths_of(source: &str) -> Vec<String> {
    let mut paths: Vec<String> = parse(source)
        .paths
        .iter()
        .map(|p| p.segments.join("::"))
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

#[test]
fn mod_declarations_with_their_path_and_whether_inline() {
    let file = parse(
        r#"
mod analyzer;
pub(crate) mod report;
#[cfg(test)]
#[path = "x_test.rs"]
mod tests;
mod inner {
    pub mod deep;
}
mod after;
"#,
    );
    let decls: Vec<(String, Option<&str>, bool)> = file
        .mods
        .iter()
        .map(|m| {
            let name = [m.scope.join("::"), m.name.clone()].join("/");
            (name, m.path.as_deref(), m.inline)
        })
        .collect();
    assert_eq!(
        decls,
        [
            ("/analyzer".to_string(), None, false),
            ("/report".to_string(), None, false),
            ("/tests".to_string(), Some("x_test.rs"), false),
            ("/inner".to_string(), None, true),
            ("inner/deep".to_string(), None, false),
            ("/after".to_string(), None, false),
        ]
    );
}

#[test]
fn use_trees_are_spread_into_paths() {
    let source = r#"
use crate::git::GitRepo;
use super::analyzer::{self, Level, report::{print, Kind as K}};
pub use self::inner::*;
use std::{
    collections::HashMap,
    path::Path,
};
use super::*;
"#;
    assert_eq!(
        paths_of(source),
        [
            "crate::git::GitRepo",
            "self::inner",
            "std::collections::HashMap",
            "std::path::Path",
            "super",
            "super::analyzer",
            "super::analyzer::Level",
            "super::analyzer::report::Kind",
            "super::analyzer::report::print",
        ]
    );
    let file = parse(source);
    let names: Vec<&str> = file.imports.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        [
            "GitRepo", "analyzer", "Level", "print", "K", "HashMap", "Path"
        ]
    );
}

#[test]
fn qualified_paths_in_code_are_paths() {
    let source = r#"
fn run(x: &crate::cli::Args) -> Result<(), Box<dyn std::error::Error>> {
    let since = crate::util::parse_since(&x.since)?;
    report::print(super::analyzer::compute(since), Kind::Table);
    let v = Vec::<u8>::new();
    x.iter().map(Item::name).count();
    Ok(())
}
"#;
    assert_eq!(
        paths_of(source),
        [
            "Item::name",
            "Kind::Table",
            "crate::cli::Args",
            "crate::util::parse_since",
            "report::print",
            "std::error::Error",
            "super::analyzer::compute",
        ]
    );
}

#[test]
fn comments_strings_and_macro_crates_name_nothing() {
    let source = r##"
// use crate::commented::Out;
/// See [`crate::docs::Link`].
const TEXT: &str = "use crate::in_string::X;";
const RAW: &str = r#"crate::raw::Y"#;
macro_rules! m { () => { $crate::expanded::z() } }
"##;
    assert_eq!(paths_of(source), Vec::<String>::new());
}

#[test]
fn a_path_carries_the_inline_modules_around_it() {
    let file = parse(
        r#"
use crate::a::A;
mod tests {
    use super::*;
    mod deeper { fn f() { super::super::helper::run() } }
    fn g() { crate::b::go() }
}
fn h() { crate::c::go() }
"#,
    );
    assert_eq!(
        file.paths,
        [
            path(&[], &["crate", "a", "A"]),
            path(&["tests"], &["super"]),
            path(&["tests", "deeper"], &["super", "super", "helper", "run"]),
            path(&["tests"], &["crate", "b", "go"]),
            path(&[], &["crate", "c", "go"]),
        ]
    );
}

#[test]
fn a_test_attribute_tells_the_file_holds_tests() {
    assert!(parse("#[test]\nfn a() {}").has_tests);
    assert!(parse("#[tokio::test]\nasync fn a() {}").has_tests);
    assert!(parse("#[rstest]\nfn a() {}").has_tests);
    assert!(parse("#[test_case(1)]\nfn a(x: u8) {}").has_tests);
    // A module of tests kept in another file holds none here.
    assert!(!parse("#[cfg(test)]\n#[path = \"x_test.rs\"]\nmod tests;").has_tests);
    assert!(!parse("#[derive(Debug)]\nstruct A;\n// #[test]").has_tests);
}

#[test]
fn unfinished_source_does_not_panic() {
    for source in [
        "use",
        "use a::{",
        "use a::{b,",
        "mod",
        "mod x",
        "#[",
        "#[path =",
        "a::",
        "use a as",
        "}}}",
    ] {
        let _ = parse(source);
    }
}

#[test]
fn an_impl_block_tells_the_type_it_is_for() {
    let file = parse(
        r#"
use super::GitRepo;
impl GitRepo { fn a(&self) {} }
impl<T: Clone> fmt::Display for crate::model::Wrapper<T> { }
unsafe impl Send for Handle {}
fn takes(x: impl Iterator<Item = u8>) -> impl Fn() -> Box<dyn Any> { todo!() }
"#,
    );
    let targets: Vec<String> = file.impls.iter().map(|p| p.segments.join("::")).collect();
    assert_eq!(targets, ["GitRepo", "crate::model::Wrapper", "Handle"]);
    // The paths an `impl` names are paths like any other.
    assert!(file.paths.contains(&path(&[], &["fmt", "Display"])));
    assert!(
        file.paths
            .contains(&path(&[], &["crate", "model", "Wrapper"]))
    );
}
