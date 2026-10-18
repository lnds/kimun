use super::*;
use std::path::PathBuf;

#[test]
fn max_path_width_with_paths() {
    let paths = vec![
        PathBuf::from("src/foo.rs"),
        PathBuf::from("src/very_long_name.rs"),
    ];
    let w = max_path_width(paths.iter().map(|p| p.as_path()), 4);
    assert_eq!(w, "src/very_long_name.rs".len());
}

#[test]
fn max_path_width_empty() {
    let paths: Vec<PathBuf> = vec![];
    let w = max_path_width(paths.iter().map(|p| p.as_path()), 4);
    assert_eq!(w, 4);
}

#[test]
fn max_path_width_min_enforced() {
    let paths = vec![PathBuf::from("a")];
    let w = max_path_width(paths.iter().map(|p| p.as_path()), 10);
    assert_eq!(w, 10);
}

#[test]
fn separator_width() {
    let s = separator(5);
    // Each ─ is 3 bytes in UTF-8
    assert_eq!(s.chars().count(), 5);
}

#[test]
fn print_json_stdout_works() {
    let data = vec![1, 2, 3];
    print_json_stdout(&data).unwrap();
}

struct Row(&'static str, usize);

impl PerFunctionRow for Row {
    fn name(&self) -> &str {
        self.0
    }
    fn start_line(&self) -> usize {
        1
    }
    fn complexity(&self) -> usize {
        self.1
    }
    fn level_str(&self) -> &str {
        ""
    }
}

#[test]
fn rows_at_least_keeps_only_rows_meeting_threshold() {
    let rows = [Row("trivial", 0), Row("medium", 6), Row("nasty", 23)];
    let names: Vec<&str> = rows_at_least(&rows, 6).iter().map(|r| r.name()).collect();
    assert_eq!(names, ["medium", "nasty"]);
}

#[test]
fn rows_at_least_zero_keeps_everything() {
    let rows = [Row("trivial", 0), Row("nasty", 23)];
    assert_eq!(rows_at_least(&rows, 0).len(), 2);
}

#[test]
fn rows_at_least_above_max_keeps_nothing() {
    let rows = [Row("trivial", 0), Row("nasty", 23)];
    assert!(rows_at_least(&rows, 24).is_empty());
}

#[test]
fn weighted_mi_weighs_by_lines_and_floors_at_zero() {
    // 900 lines at 80 and 100 lines at -200 (taken as 0): 72.
    let mi = weighted_mi([(80.0, 900), (-200.0, 100)].into_iter());
    assert!((mi - 72.0).abs() < 1e-9);
    assert_eq!(weighted_mi(std::iter::empty()), 0.0);
}

#[test]
fn tables_show_top_rows_json_every_entry_unless_asked() {
    use crate::cli::OutputMode;
    assert_eq!(shown_count(OutputMode::Table, None, 30), DEFAULT_TOP);
    assert_eq!(shown_count(OutputMode::Table, Some(5), 30), 5);
    assert_eq!(shown_count(OutputMode::Table, None, 3), 3);
    assert_eq!(shown_count(OutputMode::Json, None, 30), 30);
    assert_eq!(shown_count(OutputMode::Json, Some(5), 30), 5);
    assert_eq!(shown_count(OutputMode::Short, Some(5), 30), 30);
    assert_eq!(shown_count(OutputMode::Github, None, 30), 30);
}

#[test]
fn total_label_says_how_many_are_shown() {
    assert_eq!(total_label(30, 20), " Total (30 files, 20 shown)");
    assert_eq!(total_label(3, 3), " Total (3 files)");
}

#[test]
fn shown_paths_have_no_leading_dot() {
    assert_eq!(shown(std::path::Path::new("./src/a.js")), "src/a.js");
    assert_eq!(
        shown(std::path::Path::new("src/missing.js")),
        "src/missing.js"
    );
}

#[test]
fn output_results_totals_every_entry_and_shows_the_top() {
    let rows: Vec<u32> = (0..30).collect();
    let mut seen = (0, 0);
    output_results(
        &rows,
        None,
        crate::cli::OutputMode::Table,
        |_| Ok(()),
        |all, shown| seen = (all.len(), shown),
        |_| {},
        |_| {},
    )
    .unwrap();
    assert_eq!(seen, (30, DEFAULT_TOP));
}
