use super::*;

// ─── Legacy model: normalize_mi ──────────────────────────────────────

#[test]
fn test_normalize_mi_0() {
    let s = normalize_mi(0.0);
    assert!((s - 0.0).abs() < 0.01, "mi=0 -> 0, got {s}");
}

#[test]
fn test_normalize_mi_25() {
    let s = normalize_mi(25.0);
    assert!((s - 20.0).abs() < 0.01, "mi=25 -> 20, got {s}");
}

#[test]
fn test_normalize_mi_50() {
    let s = normalize_mi(50.0);
    assert!((s - 60.0).abs() < 0.01, "mi=50 -> 60, got {s}");
}

#[test]
fn test_normalize_mi_75() {
    let s = normalize_mi(75.0);
    assert!((s - 90.0).abs() < 0.01, "mi=75 -> 90, got {s}");
}

#[test]
fn test_normalize_mi_100() {
    let s = normalize_mi(100.0);
    assert!((s - 100.0).abs() < 0.01, "mi=100 -> 100, got {s}");
}

#[test]
fn test_normalize_mi_monotonic() {
    let values = [0.0, 10.0, 25.0, 40.0, 50.0, 65.0, 75.0, 85.0, 100.0];
    for window in values.windows(2) {
        let lo = normalize_mi(window[0]);
        let hi = normalize_mi(window[1]);
        assert!(
            hi >= lo,
            "normalize_mi should be monotonically increasing: mi={} -> {lo}, mi={} -> {hi}",
            window[0],
            window[1]
        );
    }
}

// ─── Legacy model: normalize_complexity ──────────────────────────────

#[test]
fn test_normalize_complexity_5() {
    let s = normalize_complexity(5);
    assert!((s - 100.0).abs() < 0.01, "cycom=5 -> 100, got {s}");
}

#[test]
fn test_normalize_complexity_10() {
    let s = normalize_complexity(10);
    assert!((s - 85.0).abs() < 0.01, "cycom=10 -> 85, got {s}");
}

#[test]
fn test_normalize_complexity_15() {
    let s = normalize_complexity(15);
    assert!((s - 65.0).abs() < 0.01, "cycom=15 -> 65, got {s}");
}

#[test]
fn test_normalize_complexity_25() {
    let s = normalize_complexity(25);
    assert!((s - 35.0).abs() < 0.01, "cycom=25 -> 35, got {s}");
}

#[test]
fn test_normalize_complexity_50() {
    let s = normalize_complexity(50);
    assert!((s - 5.0).abs() < 0.01, "cycom=50 -> 5, got {s}");
}

#[test]
fn test_normalize_complexity_100() {
    let s = normalize_complexity(100);
    assert!((s - 0.0).abs() < 0.01, "cycom=100 -> 0, got {s}");
}

#[test]
fn test_normalize_complexity_monotonic() {
    let values = [0, 2, 5, 7, 10, 12, 15, 20, 25, 40, 50, 75, 100];
    for window in values.windows(2) {
        let lo = normalize_complexity(window[0]);
        let hi = normalize_complexity(window[1]);
        assert!(
            lo >= hi,
            "normalize_complexity should be monotonically decreasing: cycom={} -> {lo}, cycom={} -> {hi}",
            window[0],
            window[1]
        );
    }
}

// ─── Cognitive model: normalize_cognitive ────────────────────────────

#[test]
fn test_normalize_cognitive_0() {
    assert!((normalize_cognitive(0) - 100.0).abs() < 0.01);
}

#[test]
fn test_normalize_cognitive_4() {
    assert!((normalize_cognitive(4) - 100.0).abs() < 0.01);
}

#[test]
fn test_normalize_cognitive_9() {
    let s = normalize_cognitive(9);
    assert!((s - 85.0).abs() < 0.01, "cognitive=9 -> 85, got {s}");
}

#[test]
fn test_normalize_cognitive_14() {
    let s = normalize_cognitive(14);
    assert!((s - 65.0).abs() < 0.01, "cognitive=14 -> 65, got {s}");
}

#[test]
fn test_normalize_cognitive_24() {
    let s = normalize_cognitive(24);
    assert!((s - 35.0).abs() < 0.01, "cognitive=24 -> 35, got {s}");
}

#[test]
fn test_normalize_cognitive_50() {
    let s = normalize_cognitive(50);
    assert!((s - 5.0).abs() < 0.01, "cognitive=50 -> 5, got {s}");
}

#[test]
fn test_normalize_cognitive_100() {
    assert!((normalize_cognitive(100) - 0.0).abs() < 0.01);
}

