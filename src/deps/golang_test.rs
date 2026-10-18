use super::*;
use crate::deps::graph::FileGraph;

fn go(path: &str, text: &str) -> Source {
    Source {
        path: PathBuf::from(path),
        language: "Go".to_string(),
        text: text.to_string(),
        templates: Vec::new(),
    }
}

fn layout(modules: &[(&str, &str)]) -> Layout {
    Layout {
        go_modules: modules
            .iter()
            .map(|(dir, module)| (PathBuf::from(dir), module.to_string()))
            .collect(),
        ..Layout::default()
    }
}

/// The paths `from` uses in the graph of `sources`, sorted.
fn used(sources: &[Source], layout: &Layout, from: &str) -> Vec<String> {
    let graph = FileGraph::build(sources, layout, &|_, _| false);
    let mut used: Vec<String> = graph.edges()[Path::new(from)]
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    used.sort();
    used
}

const MODULE: &[(&str, &str)] = &[("", "example.com/proj")];

#[test]
fn import_leads_to_every_file_directly_in_the_package() {
    let sources = [
        go(
            "main.go",
            "package main\n\nimport \"example.com/proj/pkg/store\"\n",
        ),
        go("pkg/store/store.go", "package store\n"),
        go("pkg/store/index.go", "package store\n"),
        go("pkg/store/sql/query.go", "package sql\n"),
        go("pkg/storefront/page.go", "package storefront\n"),
    ];
    assert_eq!(
        used(&sources, &layout(MODULE), "main.go"),
        ["pkg/store/index.go", "pkg/store/store.go"]
    );
}

#[test]
fn test_files_and_ignored_files_are_not_what_a_package_offers() {
    let sources = [
        go(
            "main.go",
            "package main\n\nimport (\n\t\"fmt\"\n\tst \"example.com/proj/store\"\n)\n",
        ),
        go("store/store.go", "package store\n"),
        go("store/store_test.go", "package store\n"),
        go("store/_draft.go", "package store\n"),
    ];
    assert_eq!(
        used(&sources, &layout(MODULE), "main.go"),
        ["store/store.go"]
    );
}

#[test]
fn external_test_package_uses_the_package_beside_it() {
    let sources = [
        go("store/store.go", "package store\n"),
        go(
            "store/api_test.go",
            "package store_test\n\nimport \"example.com/proj/store\"\n",
        ),
    ];
    assert_eq!(
        used(&sources, &layout(MODULE), "store/api_test.go"),
        ["store/store.go"]
    );
}

#[test]
fn import_of_the_module_path_is_the_package_at_its_root() {
    let sources = [
        go(
            "cmd/tool/main.go",
            "package main\n\nimport \"example.com/proj\"\n",
        ),
        go("proj.go", "package proj\n"),
        go("cmd/other/main.go", "package main\n"),
    ];
    assert_eq!(
        used(&sources, &layout(MODULE), "cmd/tool/main.go"),
        ["proj.go"]
    );
}

#[test]
fn module_path_matches_whole_segments_only() {
    let sources = [
        go(
            "main.go",
            "package main\n\nimport \"example.com/project/store\"\n",
        ),
        go("ect/store/store.go", "package store\n"),
        go("store/store.go", "package store\n"),
    ];
    assert!(used(&sources, &layout(MODULE), "main.go").is_empty());
}

#[test]
fn each_module_of_the_repository_resolves_from_its_own_directory() {
    let modules = [
        ("services/api", "example.com/api"),
        ("libs/core", "example.com/core"),
        ("libs/core/extra", "example.com/core/extra"),
    ];
    let sources = [
        go(
            "services/api/main.go",
            "package main\n\nimport (\n\t\"example.com/core/model\"\n\t\"example.com/core/extra/codec\"\n\t\"example.com/api/internal/db\"\n)\n",
        ),
        go("libs/core/model/model.go", "package model\n"),
        go("libs/core/extra/codec/codec.go", "package codec\n"),
        go("services/api/internal/db/db.go", "package db\n"),
    ];
    assert_eq!(
        used(&sources, &layout(&modules), "services/api/main.go"),
        [
            "libs/core/extra/codec/codec.go",
            "libs/core/model/model.go",
            "services/api/internal/db/db.go"
        ]
    );
}

#[test]
fn nothing_resolves_without_a_module_or_outside_it() {
    let sources = [
        go(
            "main.go",
            "package main\n\nimport (\n\t\"fmt\"\n\t\"example.com/proj/store\"\n)\n",
        ),
        go("store/store.go", "package store\n"),
    ];
    assert!(used(&sources, &layout(&[]), "main.go").is_empty());
    let other = layout(&[("", "example.com/other")]);
    assert!(used(&sources, &other, "main.go").is_empty());
}

#[test]
fn files_of_one_package_have_no_edges_between_them() {
    let sources = [
        go("store/store.go", "package store\n"),
        go("store/index.go", "package store\n"),
    ];
    assert!(used(&sources, &layout(MODULE), "store/store.go").is_empty());
}

#[test]
fn an_import_in_the_package_comment_or_of_its_own_package_is_no_edge() {
    let sources = [
        go(
            "doc.go",
            "/*\nPackage proj does things.\n\n\timport \"example.com/proj/render\"\n*/\npackage proj\n",
        ),
        go(
            "main.go",
            "package proj\n\nimport \"example.com/proj\"\n\nfunc Run() {}\n",
        ),
        go("render/render.go", "package render\n"),
    ];
    assert!(used(&sources, &layout(MODULE), "doc.go").is_empty());
    assert!(used(&sources, &layout(MODULE), "main.go").is_empty());
}
