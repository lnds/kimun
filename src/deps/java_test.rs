use super::*;
use crate::deps::graph::FileGraph;
use crate::deps::layout::Layout;
use std::path::{Path, PathBuf};

fn java(path: &str, text: &str) -> Source {
    Source {
        path: PathBuf::from(path),
        language: "Java".to_string(),
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

#[test]
fn a_file_is_read_for_its_package_imports_and_classes() {
    let src = "package com.acme.shop;\n\nimport java.util.List;\nimport static com.acme.util.Strings.pad;\nimport com.acme.model.*;\n\n@Entity(\"x\")\npublic final class Cart<T> extends Base implements Serializable {\n  enum Kind { A }\n  List<Item> items;\n  Object k = Cart.class;\n}\ninterface Priced {}\n";
    let file = read(src);
    assert_eq!(file.package, "com.acme.shop");
    assert_eq!(file.declares, ["Cart", "Priced"]);
    assert_eq!(
        file.imports,
        ["java.util.List", "com.acme.util.Strings.pad"]
    );
    assert_eq!(file.on_demand, ["com.acme.model"]);
    let names: Vec<String> = file.names.iter().map(|n| n.join(".")).collect();
    for name in [
        "Entity",
        "Base",
        "Serializable",
        "List",
        "Item",
        "Cart.class",
    ] {
        assert!(names.contains(&name.to_string()), "{name} in {names:?}");
    }
}

#[test]
fn imports_and_classes_of_the_same_package_are_used() {
    let sources = [
        java(
            "src/main/java/com/acme/shop/Cart.java",
            "package com.acme.shop;\nimport com.acme.model.Item;\nimport java.util.List;\nclass Cart { List<Item> items; Pricing p; }\n",
        ),
        java(
            "src/main/java/com/acme/shop/Pricing.java",
            "package com.acme.shop;\nclass Pricing {}\n",
        ),
        java(
            "src/main/java/com/acme/shop/Unused.java",
            "package com.acme.shop;\nclass Unused {}\n",
        ),
        java(
            "src/main/java/com/acme/model/Item.java",
            "package com.acme.model;\npublic class Item {}\n",
        ),
    ];
    assert_eq!(
        used(&sources, "src/main/java/com/acme/shop/Cart.java"),
        [
            "src/main/java/com/acme/model/Item.java",
            "src/main/java/com/acme/shop/Pricing.java"
        ]
    );
}

#[test]
fn on_demand_imports_use_only_the_classes_named() {
    let sources = [
        java(
            "app/Main.java",
            "package app;\nimport lib.*;\nclass Main { Parser p; }\n",
        ),
        java("lib/Parser.java", "package lib;\npublic class Parser {}\n"),
        java(
            "lib/Printer.java",
            "package lib;\npublic class Printer {}\n",
        ),
    ];
    assert_eq!(used(&sources, "app/Main.java"), ["lib/Parser.java"]);
}

#[test]
fn static_nested_and_qualified_uses_lead_to_the_outer_class() {
    let sources = [
        java(
            "a/Main.java",
            "package a;\nimport static b.Util.max;\nimport b.Tree.Node;\nclass Main { Object o = c.Clock.now(); }\n",
        ),
        java("b/Util.java", "package b;\npublic class Util {}\n"),
        java(
            "b/Tree.java",
            "package b;\npublic class Tree { public static class Node {} }\n",
        ),
        java("c/Clock.java", "package c;\npublic class Clock {}\n"),
    ];
    assert_eq!(
        used(&sources, "a/Main.java"),
        ["b/Tree.java", "b/Util.java", "c/Clock.java"]
    );
}

#[test]
fn classes_of_another_package_need_an_import() {
    let sources = [
        java("a/Main.java", "package a;\nclass Main { Helper h; }\n"),
        java("b/Helper.java", "package b;\npublic class Helper {}\n"),
    ];
    assert!(used(&sources, "a/Main.java").is_empty());
}
