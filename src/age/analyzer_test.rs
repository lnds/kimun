use super::*;
use std::path::PathBuf;

const NOW: i64 = 1_000_000_000;
const DAY: i64 = 86_400;

fn thresholds() -> AgeThresholds {
    AgeThresholds {
        active_days: 90,
        frozen_days: 365,
    }
}

#[test]
fn recent_file_is_active() {
    let f = classify(
        PathBuf::from("a.rs"),
        "Rust",
        NOW - 30 * DAY,
        NOW,
        &thresholds(),
    );
    assert_eq!(f.status, AgeStatus::Active);
}

#[test]
fn boundary_active_stale() {
    // exactly at active_days boundary → Stale
    let f = classify(
        PathBuf::from("a.rs"),
        "Rust",
        NOW - 90 * DAY,
        NOW,
        &thresholds(),
    );
    assert_eq!(f.status, AgeStatus::Stale);
}

#[test]
fn mid_range_is_stale() {
    let f = classify(
        PathBuf::from("a.rs"),
        "Rust",
        NOW - 180 * DAY,
        NOW,
        &thresholds(),
    );
    assert_eq!(f.status, AgeStatus::Stale);
}

#[test]
fn boundary_stale_frozen() {
    // exactly at frozen_days boundary → Frozen
    let f = classify(
        PathBuf::from("a.rs"),
        "Rust",
        NOW - 365 * DAY,
        NOW,
        &thresholds(),
    );
    assert_eq!(f.status, AgeStatus::Frozen);
}

#[test]
fn old_file_is_frozen() {
    let f = classify(
        PathBuf::from("a.rs"),
        "Rust",
        NOW - 500 * DAY,
        NOW,
        &thresholds(),
    );
    assert_eq!(f.status, AgeStatus::Frozen);
}

#[test]
fn custom_thresholds() {
    let t = AgeThresholds {
        active_days: 7,
        frozen_days: 30,
    };
    let active = classify(PathBuf::from("a.rs"), "Rust", NOW - 3 * DAY, NOW, &t);
    let stale = classify(PathBuf::from("b.rs"), "Rust", NOW - 14 * DAY, NOW, &t);
    let frozen = classify(PathBuf::from("c.rs"), "Rust", NOW - 60 * DAY, NOW, &t);
    assert_eq!(active.status, AgeStatus::Active);
    assert_eq!(stale.status, AgeStatus::Stale);
    assert_eq!(frozen.status, AgeStatus::Frozen);
}

#[test]
fn labels() {
    assert_eq!(AgeStatus::Active.label(), "ACTIVE");
    assert_eq!(AgeStatus::Stale.label(), "STALE");
    assert_eq!(AgeStatus::Frozen.label(), "FROZEN");
}
