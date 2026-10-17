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
    let lines: Vec<String> = vec!["    a();"; MIN_GRADED_LINES]
        .into_iter()
        .map(String::from)
        .collect();
    let kinds = vec![LineKind::Code; lines.len()];
    let m = analyze(&lines, &kinds, 4).unwrap();
    assert_eq!(m.complexity, ComplexityLevel::Low);
}

#[test]
fn analyze_leaves_small_files_ungraded() {
    let lines: Vec<String> = vec!["a();", "        b();", "c();"]
        .into_iter()
        .map(String::from)
        .collect();
    let kinds = vec![LineKind::Code; 3];
    let m = analyze(&lines, &kinds, 4).unwrap();
    assert_eq!(m.complexity, ComplexityLevel::Ungraded);
    assert_eq!(m.complexity.as_str(), "-");
    assert_eq!(
        serde_json::to_string(&ComplexityLevel::Ungraded).unwrap(),
        "\"ungraded\""
    );
}

fn depths(code: &str) -> Vec<usize> {
    let lines: Vec<&str> = code.lines().collect();
    structural_depths(&lines, 4)
}

#[test]
fn aligned_arguments_are_one_level_deeper_than_their_call() {
    let code = "\
int load(const char *path) {
    if (!f) {
        return report(\"Could not open %s (flags %d)\",
                                          path,
                                          flags);
    }
    return 0;
}";
    assert_eq!(depths(code), [0, 1, 2, 3, 3, 1, 1, 0]);
}

#[test]
fn block_style_parameters_keep_their_depth() {
    let code = "\
fn f(
    a: u32,
    b: u32,
) -> u32 {
    if a > b {
        a
    } else {
        b
    }
}";
    assert_eq!(depths(code), [0, 1, 1, 0, 1, 2, 1, 2, 1, 0]);
}

#[test]
fn a_closure_inside_a_call_nests_again() {
    let code = "\
describe('x', () => {
    it('y', () => {
        if (a) {
            b();
        }
    });
});";
    assert_eq!(depths(code), [0, 1, 2, 3, 2, 1, 0]);
}

#[test]
fn elixir_fn_inside_a_call_nests_again() {
    let code = "\
Enum.map(list, fn x ->
    case x do
        1 -> :one
        _ -> :other
    end
end)";
    assert_eq!(depths(code), [0, 1, 2, 2, 1, 0]);
}

#[test]
fn lines_after_or_starting_with_an_operator_continue_the_statement() {
    let code = "\
defmodule Serializer do
  @callback encode!(term()) ::
              {:push, :text, iodata()}
              | {:push, :binary, iodata()}
end";
    // Two spaces are no level with a width of 4; the continuation lines
    // are one level below the `@callback`, not three.
    assert_eq!(depths(code), [0, 0, 1, 1, 0]);
}

#[test]
fn method_chains_continue_the_statement() {
    let code = "\
let v = items
                .iter()
                .map(f)
                .collect();
let w = 1;";
    assert_eq!(depths(code), [0, 1, 1, 1, 0]);
}

#[test]
fn brackets_in_literals_and_comments_count_for_nothing() {
    let code = "\
fn f() {
    let s = \"(((\"; // ((
    let c = '(';
    g();
}";
    assert_eq!(depths(code), [0, 1, 1, 1, 0]);
}
