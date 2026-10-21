use super::*;
use crate::config::{DocsConfig, OneOrMany};

const DAY: i64 = 86_400;

fn files(paths: &[&str]) -> Files {
    Files::new(paths.iter().map(PathBuf::from))
}

/// The history of each file: the commits that touched it, as numbers
/// and days.
fn history(entries: &[(&str, &[(usize, i64)])]) -> CommitsByFile {
    entries
        .iter()
        .map(|(path, commits)| {
            let commits = commits.iter().map(|(n, day)| (*n, day * DAY)).collect();
            (PathBuf::from(path), commits)
        })
        .collect()
}

fn rules(covers: &[(&str, &str)]) -> Rules {
    let config = DocsConfig {
        covers: covers
            .iter()
            .map(|(code, doc)| (code.to_string(), OneOrMany::One(doc.to_string())))
            .collect(),
        ignore: Vec::new(),
    };
    Rules::new(&config).unwrap()
}

fn document(path: &str, text: &str) -> (PathBuf, String) {
    (PathBuf::from(path), text.to_string())
}

#[test]
fn a_document_behind_the_files_it_names() {
    let files = files(&["kb/a.md", "src/a.rs", "src/b.rs", "src/deps/graph.rs"]);
    let history = history(&[
        ("kb/a.md", &[(3, 10)]),
        ("src/a.rs", &[(5, 30), (4, 20), (1, 1)]),
        ("src/b.rs", &[(4, 20)]),
        ("src/deps/graph.rs", &[(2, 5)]),
    ]);
    let docs = [document(
        "kb/a.md",
        "`src/a.rs` and [b](../src/b.rs) and `deps/graph.rs`\n",
    )];
    let stale = stale(&docs, &files, &history, &rules(&[]));
    assert_eq!(stale.len(), 1);
    let found = &stale[0];
    assert_eq!(found.updated, 10 * DAY);
    assert_eq!(found.commits, 2, "commit 4 touched both files");
    assert_eq!(found.behind(), 20 * DAY);
    let behind: Vec<(&str, usize)> = found
        .files
        .iter()
        .map(|f| (f.path.to_str().unwrap(), f.commits))
        .collect();
    assert_eq!(behind, [("src/a.rs", 2), ("src/b.rs", 1)]);
}

#[test]
fn covered_files_count_and_documents_are_never_described() {
    let files = files(&["kb/pay.md", "kb/other.md", "apps/pay/x.ex"]);
    let history = history(&[
        ("kb/pay.md", &[(1, 1)]),
        ("kb/other.md", &[(3, 9)]),
        ("apps/pay/x.ex", &[(2, 5)]),
    ]);
    let docs = [document("kb/pay.md", "See [other](other.md).\n")];
    let stale = stale(
        &docs,
        &files,
        &history,
        &rules(&[("apps/pay/**", "kb/pay.md")]),
    );
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].files[0].path, PathBuf::from("apps/pay/x.ex"));
    assert_eq!(stale[0].files.len(), 1);
}

#[test]
fn up_to_date_uncommitted_and_changelog_documents_are_not_stale() {
    let files = files(&["README.md", "CHANGELOG.md", "src/a.rs"]);
    let history = history(&[
        ("README.md", &[(2, 9)]),
        ("CHANGELOG.md", &[(1, 1)]),
        ("src/a.rs", &[(2, 9)]),
    ]);
    let text = "`src/a.rs`\n";
    let docs = [
        document("README.md", text),
        document("CHANGELOG.md", text),
        document("NEW.md", text),
    ];
    assert!(stale(&docs, &files, &history, &rules(&[])).is_empty());
}

#[test]
fn furthest_behind_first() {
    let files = files(&["a.md", "b.md", "src/x.rs", "src/y.rs"]);
    let history = history(&[
        ("a.md", &[(1, 1)]),
        ("b.md", &[(2, 2)]),
        ("src/x.rs", &[(3, 5)]),
        ("src/y.rs", &[(4, 50)]),
    ]);
    let docs = [
        document("a.md", "`src/x.rs`"),
        document("b.md", "`src/y.rs`"),
    ];
    let stale = stale(&docs, &files, &history, &rules(&[]));
    let order: Vec<&Path> = stale.iter().map(|s| s.document.as_path()).collect();
    assert_eq!(order, [Path::new("b.md"), Path::new("a.md")]);
}

#[test]
fn a_directory_names_the_files_under_it() {
    let files = files(&["lib/billing/a.ex", "lib/billing/b.ex", "lib/other.ex"]);
    assert_eq!(files.named(Path::new("lib/billing"), true).len(), 2);
    assert!(files.named(Path::new("lib"), true).is_empty());
    assert_eq!(files.named(Path::new("billing/a.ex"), false).len(), 1);
    assert!(files.named(Path::new("billing/a.ex"), true).is_empty());
}
