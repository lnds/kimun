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

/// One file: `(author, lines)` with the author's name as email.
fn file(authors: &[(&str, usize)]) -> Vec<BlameInfo> {
    authors
        .iter()
        .map(|(a, n)| make_blame(a, &format!("{a}@x.org"), *n, 100))
        .collect()
}

/// Two authors of 30 lines and eight of 5, each in a file of their own.
fn long_tail() -> Vec<Vec<BlameInfo>> {
    let mut files = vec![file(&[("Ana", 30)]), file(&[("Bea", 30)])];
    for dev in ["c", "d", "e", "f", "g", "h", "i", "j"] {
        files.push(file(&[(dev, 5)]));
    }
    files
}

/// Active authors as `recent_authors` gives them: by identity.
fn active(names: &[&str]) -> HashSet<String> {
    names
        .iter()
        .map(|n| crate::git::identity(n, &format!("{n}@x.org")))
        .collect()
}

#[test]
fn bus_factor_single_author() {
    let bf = compute_bus_factor(&[file(&[("Ana", 10)])], None, 50.0);
    assert_eq!(bf.factor, 1);
    assert_eq!(bf.total_files, 1);
    assert_eq!(bf.total_lines, 10);
    assert_eq!(bf.contributors[0].orphaned_pct, Some(100.0));
}

#[test]
fn bus_factor_ignores_the_long_tail() {
    let bf = compute_bus_factor(&long_tail(), None, 50.0);
    assert_eq!(bf.factor, 2);
    let leaving: Vec<(&str, Option<f64>)> = bf
        .contributors
        .iter()
        .map(|e| (e.author.as_str(), e.orphaned_pct))
        .take(3)
        .collect();
    assert_eq!(
        leaving,
        [("Ana", Some(30.0)), ("Bea", Some(60.0)), ("c", None)]
    );
    assert!(bf.contributors[1].is_critical);
    assert!(!bf.contributors[2].is_critical);
    assert_eq!(bf.contributors.len(), 10);
}

#[test]
fn bus_factor_threshold_is_exceeded_not_reached() {
    // 30, 60, 65, 70 do not exceed 70; 75 does.
    let bf = compute_bus_factor(&long_tail(), None, 70.0);
    assert_eq!(bf.factor, 5);
}

#[test]
fn bus_factor_shared_file_needs_both_authors_gone() {
    // 45 is at least 75% of 50, so Bea is an author too.
    let bf = compute_bus_factor(&[file(&[("Ana", 50), ("Bea", 45)])], None, 50.0);
    assert_eq!(bf.factor, 2);
    assert_eq!(bf.contributors[0].orphaned_pct, Some(0.0));
    assert_eq!(bf.contributors[1].orphaned_pct, Some(100.0));
}

#[test]
fn bus_factor_minor_contributor_is_not_an_author() {
    let bf = compute_bus_factor(&[file(&[("Ana", 80), ("Bea", 20)])], None, 50.0);
    assert_eq!(bf.factor, 1);
    assert_eq!(bf.contributors[0].files, 1);
    assert_eq!(bf.contributors[1].files, 0);
    assert_eq!(bf.contributors[1].lines, 20);
}

#[test]
fn bus_factor_is_zero_when_inactive_authors_left_most_code() {
    let files = [file(&[("Ana", 90)]), file(&[("Bea", 10)])];
    let bf = compute_bus_factor(&files, Some(&active(&["Bea"])), 50.0);
    assert_eq!(bf.factor, 0);
    assert_eq!(bf.orphaned_by_inactivity, Some(90.0));
    assert_eq!(bf.contributors[0].author, "Ana");
    assert_eq!(bf.contributors[0].active, Some(false));
    assert!(!bf.contributors[0].is_critical);
    assert_eq!(bf.contributors[1].active, Some(true));
}

#[test]
fn bus_factor_counts_from_what_inactive_authors_left() {
    // Bea is gone: 30% orphaned before anyone is counted, Ana takes it to 90%.
    let files = [
        file(&[("Ana", 60)]),
        file(&[("Bea", 30)]),
        file(&[("Cid", 10)]),
    ];
    let bf = compute_bus_factor(&files, Some(&active(&["Ana", "Cid"])), 50.0);
    assert_eq!(bf.factor, 1);
    assert_eq!(bf.orphaned_by_inactivity, Some(30.0));
    assert_eq!(bf.contributors[0].orphaned_pct, Some(90.0));
}

#[test]
fn bus_factor_without_window_knows_no_activity() {
    let bf = compute_bus_factor(&long_tail(), None, 50.0);
    assert_eq!(bf.orphaned_by_inactivity, None);
    assert!(bf.contributors.iter().all(|e| e.active.is_none()));
}

#[test]
fn bus_factor_joins_names_of_one_email() {
    let files = [
        vec![make_blame("Victor Z", "v@x.org", 45, 100)],
        vec![make_blame("vitaut", "V@X.org", 40, 100)],
        file(&[("Bea", 15)]),
    ];
    let bf = compute_bus_factor(&files, None, 50.0);
    assert_eq!(bf.factor, 1);
    assert_eq!(
        bf.contributors[0].author, "Victor Z",
        "the name with most lines"
    );
    assert_eq!(bf.contributors[0].lines, 85);
    assert_eq!(bf.contributors[0].files, 2);
}

#[test]
fn bus_factor_empty() {
    let bf = compute_bus_factor(&[], None, 50.0);
    assert_eq!(bf.factor, 0);
    assert_eq!(bf.total_lines, 0);
    assert!(bf.contributors.is_empty());
}

#[test]
fn ownership_activity_follows_identity() {
    let blames = vec![make_blame("Alice", "Alice@X.com", 90, 100)];
    let recent = HashSet::from(["alice@x.com".to_string()]);
    let result = compute_ownership(PathBuf::from("a.rs"), "Rust", &blames, &recent);
    assert!(!result.knowledge_loss);
}

#[test]
fn summary_groups_owners_by_identity() {
    let owned = |name: &str, email: &str| {
        compute_ownership(
            PathBuf::from("a.rs"),
            "Rust",
            &[make_blame(name, email, 10, 100)],
            &HashSet::new(),
        )
    };
    let summary = aggregate_by_author(&[
        owned("Victor Z", "v@x.org"),
        owned("vitaut", "V@x.org"),
        owned("Kengo Watanabe", "kengo@a.org"),
        owned("Kengo Watanabe", "kengo@b.org"),
    ]);
    assert_eq!(summary.len(), 3);
    assert!(
        summary
            .iter()
            .any(|a| a.author == "Victor Z" && a.files_owned == 2)
    );
}
