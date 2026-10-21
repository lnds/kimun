use super::*;
use std::fs;

/// A repository on disk with `files`, each with its text.
fn repository(files: &[(&str, &str)]) -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let full = dir.path().join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, text).unwrap();
    }
    let root = dir.path().canonicalize().unwrap();
    let repository = Repository::index(&root, || None);
    (dir, repository)
}

fn dead(repository: &Repository, document: &str, text: &str) -> Vec<(String, Missing)> {
    let path = repository.root().join(document);
    check(&path, text, repository)
        .dead
        .into_iter()
        .map(|d| (d.reference, d.missing))
        .collect()
}

const FILES: &[(&str, &str)] = &[
    ("README.md", "# Kimün\n## Install\n"),
    ("docs/guide.md", "# Guide\n## Setup steps\n"),
    ("docs/img/logo.png", ""),
    ("src/main.rs", "fn main() {}\n"),
    ("src/deps/graph.rs", ""),
];

#[test]
fn links_to_files_that_exist_are_alive() {
    let (_dir, repository) = repository(FILES);
    let text = "[guide](docs/guide.md) [setup](docs/guide.md#setup-steps) ![](docs/img/logo.png)\n\
                [main](/src/main.rs) [src](src/) [web](https://example.com) [mail](mailto:a@b.c)\n";
    assert!(dead(&repository, "README.md", text).is_empty());
}

#[test]
fn links_relative_to_the_document() {
    let (_dir, repository) = repository(FILES);
    let text = "# Guide\n[readme](../README.md#install) [logo](img/logo.png) [top](#guide)\n";
    assert!(dead(&repository, "docs/guide.md", text).is_empty());
}

#[test]
fn missing_files_and_headings_are_dead() {
    let (_dir, repository) = repository(FILES);
    let text =
        "[old](docs/old.md) ![x](img/x.png) [bad](docs/guide.md#teardown) [here](#nowhere)\n";
    assert_eq!(
        dead(&repository, "README.md", text),
        vec![
            ("docs/old.md".to_string(), Missing::File),
            ("img/x.png".to_string(), Missing::File),
            ("docs/guide.md#teardown".to_string(), Missing::Heading),
            ("#nowhere".to_string(), Missing::Heading),
        ]
    );
}

#[test]
fn escaped_spaces_and_anchors_of_other_files_than_markdown() {
    let (_dir, repository) = repository(&[("my notes.md", "# A\n"), ("src/a.rs", "")]);
    let text = "[n](my%20notes.md#a) [code](src/a.rs#L10)\n";
    assert!(dead(&repository, "README.md", text).is_empty());
}

#[test]
fn paths_in_code_spans() {
    let (_dir, repository) = repository(FILES);
    let text = "`src/main.rs` `src/main.rs:12` `deps/graph.rs` `src/deps/` `./docs/guide.md`\n\
                `src/old.rs` `docs/gone/` `deps/tree.rs`\n";
    assert_eq!(
        dead(&repository, "README.md", text),
        vec![
            ("src/old.rs".to_string(), Missing::Path),
            ("docs/gone/".to_string(), Missing::Path),
            ("deps/tree.rs".to_string(), Missing::Path),
        ]
    );
}

#[test]
fn code_spans_that_are_no_paths_of_the_repository_are_not_read() {
    let (_dir, repository) = repository(FILES);
    let text = "`origin/main` `text/html` `km loc src/` `vendor/**` `/usr/bin/env` \
                `~/.claude/x.json` `lnds/kimun` `1/2` `src` `a//b` `$HOME/src/x`\n";
    let checked = check(&repository.root().join("README.md"), text, &repository);
    assert_eq!(checked.references, 0);
}

#[test]
fn a_path_relative_to_the_document_must_exist_from_it() {
    let (_dir, repository) = repository(FILES);
    assert!(dead(&repository, "docs/guide.md", "`../src/main.rs`").is_empty());
    assert_eq!(
        dead(&repository, "docs/guide.md", "`../src/gone.rs`"),
        vec![("../src/gone.rs".to_string(), Missing::Path)]
    );
}

#[test]
fn a_dead_reference_names_the_path_from_the_root() {
    let (_dir, repository) = repository(FILES);
    let path = repository.root().join("docs/guide.md");
    let names: Vec<PathBuf> = check(&path, "[a](../src/old.rs) `src/gone.rs`", &repository)
        .dead
        .into_iter()
        .map(|d| d.names)
        .collect();
    assert_eq!(
        names,
        vec![PathBuf::from("src/old.rs"), PathBuf::from("src/gone.rs")]
    );
}

#[test]
fn with_history_only_paths_it_held_are_read() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "").unwrap();
    let root = dir.path().canonicalize().unwrap();
    let history = || Some(["src/main.rs", "src/old/gone.rs"].map(PathBuf::from).into());
    let repository = Repository::index(&root, history);
    let text = "`src/main.rs` `src/lib.rs` `old/gone.rs` `src/old/` `lib/a/b.ex`";
    assert_eq!(
        dead(&repository, "README.md", text),
        vec![
            ("old/gone.rs".to_string(), Missing::Path),
            ("src/old/".to_string(), Missing::Path),
        ]
    );
}

#[test]
fn absolute_links_without_extension_are_routes_of_a_site() {
    let (_dir, repository) = repository(FILES);
    let text = "[score](/en/docs/score/) [k](/docs/knowledge#who) [x](/docs/x.md)";
    assert_eq!(
        dead(&repository, "README.md", text),
        vec![("/docs/x.md".to_string(), Missing::File)]
    );
}

#[test]
fn ignored_files_are_no_paths_of_the_repository() {
    let (dir, _) = repository(&[(".gitignore", "build/\n"), ("build/out/a.rs", "")]);
    std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    let repository = Repository::index(&dir.path().canonicalize().unwrap(), || None);
    let checked = check(
        &repository.root().join("README.md"),
        "`out/a.rs`",
        &repository,
    );
    assert_eq!(checked.references, 0);
}

#[test]
fn path_in_strips_line_numbers() {
    assert_eq!(path_in("src/a.rs:10:4"), Some("src/a.rs"));
    assert_eq!(path_in("src/a.rs"), Some("src/a.rs"));
    assert_eq!(path_in("../"), None);
    assert_eq!(path_in("a.rs"), None);
}
