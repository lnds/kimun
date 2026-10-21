use super::*;

#[test]
fn covers_in_front_matter_as_a_list_a_block_or_a_value() {
    assert_eq!(
        declared_in("---\ncovers: [\"a/**\", b.ex]\n---\n"),
        ["a/**", "b.ex"]
    );
    assert_eq!(
        declared_in("---\ncovers:\n  - lib/x\n  - 'lib/y.ex'\nowner: z\n---\n"),
        ["lib/x", "lib/y.ex"]
    );
    assert_eq!(declared_in("---\ncovers: apps/**\n---\n"), ["apps/**"]);
    assert!(declared_in("covers: apps/**\n").is_empty());
}

#[test]
fn a_glob_covers_what_is_under_a_directory() {
    let set = globs(&["lib/billing".to_string()]).unwrap();
    assert!(set.is_match("lib/billing/invoice.ex"));
    assert!(!set.is_match("lib/billing_old.ex"));
}

#[test]
fn a_reference_names_a_path_whole_by_its_end_or_from_above() {
    assert!(names(Path::new("src/a.rs"), true, Path::new("src/a.rs")));
    assert!(names(
        Path::new("deps/graph.rs"),
        false,
        Path::new("src/deps/graph.rs")
    ));
    assert!(!names(
        Path::new("deps/graph.rs"),
        true,
        Path::new("src/deps/graph.rs")
    ));
    assert!(names(
        Path::new("lib/billing"),
        true,
        Path::new("lib/billing/x.ex")
    ));
    assert!(!names(Path::new("src"), false, Path::new("src/main.rs")));
}
