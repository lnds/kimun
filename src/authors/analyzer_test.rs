use super::*;
use crate::git::BlameInfo;

fn blame(author: &str, email: &str, lines: usize, time: i64) -> BlameInfo {
    BlameInfo {
        author: author.to_string(),
        email: email.to_string(),
        lines,
        last_commit_time: time,
    }
}

#[test]
fn single_author_owns_file() {
    let blames = vec![blame("Alice", "alice@example.com", 100, 1000)];
    let result = compute_authors(&[("Rust", &blames)]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Alice");
    assert_eq!(result[0].owned_files, 1);
    assert_eq!(result[0].lines, 100);
    assert_eq!(result[0].languages, vec!["Rust"]);
    assert_eq!(result[0].last_active, 1000);
}

#[test]
fn primary_owner_is_author_with_most_lines() {
    // blames are sorted by lines desc — first entry is primary owner
    let blames = vec![
        blame("Alice", "alice@example.com", 80, 2000),
        blame("Bob", "bob@example.com", 20, 1000),
    ];
    let result = compute_authors(&[("Rust", &blames)]);
    let alice = result.iter().find(|a| a.name == "Alice").unwrap();
    let bob = result.iter().find(|a| a.name == "Bob").unwrap();
    assert_eq!(alice.owned_files, 1);
    assert_eq!(bob.owned_files, 0);
}

#[test]
fn authors_with_as_many_lines_come_by_identity() {
    let first = vec![blame("Zoe", "zoe@example.com", 10, 1000)];
    let second = vec![blame("Ana", "ana@example.com", 10, 1000)];
    for files in [
        [("Rust", first.as_slice()), ("Rust", second.as_slice())],
        [("Rust", second.as_slice()), ("Rust", first.as_slice())],
    ] {
        let names: Vec<String> = compute_authors(&files)
            .into_iter()
            .map(|a| a.name)
            .collect();
        assert_eq!(names, ["Ana", "Zoe"]);
    }
}

#[test]
fn lines_accumulated_across_files() {
    let blames_a = vec![blame("Alice", "alice@example.com", 50, 1000)];
    let blames_b = vec![blame("Alice", "alice@example.com", 30, 2000)];
    let result = compute_authors(&[("Rust", &blames_a), ("Python", &blames_b)]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].lines, 80);
    assert_eq!(result[0].owned_files, 2);
}

#[test]
fn languages_are_sorted_and_deduplicated() {
    let blames_a = vec![blame("Alice", "alice@example.com", 10, 1000)];
    let blames_b = vec![blame("Alice", "alice@example.com", 10, 1000)];
    let blames_c = vec![blame("Alice", "alice@example.com", 10, 1000)];
    let result = compute_authors(&[
        ("Rust", &blames_a),
        ("Python", &blames_b),
        ("Rust", &blames_c),
    ]);
    assert_eq!(result[0].languages, vec!["Python", "Rust"]);
}

#[test]
fn last_active_is_most_recent_commit() {
    let blames = vec![
        blame("Alice", "alice@example.com", 50, 3000),
        blame("Alice", "alice@example.com", 50, 1000),
    ];
    let result = compute_authors(&[("Rust", &blames)]);
    assert_eq!(result[0].last_active, 3000);
}

#[test]
fn sorted_by_lines_descending() {
    let blames_alice = vec![blame("Alice", "alice@example.com", 200, 1000)];
    let blames_bob = vec![blame("Bob", "bob@example.com", 50, 1000)];
    let result = compute_authors(&[("Rust", &blames_alice), ("Rust", &blames_bob)]);
    assert_eq!(result[0].name, "Alice");
    assert_eq!(result[1].name, "Bob");
}

#[test]
fn empty_input_returns_empty() {
    let result = compute_authors(&[]);
    assert!(result.is_empty());
}

#[test]
fn last_active_is_the_last_commit() {
    let blames = [BlameInfo {
        author: "Ana".to_string(),
        email: "Ana@x.org".to_string(),
        lines: 2,
        last_commit_time: 100,
    }];
    let mut authors = compute_authors(&[("Rust", &blames[..])]);
    let last = HashMap::from([("ana@x.org".to_string(), 500)]);
    set_last_commits(&mut authors, &last);
    assert_eq!(authors[0].last_active, 500);
}
