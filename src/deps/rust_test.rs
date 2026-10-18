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

fn mod_paths(source: &str) -> Vec<(String, Option<String>)> {
    parse(source)
        .mods
        .into_iter()
        .map(|m| (m.name, m.path))
        .collect()
}

#[test]
fn each_path_attribute_names_the_string_it_holds() {
    let mods = mod_paths("#[path = \"a.rs\"]\nmod a;\n#[path = \"b.rs\"]\nmod b;\n");
    assert_eq!(
        mods,
        [
            ("a".to_string(), Some("a.rs".to_string())),
            ("b".to_string(), Some("b.rs".to_string())),
        ]
    );
}

#[test]
fn a_path_attribute_ends_with_the_statement_it_is_on() {
    for statement in [
        "const A: u8 = 1;",
        "use a::b as c;",
        "use a::*;",
        "use a::{b, c};",
    ] {
        let source = format!("#[path = \"x.rs\"]\n{statement}\nmod m;\n");
        assert_eq!(mod_paths(&source), [("m".to_string(), None)], "{statement}");
    }
}

#[test]
fn what_an_attribute_holds_names_no_path() {
    assert!(paths_of("#![warn(clippy::all)]\nfn f() {}\n").is_empty());
    assert!(paths_of("#[derive(serde::Serialize)]\nstruct A;\n").is_empty());
    assert_eq!(
        paths_of("#[derive(Debug)]\nstruct A(crate::b::B);\n"),
        ["crate::b::B"]
    );
}

#[test]
fn a_mod_that_declares_nothing_is_skipped_alone() {
    // Not Rust, but what is read must not swallow what follows.
    let file = parse("mod outer { mod }\nuse crate::x::Y;\n");
    assert_eq!(file.paths, [path(&[], &["crate", "x", "Y"])]);
    assert_eq!(paths_of("mod a::b;"), ["a::b"]);
}

#[test]
fn an_arrow_inside_generics_closes_none() {
    let file = parse("impl<F: Fn() -> u32> Call for Holder<F> {}\n");
    assert_eq!(file.impls, [path(&[], &["Holder"])]);
}

#[test]
fn an_impl_at_the_very_start_and_a_stray_angle_are_read() {
    assert_eq!(parse("impl Foo {}").impls, [path(&[], &["Foo"])]);
    assert_eq!(parse("impl A > B {}").impls, [path(&[], &["A"])]);
}

#[test]
fn a_macro_crate_at_the_very_start_names_nothing() {
    assert!(parse("$crate::a::b").paths.is_empty());
}

#[test]
fn a_use_inside_an_inline_module_keeps_what_follows_inside_it() {
    let file = parse("mod m {\n    use a::{b, c};\n    use x::y;\n}\n");
    assert!(file.paths.contains(&path(&["m"], &["x", "y"])));
}

#[test]
fn what_a_use_names_and_what_it_does_not() {
    // A name is bound only to a path of more than one segment.
    assert!(parse("use serde;").imports.is_empty());
    // `self` stands for the module before it, when there is one.
    let group = parse("use a::{self, b};");
    assert_eq!(group.paths, [path(&[], &["a"]), path(&[], &["a", "b"])]);
    assert_eq!(group.imports, [("b".to_string(), path(&[], &["a", "b"]))]);
    assert_eq!(parse("use self;").paths, [path(&[], &["self"])]);
    // An alias that is no name binds nothing.
    assert!(parse("use a::b as ;").imports.is_empty());
}

#[test]
fn extern_crate_is_a_path_to_the_crate_and_binds_its_alias() {
    let file = parse("pub extern crate grep_printer as printer;\nextern crate serde;");
    assert_eq!(
        file.imports,
        [("printer".to_string(), path(&[], &["grep_printer"]))]
    );
    assert_eq!(
        file.paths,
        [path(&[], &["grep_printer"]), path(&[], &["serde"])]
    );
    assert!(parse("extern crate self as me;").paths.is_empty());
    assert!(parse("extern crate foo as _;").imports.is_empty());
}
