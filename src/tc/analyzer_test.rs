use super::*;

fn path(s: &str) -> PathBuf {
    PathBuf::from(s)
}

fn freq(pairs: &[(&str, usize)]) -> HashMap<PathBuf, usize> {
    pairs.iter().map(|(p, c)| (path(p), *c)).collect()
}

#[test]
fn test_single_commit_two_files() {
    let co = vec![vec![path("a.rs"), path("b.rs")]];
    let fm = freq(&[("a.rs", 5), ("b.rs", 5)]);
    let result = compute_coupling(&co, &fm, 1);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].shared_commits, 1);
}

#[test]
fn test_multiple_shared_commits() {
    let co = vec![
        vec![path("a.rs"), path("b.rs")],
        vec![path("a.rs"), path("b.rs")],
        vec![path("a.rs"), path("b.rs")],
    ];
    let fm = freq(&[("a.rs", 5), ("b.rs", 3)]);
    let result = compute_coupling(&co, &fm, 1);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].shared_commits, 3);
    // strength = 3 / min(5, 3) = 3/3 = 1.0
    assert!((result[0].strength - 1.0).abs() < 0.001);
}

#[test]
fn test_no_coupling() {
    let co = vec![vec![path("a.rs")], vec![path("b.rs")]];
    let fm = freq(&[("a.rs", 3), ("b.rs", 3)]);
    let result = compute_coupling(&co, &fm, 1);
    assert!(result.is_empty());
}

#[test]
fn test_min_degree_filters() {
    let co = vec![vec![path("a.rs"), path("b.rs")]];
    let fm = freq(&[("a.rs", 5), ("b.rs", 2)]);
    // b.rs has only 2 commits, min_degree=3 filters it out
    let result = compute_coupling(&co, &fm, 3);
    assert!(result.is_empty());
}

#[test]
fn test_pair_ordering() {
    // (B, A) order in commit should still produce the same pair as (A, B)
    let co = vec![
        vec![path("b.rs"), path("a.rs")],
        vec![path("a.rs"), path("b.rs")],
    ];
    let fm = freq(&[("a.rs", 5), ("b.rs", 5)]);
    let result = compute_coupling(&co, &fm, 1);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].shared_commits, 2);
}

#[test]
fn test_level_thresholds() {
    assert_eq!(classify_level(0.5), CouplingLevel::Strong);
    assert_eq!(classify_level(0.8), CouplingLevel::Strong);
    assert_eq!(classify_level(0.3), CouplingLevel::Moderate);
    assert_eq!(classify_level(0.49), CouplingLevel::Moderate);
    assert_eq!(classify_level(0.29), CouplingLevel::Weak);
    assert_eq!(classify_level(0.0), CouplingLevel::Weak);
}

#[test]
fn test_three_files_in_commit() {
    let co = vec![vec![path("a.rs"), path("b.rs"), path("c.rs")]];
    let fm = freq(&[("a.rs", 5), ("b.rs", 5), ("c.rs", 5)]);
    let result = compute_coupling(&co, &fm, 1);
    // 3 files → 3 pairs: (a,b), (a,c), (b,c)
    assert_eq!(result.len(), 3);
}

#[test]
fn test_strength_calculation() {
    let co = vec![
        vec![path("a.rs"), path("b.rs")],
        vec![path("a.rs"), path("b.rs")],
    ];
    // a has 10 commits, b has 4 → strength = 2 / min(10, 4) = 2/4 = 0.5
    let fm = freq(&[("a.rs", 10), ("b.rs", 4)]);
    let result = compute_coupling(&co, &fm, 1);
    assert_eq!(result.len(), 1);
    assert!((result[0].strength - 0.5).abs() < 0.001);
    assert_eq!(result[0].level, CouplingLevel::Strong);
}

#[test]
fn test_empty_co_changes() {
    let co: Vec<Vec<PathBuf>> = vec![];
    let fm = freq(&[("a.rs", 5)]);
    let result = compute_coupling(&co, &fm, 1);
    assert!(result.is_empty());
}

#[test]
fn test_empty_freq_map() {
    let co = vec![vec![path("a.rs"), path("b.rs")]];
    let fm: HashMap<PathBuf, usize> = HashMap::new();
    let result = compute_coupling(&co, &fm, 1);
    assert!(result.is_empty());
}

#[test]
fn test_sorted_by_strength_desc() {
    let co = vec![
        vec![path("a.rs"), path("b.rs")],
        vec![path("c.rs"), path("d.rs")],
        vec![path("c.rs"), path("d.rs")],
        vec![path("c.rs"), path("d.rs")],
    ];
    let fm = freq(&[("a.rs", 5), ("b.rs", 5), ("c.rs", 3), ("d.rs", 3)]);
    let result = compute_coupling(&co, &fm, 1);
    assert_eq!(result.len(), 2);
    // c-d: 3/3 = 1.0, a-b: 1/5 = 0.2
    assert!(result[0].strength > result[1].strength);
}
