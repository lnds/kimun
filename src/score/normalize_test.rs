use super::*;

#[test]
fn test_normalize_mi_pathological() {
    let s = normalize_mi(-100.0);
    assert!((s - 0.0).abs() < 0.01, "MI=-100 should score 0, got {s}");
}

#[test]
fn test_normalize_mi_zero() {
    let s = normalize_mi(0.0);
    assert!((s - 30.0).abs() < 0.01, "MI=0 should score 30, got {s}");
}

#[test]
fn test_normalize_mi_very_difficult() {
    let s = normalize_mi(20.0);
    assert!((s - 40.0).abs() < 0.01, "MI=20 should score 40, got {s}");
}

#[test]
fn test_normalize_mi_difficult() {
    let s = normalize_mi(40.0);
    assert!((s - 50.0).abs() < 0.01, "MI=40 should score 50, got {s}");
}

#[test]
fn test_normalize_mi_moderate_boundary() {
    let s = normalize_mi(65.0);
    assert!((s - 70.0).abs() < 0.01, "MI=65 should score 70, got {s}");
}

#[test]
fn test_normalize_mi_good_boundary() {
    let s = normalize_mi(85.0);
    assert!((s - 90.0).abs() < 0.01, "MI=85 should score 90, got {s}");
}

#[test]
fn test_normalize_mi_max() {
    let s = normalize_mi(171.0);
    assert!((s - 100.0).abs() < 0.01, "MI=171 should score 100, got {s}");
}

#[test]
fn test_normalize_mi_below_min() {
    let s = normalize_mi(-200.0);
    assert!((s - 0.0).abs() < 0.01, "MI=-200 should clamp to 0, got {s}");
}

#[test]
fn test_normalize_mi_above_max() {
    let s = normalize_mi(200.0);
    assert!(
        (s - 100.0).abs() < 0.01,
        "MI=200 should clamp to 100, got {s}"
    );
}

#[test]
fn test_normalize_mi_monotonic() {
    let values = [
        -100.0, -50.0, 0.0, 20.0, 40.0, 65.0, 75.0, 85.0, 120.0, 171.0,
    ];
    for window in values.windows(2) {
        let lo = normalize_mi(window[0]);
        let hi = normalize_mi(window[1]);
        assert!(
            hi >= lo,
            "normalize_mi should be monotonic: MI={} -> {lo}, MI={} -> {hi}",
            window[0],
            window[1]
        );
    }
}

#[test]
fn test_normalize_complexity_1() {
    assert!((normalize_complexity(1) - 100.0).abs() < 0.01);
}

#[test]
fn test_normalize_complexity_5() {
    assert!((normalize_complexity(5) - 100.0).abs() < 0.01);
}

#[test]
fn test_normalize_complexity_10() {
    let s = normalize_complexity(10);
    assert!((s - 80.0).abs() < 0.01, "complexity=10 -> 80, got {s}");
}

#[test]
fn test_normalize_complexity_20() {
    let s = normalize_complexity(20);
    assert!((s - 50.0).abs() < 0.01, "complexity=20 -> 50, got {s}");
}

#[test]
fn test_normalize_complexity_50() {
    let s = normalize_complexity(50);
    assert!((s - 5.0).abs() < 0.01, "complexity=50 -> 5, got {s}");
}

#[test]
fn test_normalize_complexity_100() {
    assert!((normalize_complexity(100) - 0.0).abs() < 0.01);
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
