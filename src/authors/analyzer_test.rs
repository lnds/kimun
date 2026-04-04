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
