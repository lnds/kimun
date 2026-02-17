use super::*;

#[test]
fn indent_depth_spaces() {
    // 4 spaces = 1 logical level, 8 spaces = 2 logical levels
    assert_eq!(indent_depth("    code", 4), 1);
    assert_eq!(indent_depth("        code", 4), 2);
}

#[test]
fn indent_depth_tabs() {
    // 1 tab = 1 logical level, 2 tabs = 2 logical levels
    assert_eq!(indent_depth("\tcode", 4), 1);
    assert_eq!(indent_depth("\t\tcode", 4), 2);
}

#[test]
fn indent_depth_mixed() {
    // 1 tab + 2 spaces = 6 raw spaces / 4 = 1 logical level (integer division)
    assert_eq!(indent_depth("\t  code", 4), 1);
}

#[test]
fn indent_depth_no_indent() {
    assert_eq!(indent_depth("code", 4), 0);
}

#[test]
fn indent_depth_empty_line() {
    assert_eq!(indent_depth("", 4), 0);
}

#[test]
fn indent_depth_partial_indent() {
    // 2 spaces = 0 logical levels (less than one tab_width)
    assert_eq!(indent_depth("  code", 4), 0);
    // 6 spaces = 1 logical level
    assert_eq!(indent_depth("      code", 4), 1);
}

#[test]
fn analyze_basic_file() {
    let lines: Vec<String> = vec![
        "fn main() {",
        "    let x = 1;",
        "    if x > 0 {",
        "        println!(\"hi\");",
        "    }",
        "}",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let kinds = vec![LineKind::Code; 6];

    let m = analyze(&lines, &kinds, 4).unwrap();
    // logical depths = [0, 1, 1, 2, 1, 0] → mean=0.83, max=2
    assert_eq!(m.code_lines, 6);
    assert_eq!(m.max_depth, 2);
    assert!(m.stddev > 0.0);
}

#[test]
fn analyze_filters_non_code() {
    let lines: Vec<String> = vec!["// comment", "", "fn main() {", "    let x = 1;", "}"]
        .into_iter()
        .map(String::from)
        .collect();
    let kinds = vec![
        LineKind::Comment,
        LineKind::Blank,
        LineKind::Code,
        LineKind::Code,
        LineKind::Code,
    ];

    let m = analyze(&lines, &kinds, 4).unwrap();
    assert_eq!(m.code_lines, 3);
}

#[test]
fn analyze_empty_returns_none() {
    let m = analyze(&[], &[], 4);
    assert!(m.is_none());
}

#[test]
fn analyze_all_comments_returns_none() {
    let lines = vec!["// comment".to_string()];
    let kinds = vec![LineKind::Comment];
    assert!(analyze(&lines, &kinds, 4).is_none());
}

#[test]
fn analyze_uniform_indent_zero_stddev() {
    let lines: Vec<String> = vec!["    a();", "    b();", "    c();"]
        .into_iter()
        .map(String::from)
        .collect();
    let kinds = vec![LineKind::Code; 3];

    let m = analyze(&lines, &kinds, 4).unwrap();
    assert!((m.stddev - 0.0).abs() < 0.001);
    assert_eq!(m.max_depth, 1); // 4 spaces = 1 logical level
}

#[test]
fn stddev_calculation() {
    // logical levels [0, 4, 8] → mean=4, variance=((16+0+16)/2)=16, sd=4.0 (Bessel's correction)
    let m = calculate_stddev(&[0, 4, 8]);
    assert!((m - 4.0).abs() < 0.01);
}

#[test]
fn stddev_single_value_is_zero() {
    assert_eq!(calculate_stddev(&[5]), 0.0);
}

#[test]
fn stddev_empty_is_zero() {
    assert_eq!(calculate_stddev(&[]), 0.0);
}

#[test]
fn complexity_level_thresholds() {
    assert_eq!(ComplexityLevel::from_stddev(0.0), ComplexityLevel::Low);
    assert_eq!(ComplexityLevel::from_stddev(0.99), ComplexityLevel::Low);
    assert_eq!(ComplexityLevel::from_stddev(1.0), ComplexityLevel::Moderate);
    assert_eq!(
        ComplexityLevel::from_stddev(1.49),
        ComplexityLevel::Moderate
    );
    assert_eq!(ComplexityLevel::from_stddev(1.5), ComplexityLevel::High);
    assert_eq!(ComplexityLevel::from_stddev(1.99), ComplexityLevel::High);
    assert_eq!(ComplexityLevel::from_stddev(2.0), ComplexityLevel::VeryHigh);
    assert_eq!(ComplexityLevel::from_stddev(5.0), ComplexityLevel::VeryHigh);
}

#[test]
fn complexity_level_display() {
    assert_eq!(ComplexityLevel::Low.as_str(), "Low");
    assert_eq!(ComplexityLevel::Moderate.as_str(), "Moderate");
    assert_eq!(ComplexityLevel::High.as_str(), "High");
    assert_eq!(ComplexityLevel::VeryHigh.as_str(), "Very High");
}

#[test]
fn complexity_level_serde() {
    assert_eq!(
        serde_json::to_string(&ComplexityLevel::VeryHigh).unwrap(),
        "\"very_high\""
    );
    assert_eq!(
        serde_json::to_string(&ComplexityLevel::Low).unwrap(),
        "\"low\""
    );
}

#[test]
fn analyze_includes_complexity() {
    let lines: Vec<String> = vec!["    a();", "    b();", "    c();"]
        .into_iter()
        .map(String::from)
        .collect();
    let kinds = vec![LineKind::Code; 3];
    let m = analyze(&lines, &kinds, 4).unwrap();
    assert_eq!(m.complexity, ComplexityLevel::Low);
}
