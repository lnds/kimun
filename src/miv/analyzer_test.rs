use super::*;

/// Hand-computed reference: V=100, G=5, LOC=50, comments=10
///   comment_percent = 10/60 * 100 = 16.6667%
///   MIwoc = 171 - 5.2*ln(100) - 0.23*5 - 16.2*ln(50)
///         = 171 - 23.947 - 1.15 - 63.375 = 82.528
///   MIcw  = 50 * sin(sqrt(2.46 * radians(16.6667)))
///         = 50 * sin(sqrt(2.46 * 0.29088))
///         = 50 * sin(sqrt(0.71558))
///         = 50 * sin(0.84593)
///         = 50 * 0.74855 = 37.427
///   MI    = 82.528 + 37.427 = 119.955
#[test]
fn reference_hand_computed() {
    let m = compute_mi(100.0, 5, 50, 10).unwrap();

    assert!(
        (m.comment_percent - 16.667).abs() < 0.1,
        "comment_percent should be ~16.667, got {}",
        m.comment_percent
    );
    assert!(
        (m.mi_woc - 82.528).abs() < 0.1,
        "MIwoc should be ~82.528, got {}",
        m.mi_woc
    );
    assert!(
        m.mi_cw > 0.0,
        "MIcw should be positive with comments, got {}",
        m.mi_cw
    );
    assert!(
        (m.mi_cw - 37.4).abs() < 0.5,
        "MIcw should be ~37.4, got {}",
        m.mi_cw
    );
    assert!(
        (m.mi_score - 119.9).abs() < 1.0,
        "MI score should be ~119.9, got {}",
        m.mi_score
    );
    assert_eq!(m.loc, 50);
    assert_eq!(m.comment_lines, 10);
    assert_eq!(m.level, MILevel::Good);
}

/// Reference: no comments → MIcw = 50*sin(0) = 0, MI = MIwoc
#[test]
fn reference_no_comments() {
    let m = compute_mi(100.0, 5, 50, 0).unwrap();

    // MIwoc = 171 - 23.947 - 1.15 - 63.375 = 82.528
    assert!(
        (m.mi_woc - 82.528).abs() < 0.1,
        "MIwoc should be ~82.528, got {}",
        m.mi_woc
    );
    assert!(
        m.mi_cw.abs() < f64::EPSILON,
        "MIcw should be 0 with no comments, got {}",
        m.mi_cw
    );
    assert!(
        (m.mi_score - m.mi_woc).abs() < f64::EPSILON,
        "MI should equal MIwoc when no comments"
    );
}

#[test]
fn zero_code_returns_none() {
    assert!(
        compute_mi(100.0, 5, 0, 10).is_none(),
        "code_lines=0 should return None"
    );
}

#[test]
fn zero_volume_returns_none() {
    assert!(
        compute_mi(0.0, 5, 50, 10).is_none(),
        "volume=0 should return None"
    );
}

#[test]
fn negative_volume_returns_none() {
    assert!(
        compute_mi(-1.0, 5, 50, 10).is_none(),
        "negative volume should return None"
    );
}

#[test]
fn zero_complexity_returns_none() {
    assert!(
        compute_mi(100.0, 0, 50, 10).is_none(),
        "complexity=0 should return None"
    );
}

#[test]
fn level_good() {
    let m = compute_mi(10.0, 1, 5, 2).unwrap();
    assert!(
        m.mi_score >= 85.0,
        "MI should be >= 85 for small/simple code, got {}",
        m.mi_score
    );
    assert_eq!(m.level, MILevel::Good);
}

#[test]
fn level_difficult_extreme() {
    let m = compute_mi(50000.0, 100, 2000, 0).unwrap();
    assert!(
        m.mi_score < 0.0,
        "MI should be negative for extreme code, got {}",
        m.mi_score
    );
    assert_eq!(m.level, MILevel::Difficult);
}

#[test]
fn extremely_high_complexity() {
    let m = compute_mi(10000.0, 500, 1000, 0).unwrap();
    assert!(
        m.mi_score.is_finite(),
        "MI should be finite even with G=500, got {}",
        m.mi_score
    );
    assert_eq!(m.level, MILevel::Difficult, "G=500 should be Difficult");
}

#[test]
fn extremely_large_volume() {
    let m = compute_mi(1_000_000.0, 10, 500, 50).unwrap();
    assert!(
        m.mi_score.is_finite(),
        "MI should be finite even with V=1M, got {}",
        m.mi_score
    );
    assert_eq!(m.level, MILevel::Difficult, "V=1M should be Difficult");
}

#[test]
fn negative_mi_score() {
    // V=50000, G=100, LOC=2000 → MIwoc ≈ -31.17, MIcw=0 → MI ≈ -31
    let m = compute_mi(50000.0, 100, 2000, 0).unwrap();
    assert!(
        m.mi_score < 0.0,
        "pathological code can produce negative MI, got {}",
        m.mi_score
    );
    assert_eq!(m.level, MILevel::Difficult);
}

#[test]
fn comments_always_boost_score() {
    let without = compute_mi(100.0, 5, 50, 0).unwrap();
    // Test with various comment levels — all should boost
    for comment_lines in [5, 10, 20, 40] {
        let with = compute_mi(100.0, 5, 50, comment_lines).unwrap();
        assert!(
            with.mi_score > without.mi_score,
            "comments ({comment_lines} lines) should boost MI: with={} without={}",
            with.mi_score,
            without.mi_score
        );
        assert!(
            with.mi_cw > 0.0,
            "MIcw should always be positive, got {} for {comment_lines} comment lines",
            with.mi_cw
        );
    }
}

#[test]
fn no_comments_zero_percent() {
    let m = compute_mi(100.0, 5, 50, 0).unwrap();
    assert!(
        m.comment_percent.abs() < f64::EPSILON,
        "comment_percent should be 0, got {}",
        m.comment_percent
    );
}

#[test]
fn level_boundaries() {
    assert_eq!(
        MILevel::from_score(85.0),
        MILevel::Good,
        "85.0 should be Good"
    );
    assert_eq!(
        MILevel::from_score(84.9),
        MILevel::Moderate,
        "84.9 should be Moderate"
    );
    assert_eq!(
        MILevel::from_score(65.0),
        MILevel::Moderate,
        "65.0 should be Moderate"
    );
    assert_eq!(
        MILevel::from_score(64.9),
        MILevel::Difficult,
        "64.9 should be Difficult"
    );
}
