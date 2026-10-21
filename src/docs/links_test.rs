use super::*;

fn graph(documents: &[(&str, &str)], others: &[&str]) -> Graph {
    let documents: Vec<(PathBuf, String)> = documents
        .iter()
        .map(|(p, t)| (PathBuf::from(p), t.to_string()))
        .collect();
    let others: Vec<PathBuf> = others.iter().map(PathBuf::from).collect();
    Graph::build(&documents, |path| others.iter().any(|o| o == path))
}

#[test]
fn markdown_links_and_wikilinks_become_edges() {
    let g = graph(
        &[
            ("README.md", "[guide](docs/guide.md) and [[Payments]]\n"),
            (
                "docs/guide.md",
                "Back to [readme](../README.md#top), see [[kb/payments|pay]]\n",
            ),
            ("kb/payments.md", "# Payments\n"),
        ],
        &[],
    );
    assert_eq!(g.out[0], BTreeSet::from([1, 2]));
    assert_eq!(g.out[1], BTreeSet::from([0, 2]));
    assert!(g.out[2].is_empty());
    assert!(g.broken.is_empty());
    assert_eq!(g.fan_in(), [1, 1, 2]);
}

#[test]
fn links_to_nothing_are_broken_and_to_other_files_are_not_edges() {
    let g = graph(
        &[(
            "a.md",
            "[x](missing.md) [[Nowhere]] [code](src/a.rs) [web](https://x.y) [here](#top) [r](/docs/score/)\n",
        )],
        &["src/a.rs"],
    );
    assert!(g.out[0].is_empty());
    let broken: Vec<&str> = g.broken.iter().map(|b| b.target.as_str()).collect();
    assert_eq!(broken, ["missing.md", "Nowhere"]);
}

#[test]
fn a_note_named_by_several_files_is_the_nearest() {
    let g = graph(
        &[
            ("team/a/index.md", "[[notes]]\n"),
            ("team/a/notes.md", ""),
            ("team/b/notes.md", ""),
            ("other.md", "[[notes]]\n"),
        ],
        &[],
    );
    assert_eq!(g.out[0], BTreeSet::from([1]));
    assert!(g.out[3].is_empty(), "two notes are as near: neither");
    assert!(g.broken.is_empty());
}

#[test]
fn wikilinks_ignore_case_and_extension() {
    let g = graph(
        &[
            ("a.md", "[[Kb/Refunds.md]] [[REFUNDS#Rules]]\n"),
            ("kb/refunds.md", ""),
        ],
        &[],
    );
    assert_eq!(g.out[0], BTreeSet::from([1]));
}

#[test]
fn groups_ignore_direction_largest_first() {
    let g = graph(
        &[
            ("a.md", "[b](b.md)"),
            ("b.md", ""),
            ("c.md", "[a](a.md)"),
            ("d.md", "[e](e.md)"),
            ("e.md", ""),
            ("f.md", ""),
        ],
        &[],
    );
    assert_eq!(g.groups(), vec![vec![0, 1, 2], vec![3, 4], vec![5]]);
}

#[test]
fn entry_points_are_read_first() {
    assert!(is_entry_point(Path::new("docs/README.md")));
    assert!(is_entry_point(Path::new("kb/index.md")));
    assert!(is_entry_point(Path::new("AGENTS.md")));
    assert!(!is_entry_point(Path::new("kb/payments.md")));
}
