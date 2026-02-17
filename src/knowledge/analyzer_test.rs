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