#[test]
fn test_normalize_cognitive_monotonic() {
    let values = [0, 2, 4, 7, 9, 12, 14, 20, 24, 40, 50, 75, 100];
    for window in values.windows(2) {
        let lo = normalize_cognitive(window[0]);
        let hi = normalize_cognitive(window[1]);
        assert!(
            lo >= hi,
            "normalize_cognitive should be monotonically decreasing: cogcom={} -> {lo}, cogcom={} -> {hi}",
            window[0],
            window[1]
        );
    }
}

#[test]
fn test_normalize_duplication_0() {
    assert!((normalize_duplication(0.0) - 100.0).abs() < 0.01);
}

#[test]
fn test_normalize_duplication_5() {
    assert!((normalize_duplication(5.0) - 100.0).abs() < 0.01);
}

#[test]
fn test_normalize_duplication_10() {
    assert!((normalize_duplication(10.0) - 80.0).abs() < 0.01);
}

#[test]
fn test_normalize_duplication_20() {
    assert!((normalize_duplication(20.0) - 40.0).abs() < 0.01);
}

#[test]
fn test_normalize_duplication_40() {
    assert!((normalize_duplication(40.0) - 10.0).abs() < 0.01);
}

#[test]
fn test_normalize_duplication_70() {
    let s = normalize_duplication(70.0);
    assert!(s >= 0.0 && s < 10.0, "dup=70% should be near 0, got {s}");
}

#[test]
fn test_normalize_duplication_100() {
    assert!((normalize_duplication(100.0) - 0.0).abs() < 0.01);
}

#[test]
fn test_normalize_indent_low() {
    assert!((normalize_indent(0.5) - 100.0).abs() < 0.01);
}

#[test]
fn test_normalize_indent_1() {
    assert!((normalize_indent(1.0) - 100.0).abs() < 0.01);
}

#[test]
fn test_normalize_indent_1_5() {
    assert!((normalize_indent(1.5) - 80.0).abs() < 0.01);
}

#[test]
fn test_normalize_indent_2() {
    assert!((normalize_indent(2.0) - 50.0).abs() < 0.01);
}

#[test]
fn test_normalize_indent_3() {
    assert!((normalize_indent(3.0) - 20.0).abs() < 0.01);
}

#[test]
fn test_normalize_halstead_low_effort_per_loc() {
    assert!((normalize_halstead(100.0, 100) - 100.0).abs() < 0.01);
}

#[test]
fn test_normalize_halstead_1000_epl() {
    assert!((normalize_halstead(100000.0, 100) - 100.0).abs() < 0.01);
}

#[test]
fn test_normalize_halstead_5000_epl() {
    let s = normalize_halstead(500000.0, 100);
    assert!((s - 70.0).abs() < 0.01, "epl=5000 -> 70, got {s}");
}

#[test]
fn test_normalize_halstead_10000_epl() {
    let s = normalize_halstead(1000000.0, 100);
    assert!((s - 40.0).abs() < 0.01, "epl=10000 -> 40, got {s}");
}

#[test]
fn test_normalize_halstead_20000_epl() {
    assert!((normalize_halstead(2000000.0, 100) - 0.0).abs() < 0.01);
}

#[test]
fn test_normalize_halstead_zero_loc() {
    assert!((normalize_halstead(1000.0, 0) - 50.0).abs() < 0.01);
}

#[test]
fn test_normalize_halstead_zero_effort() {
    assert!((normalize_halstead(0.0, 100) - 50.0).abs() < 0.01);
}

#[test]
fn test_normalize_halstead_boundary_continuity() {
    let at_1000 = normalize_halstead(100000.0, 100);
    let above_1000 = normalize_halstead(100100.0, 100);
    assert!(
        above_1000 < at_1000,
        "epl=1001 should score lower than epl=1000"
    );

    let at_5000 = normalize_halstead(500000.0, 100);
    let above_5000 = normalize_halstead(500100.0, 100);
    assert!(
        above_5000 < at_5000,
        "epl=5001 should score lower than epl=5000"
    );
}

#[test]
fn test_normalize_file_size_small() {
    assert!((normalize_file_size(10) - 100.0).abs() < 0.01);
    assert!((normalize_file_size(50) - 100.0).abs() < 0.01);
}

#[test]
fn test_normalize_file_size_optimal() {
    assert!((normalize_file_size(300) - 100.0).abs() < 0.01);
    assert!((normalize_file_size(500) - 100.0).abs() < 0.01);
}

#[test]
fn test_normalize_file_size_1000() {
    let s = normalize_file_size(1000);
    assert!((s - 60.0).abs() < 0.01, "1000 LOC -> 60, got {s}");
}

#[test]
fn test_normalize_file_size_2000() {
    let s = normalize_file_size(2000);
    assert!((s - 20.0).abs() < 0.01, "2000 LOC -> 20, got {s}");
}
