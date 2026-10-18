use super::*;
use crate::deps::graph::FileGraph;
use crate::deps::layout::Layout;

fn source(path: &str, text: &str) -> Source {
    let language = match Path::new(path).extension().and_then(|e| e.to_str()) {
        Some("c" | "h") => "C",
        _ => "C++",
    };
    Source {
        path: PathBuf::from(path),
        language: language.to_string(),
        text: text.to_string(),
        templates: Vec::new(),
    }
}

/// The paths `from` uses in the graph of `sources`, sorted.
fn used(sources: &[Source], from: &str) -> Vec<String> {
    let graph = FileGraph::build(sources, &Layout::default(), &|_, _| false);
    let mut used: Vec<String> = graph.edges()[Path::new(from)]
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    used.sort();
    used
}

fn quoted(path: &str) -> Include {
    Include {
        path: path.to_string(),
        quoted: true,
    }
}

#[test]
fn includes_are_read_outside_comments() {
    let src = "#include \"a.h\"\n  #  include <sys/b.h>\n// #include \"c.h\"\n/* #include \"d.h\"\n#include \"e.h\" */ #include \"f.h\"\n#include_next \"g.h\"\n#define X \"h.h\"\n";
    assert_eq!(
        includes(src),
        [
            quoted("a.h"),
            Include {
                path: "sys/b.h".to_string(),
                quoted: false
            },
            quoted("f.h"),
            quoted("g.h"),
        ]
    );
}

#[test]
fn a_quoted_include_is_found_beside_the_includer_first() {
    let sources = [
        source("src/net/conn.c", "#include \"util.h\"\n"),
        source("src/net/util.h", ""),
        source("src/util.h", ""),
    ];
    assert_eq!(used(&sources, "src/net/conn.c"), ["src/net/util.h"]);
}

#[test]
fn includes_are_found_above_and_in_include_directories() {
    let sources = [
        source(
            "lib/core/src/map.cpp",
            "#include <core/map.hpp>\n#include \"config.h\"\n#include <vector>\n#include <stdio.h>\n",
        ),
        source("lib/core/include/core/map.hpp", ""),
        source("lib/config.h", ""),
    ];
    assert_eq!(
        used(&sources, "lib/core/src/map.cpp"),
        ["lib/config.h", "lib/core/include/core/map.hpp"]
    );
}

#[test]
fn an_include_anywhere_resolves_to_the_nearest_file_with_its_path() {
    let sources = [
        source(
            "apps/a/main.cc",
            "#include \"proto/msg.h\"\n#include \"dup.h\"\n",
        ),
        source("apps/a/gen/proto/msg.h", ""),
        source("apps/b/gen/proto/msg.h", ""),
        source("x/dup.h", ""),
        source("y/dup.h", ""),
    ];
    // The two `dup.h` are as far: neither is taken
    assert_eq!(used(&sources, "apps/a/main.cc"), ["apps/a/gen/proto/msg.h"]);
}

#[test]
fn a_bare_system_header_is_never_looked_for_anywhere() {
    let sources = [
        source("src/main.c", "#include <string.h>\n"),
        source("compat/string.h", ""),
    ];
    assert!(used(&sources, "src/main.c").is_empty());
}

/// The paths `from` uses, implied, in the graph of `sources`, sorted.
fn implied(sources: &[Source], from: &str) -> Vec<String> {
    let graph = FileGraph::build(sources, &Layout::default(), &|_, _| false);
    let at = graph
        .files
        .iter()
        .position(|f| f == Path::new(from))
        .unwrap();
    let mut used: Vec<String> = graph.implied[at]
        .iter()
        .map(|u| graph.files[u.to].to_string_lossy().into_owned())
        .collect();
    used.sort();
    used
}

#[test]
fn including_a_header_implies_using_the_sources_that_define_it() {
    let sources = [
        source(
            "app/main.c",
            "#include \"list.h\"\n#include <shape/circle.hpp>\n",
        ),
        source("app/list.h", ""),
        source("app/list.c", "#include \"list.h\"\n"),
        source("include/shape/circle.hpp", ""),
        source("src/shape/circle.cpp", "#include <shape/circle.hpp>\n"),
    ];
    assert_eq!(
        used(&sources, "app/main.c"),
        ["app/list.h", "include/shape/circle.hpp"]
    );
    assert_eq!(
        implied(&sources, "app/main.c"),
        ["app/list.c", "src/shape/circle.cpp"]
    );
    // A source uses its own header, and is not its own dependency
    assert_eq!(used(&sources, "app/list.c"), ["app/list.h"]);
    assert!(implied(&sources, "app/list.c").is_empty());
}

#[test]
fn sources_and_headers_that_include_each_other_make_no_cycle() {
    // a.c includes b.h, b.c includes a.h: only implied uses close the loop
    let sources = [
        source("a.h", ""),
        source("a.c", "#include \"a.h\"\n#include \"b.h\"\n"),
        source("b.h", ""),
        source("b.c", "#include \"b.h\"\n#include \"a.h\"\n"),
    ];
    assert_eq!(used(&sources, "a.c"), ["a.h", "b.h"]);
    assert_eq!(implied(&sources, "a.c"), ["b.c"]);
}

#[test]
fn a_header_of_one_language_is_reached_from_the_other() {
    let sources = [
        source("wrap.cpp", "extern \"C\" {\n#include \"lib.h\"\n}\n"),
        source("lib.h", ""),
    ];
    assert_eq!(used(&sources, "wrap.cpp"), ["lib.h"]);
}
