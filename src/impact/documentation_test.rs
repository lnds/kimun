use super::*;
use crate::config::{DocsConfig, OneOrMany};
use crate::impact::analyzer::Trigger;

fn change(path: &str, kind: ChangeKind, old: Option<&str>) -> FileDiffStat {
    FileDiffStat {
        path: PathBuf::from(path),
        old_path: old.map(PathBuf::from),
        kind,
        added: 1,
        deleted: 1,
        lines: Vec::new(),
        old_lines: Vec::new(),
        probe: Vec::new(),
    }
}

fn modified(path: &str) -> FileDiffStat {
    change(path, ChangeKind::Modified, Some(path))
}

fn documents(docs: &[(&str, &str)]) -> Vec<(PathBuf, String)> {
    docs.iter()
        .map(|(path, text)| (PathBuf::from(path), text.to_string()))
        .collect()
}

fn rules(covers: &[(&str, &str)], ignore: &[&str]) -> Rules {
    let config = DocsConfig {
        covers: covers
            .iter()
            .map(|(code, doc)| (code.to_string(), OneOrMany::One(doc.to_string())))
            .collect(),
        ignore: ignore.iter().map(|s| s.to_string()).collect(),
    };
    Rules::new(&config).unwrap()
}

fn summary(stale: &[Stale]) -> Vec<(String, Signal, Vec<String>)> {
    stale
        .iter()
        .map(|s| {
            (
                s.document.display().to_string(),
                s.signal,
                s.describes
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn front_matter_declares_what_a_document_covers() {
    let docs = documents(&[
        (
            "kb/payments.md",
            "---\ntitle: Payments\ncovers: [\"apps/payments/**\", lib/billing/invoice.ex]\n---\n# Payments\n",
        ),
        (
            "kb/billing.md",
            "---\ncovers:\n  - lib/billing\n  - 'lib/tax.ex'\nowner: billing\n---\n",
        ),
        ("kb/other.md", "---\ncovers: apps/other/**\n---\n"),
    ]);
    let changes = [
        modified("apps/payments/lib/refund.ex"),
        modified("lib/billing/invoice.ex"),
    ];
    let stale = stale(&docs, &changes, &[], &rules(&[], &[]));
    assert_eq!(
        summary(&stale),
        vec![
            (
                "kb/billing.md".to_string(),
                Signal::Declared,
                vec!["lib/billing/invoice.ex".to_string()]
            ),
            (
                "kb/payments.md".to_string(),
                Signal::Declared,
                vec![
                    "apps/payments/lib/refund.ex".to_string(),
                    "lib/billing/invoice.ex".to_string()
                ]
            ),
        ]
    );
    assert_eq!(stale[1].why, Why::Covers("apps/payments/**".to_string()));
}

#[test]
fn the_configuration_maps_code_to_documents() {
    let docs = documents(&[("kb/payments/refunds.md", "# Refunds\n")]);
    let changes = [modified("apps/payments/lib/refund.ex")];
    let rules = rules(&[("apps/payments/**", "kb/payments/**")], &[]);
    let stale = stale(&docs, &changes, &[], &rules);
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].signal, Signal::Declared);
}

#[test]
fn a_document_that_names_a_changed_file() {
    let docs = documents(&[
        (
            "docs/architecture.md",
            "# Architecture\n\nInvoices live in `lib/billing/invoice.ex`.\n\
             See [the parser](../src/parse/mod.rs) and `deps/graph.rs`.\n",
        ),
        ("docs/unrelated.md", "Nothing about `lib/other.ex`.\n"),
    ]);
    let changes = [
        modified("lib/billing/invoice.ex"),
        modified("src/parse/mod.rs"),
        modified("src/deps/graph.rs"),
    ];
    let stale = stale(&docs, &changes, &[], &rules(&[], &[]));
    assert_eq!(
        summary(&stale),
        vec![(
            "docs/architecture.md".to_string(),
            Signal::Referenced,
            vec![
                "lib/billing/invoice.ex".to_string(),
                "src/deps/graph.rs".to_string(),
                "src/parse/mod.rs".to_string()
            ]
        )]
    );
    assert_eq!(
        stale[0].why,
        Why::Names {
            line: 3,
            gone: false
        }
    );
}

#[test]
fn a_reference_to_a_deleted_or_moved_file_is_gone() {
    let docs = documents(&[
        ("a.md", "Run `scripts/old.sh`.\n"),
        ("b.md", "Read `src/before.rs`.\n"),
    ]);
    let changes = [
        change(
            "scripts/old.sh",
            ChangeKind::Deleted,
            Some("scripts/old.sh"),
        ),
        change("src/after.rs", ChangeKind::Renamed, Some("src/before.rs")),
    ];
    let stale = stale(&docs, &changes, &[], &rules(&[], &[]));
    assert_eq!(stale.len(), 2);
    assert!(
        stale
            .iter()
            .all(|s| matches!(s.why, Why::Names { gone: true, .. }))
    );
    assert_eq!(stale[1].describes, vec![PathBuf::from("src/after.rs")]);
}

#[test]
fn a_directory_names_what_is_under_it_but_a_top_directory_does_not() {
    let docs = documents(&[
        ("a.md", "The module `lib/billing/` does it.\n"),
        ("b.md", "Code is under `src/`.\n"),
    ]);
    let changes = [modified("lib/billing/invoice.ex"), modified("src/main.rs")];
    let stale = stale(&docs, &changes, &[], &rules(&[], &[]));
    assert_eq!(
        summary(&stale),
        vec![(
            "a.md".to_string(),
            Signal::Referenced,
            vec!["lib/billing/invoice.ex".to_string()]
        )]
    );
}

#[test]
fn documents_in_the_change_changelogs_and_ignored_ones_are_left_out() {
    let text = "Touches `lib/billing/invoice.ex`.\n";
    let docs = documents(&[
        ("kb/updated.md", text),
        ("CHANGELOG.md", text),
        ("docs/adr/0001-billing.md", text),
    ]);
    let changes = [
        modified("lib/billing/invoice.ex"),
        modified("kb/updated.md"),
    ];
    let stale = stale(&docs, &changes, &[], &rules(&[], &["docs/adr/**"]));
    assert!(stale.is_empty());
}

#[test]
fn documents_that_change_with_the_code_follow_the_others() {
    let docs = documents(&[("kb/a.md", "Names `lib/billing/invoice.ex`.\n")]);
    let changes = [modified("lib/billing/invoice.ex")];
    let trigger = Trigger {
        path: PathBuf::from("lib/billing/invoice.ex"),
        shared_commits: 7,
        commits: 9,
        confidence: 7.0 / 9.0,
    };
    let co_changed = [
        MissingCoChange {
            path: PathBuf::from("docs/billing/README.md"),
            triggers: vec![trigger.clone()],
        },
        MissingCoChange {
            path: PathBuf::from("kb/a.md"),
            triggers: vec![trigger],
        },
    ];
    let stale = stale(&docs, &changes, &co_changed, &rules(&[], &[]));
    assert_eq!(
        summary(&stale),
        vec![
            (
                "kb/a.md".to_string(),
                Signal::Referenced,
                vec!["lib/billing/invoice.ex".to_string()]
            ),
            (
                "docs/billing/README.md".to_string(),
                Signal::CoChanged,
                vec!["lib/billing/invoice.ex".to_string()]
            ),
        ]
    );
    assert!(matches!(
        stale[1].why,
        Why::ChangedWith {
            shared: 7,
            commits: 9,
            ..
        }
    ));
}

#[test]
fn links_to_sites_and_anchors_alone_name_nothing() {
    let docs = documents(&[(
        "a.md",
        "[x](https://example.com/lib/billing/invoice.ex) [y](#lib)\n",
    )]);
    let changes = [modified("lib/billing/invoice.ex")];
    assert!(stale(&docs, &changes, &[], &rules(&[], &[])).is_empty());
}

#[test]
fn is_document_by_extension() {
    assert!(is_document(Path::new("kb/a.md")));
    assert!(is_document(Path::new("README.MDX")));
    assert!(!is_document(Path::new("notes.txt")));
}

#[test]
fn a_changed_document_is_not_described_by_another() {
    let docs = documents(&[("README.md", "See [the guide](docs/guide.md).\n")]);
    let changes = [modified("docs/guide.md")];
    assert!(stale(&docs, &changes, &[], &rules(&[], &[])).is_empty());
}
