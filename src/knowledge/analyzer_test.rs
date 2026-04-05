use std::collections::HashMap;

use super::*;

fn make_blame(author: &str, email: &str, lines: usize, time: i64) -> BlameInfo {
    BlameInfo {
        author: author.to_string(),
        email: email.to_string(),
        lines,
        last_commit_time: time,
    }
}

#[test]
fn test_critical_single_owner() {
    let blames = vec![make_blame("Alice", "alice@x.com", 90, 100)];
    let recent = HashSet::from(["alice@x.com".to_string()]);
    let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);

    assert_eq!(result.risk, RiskLevel::Critical);
    assert_eq!(result.primary_owner, "Alice");
    assert!((result.ownership_pct - 100.0).abs() < 0.01);
    assert!(!result.knowledge_loss);
}

#[test]
fn test_high_risk() {
    let blames = vec![
        make_blame("Alice", "alice@x.com", 70, 100),
        make_blame("Bob", "bob@x.com", 30, 100),
    ];
    let recent = HashSet::new();
    let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
    assert_eq!(result.risk, RiskLevel::High);
}

#[test]
fn test_medium_risk() {
    let blames = vec![
        make_blame("Alice", "alice@x.com", 50, 100),
        make_blame("Bob", "bob@x.com", 40, 100),
        make_blame("Carol", "carol@x.com", 10, 100),
    ];
    let recent = HashSet::new();
    let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
    assert_eq!(result.risk, RiskLevel::Medium);
}

#[test]
fn test_low_risk() {
    let blames = vec![
        make_blame("Alice", "alice@x.com", 25, 100),
        make_blame("Bob", "bob@x.com", 25, 100),
        make_blame("Carol", "carol@x.com", 25, 100),
        make_blame("Dan", "dan@x.com", 25, 100),
    ];
    let recent = HashSet::new();
    let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
    assert_eq!(result.risk, RiskLevel::Low);
}

#[test]
fn test_knowledge_loss_detected() {
    let blames = vec![make_blame("Alice", "alice@x.com", 100, 100)];
    // Alice is NOT in recent authors → knowledge loss
    let recent = HashSet::from(["bob@x.com".to_string()]);
    let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
    assert!(result.knowledge_loss);
}

#[test]
fn test_no_knowledge_loss_when_no_since() {
    let blames = vec![make_blame("Alice", "alice@x.com", 100, 100)];
    // Empty recent_authors means --since was not used → no knowledge loss check
    let recent = HashSet::new();
    let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
    assert!(!result.knowledge_loss);
}

#[test]
fn test_empty_blames() {
    let blames: Vec<BlameInfo> = vec![];
    let recent = HashSet::new();
    let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
    assert_eq!(result.total_lines, 0);
    assert_eq!(result.risk, RiskLevel::Low);
}

#[test]
fn test_contributors_count() {
    let blames = vec![
        make_blame("Alice", "alice@x.com", 50, 100),
        make_blame("Bob", "bob@x.com", 50, 100),
    ];
    let recent = HashSet::new();
    let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
    assert_eq!(result.contributors, 2);
}

// --- bus factor tests ---

fn author_map(entries: &[(&str, usize)]) -> HashMap<String, usize> {
    entries.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

#[test]
fn bus_factor_single_dominant_owner() {
    // One person owns 90% → bus factor = 1
    let map = author_map(&[("Alice", 90), ("Bob", 10)]);
    let bf = compute_bus_factor(&map, 80.0);
    assert_eq!(bf.factor, 1);
    assert_eq!(bf.total_lines, 100);
    assert!(bf.contributors[0].is_critical);
    assert!(!bf.contributors[1].is_critical);
}

#[test]
fn bus_factor_two_owners_needed() {
    // Alice 69%, Carol 20%, Bob 11% → sorted: Alice, Carol, Bob
    // Alice alone: 69% < 80%, Alice+Carol: 89% ≥ 80% → bus factor = 2
    let map = author_map(&[("Alice", 69), ("Bob", 11), ("Carol", 20)]);
    let bf = compute_bus_factor(&map, 80.0);
    assert_eq!(bf.factor, 2);
    assert_eq!(bf.contributors[0].author, "Alice");
    assert_eq!(bf.contributors[1].author, "Carol");
}

#[test]
fn bus_factor_exact_threshold() {
    // Alice owns exactly 80% → bus factor = 1
    let map = author_map(&[("Alice", 80), ("Bob", 20)]);
    let bf = compute_bus_factor(&map, 80.0);
    assert_eq!(bf.factor, 1);
}

#[test]
fn bus_factor_empty() {
    let map = author_map(&[]);
    let bf = compute_bus_factor(&map, 80.0);
    assert_eq!(bf.factor, 0);
    assert_eq!(bf.total_lines, 0);
    assert!(bf.contributors.is_empty());
}

#[test]
fn bus_factor_cumulative_pct_reaches_100() {
    let map = author_map(&[("Alice", 50), ("Bob", 50)]);
    let bf = compute_bus_factor(&map, 80.0);
    let last = bf.contributors.last().unwrap();
    assert!((last.cumulative_pct - 100.0).abs() < 0.01);
}

#[test]
fn bus_factor_contributors_sorted_descending() {
    let map = author_map(&[("C", 10), ("A", 50), ("B", 40)]);
    let bf = compute_bus_factor(&map, 80.0);
    assert_eq!(bf.contributors[0].author, "A");
    assert_eq!(bf.contributors[1].author, "B");
    assert_eq!(bf.contributors[2].author, "C");
}
