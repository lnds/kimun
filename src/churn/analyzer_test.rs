use super::*;
use std::path::PathBuf;

const DAY: i64 = 86_400;
const MONTH: i64 = 30 * DAY;

fn path() -> PathBuf {
    PathBuf::from("src/foo.rs")
}

#[test]
fn single_commit_zero_span_clamps_to_medium() {
    // span = 0 → months clamped to 1.0 → rate = 1/1 = 1.0 → Medium
    let f = classify(path(), "Rust", 1, 0, 0);
    assert_eq!(f.level, ChurnLevel::Medium);
    assert!((f.rate - 1.0).abs() < 1e-9);
}

#[test]
fn two_commits_over_three_months_is_low() {
    let f = classify(path(), "Rust", 2, 0, 3 * MONTH);
    // rate = 2/3 ≈ 0.67 → Low
    assert_eq!(f.level, ChurnLevel::Low);
    assert!(f.rate < 1.0);
}

#[test]
fn four_commits_per_month_is_medium() {
    let f = classify(path(), "Rust", 4, 0, MONTH);
    // rate = 4/1 = 4.0 → Medium (not > 4)
    assert_eq!(f.level, ChurnLevel::Medium);
}

#[test]
fn five_commits_per_month_is_high() {
    let f = classify(path(), "Rust", 5, 0, MONTH);
    // rate = 5/1 = 5.0 > 4 → High
    assert_eq!(f.level, ChurnLevel::High);
}

#[test]
fn many_commits_over_long_period_can_be_low() {
    // 12 commits over 2 years = 0.5/month → Low
    let f = classify(path(), "Rust", 12, 0, 24 * MONTH);
    assert_eq!(f.level, ChurnLevel::Low);
}

#[test]
fn level_labels() {
    assert_eq!(ChurnLevel::Low.label(), "LOW");
    assert_eq!(ChurnLevel::Medium.label(), "MEDIUM");
    assert_eq!(ChurnLevel::High.label(), "HIGH");
}

#[test]
fn fields_are_stored() {
    let first = 1_000_000i64;
    let last = first + 6 * MONTH;
    let f = classify(PathBuf::from("a/b.py"), "Python", 10, first, last);
    assert_eq!(f.path, PathBuf::from("a/b.py"));
    assert_eq!(f.language, "Python");
    assert_eq!(f.commits, 10);
    assert_eq!(f.first_commit, first);
    assert_eq!(f.last_commit, last);
}
