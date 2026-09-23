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
