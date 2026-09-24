use super::*;

fn change(path: &str, old_path: Option<&str>) -> FileChange {
    FileChange {
        path: PathBuf::from(path),
        old_path: old_path.map(PathBuf::from),
    }
}

fn scores(entries: &[(&str, f64)]) -> HashMap<PathBuf, f64> {
    entries
        .iter()
        .map(|(p, s)| (PathBuf::from(p), *s))
        .collect()
}

fn file(path: &str, before: Option<f64>, after: f64) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        before,
        after,
        delta: before.map(|b| after - b),
    }
}

fn scope(files: Vec<ChangedFile>, dup_before: usize, dup_after: usize) -> ChangedScope {
    ChangedScope {
        files,
        duplicated_lines_before: dup_before,
        duplicated_lines_after: dup_after,
    }
}

#[test]
fn changed_files_pairs_modified_renamed_and_new() {
    let changes = [
        change("a.rs", Some("a.rs")),
        change("b_new_name.rs", Some("b.rs")),
        change("c.rs", None),
    ];
    let before = scores(&[("a.rs", 80.0), ("b.rs", 70.0)]);
    let after = scores(&[("a.rs", 75.0), ("b_new_name.rs", 72.0), ("c.rs", 60.0)]);

    let files = changed_files(&changes, Path::new(""), &before, &after);

    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["a.rs", "b_new_name.rs", "c.rs"]);
    assert_eq!(files[0].delta, Some(-5.0));
    assert_eq!(files[1].before, Some(70.0));
    assert_eq!(files[2].before, None);
    assert_eq!(files[2].delta, None);
}

#[test]
fn changed_files_maps_repo_paths_into_analyzed_subdir() {
    let changes = [
        change("lib/x.ex", Some("lib/x.ex")),
        change("docs/y.ex", None),
    ];
    let before = scores(&[("x.ex", 90.0)]);
    let after = scores(&[("x.ex", 85.0), ("y.ex", 50.0)]);

    let files = changed_files(&changes, Path::new("lib"), &before, &after);

    assert_eq!(files.len(), 1, "changes outside the prefix are skipped");
    assert_eq!(files[0].path, "lib/x.ex", "paths stay repo-relative");
    assert_eq!(files[0].delta, Some(-5.0));
}

#[test]
fn changed_files_skips_unscored_files() {
    let changes = [change("README.md", Some("README.md"))];
    let files = changed_files(&changes, Path::new(""), &scores(&[]), &scores(&[]));
    assert!(files.is_empty());
}

#[test]
fn changed_files_treats_previously_unscored_file_as_new() {
    let changes = [change("a.rs", Some("a.rs"))];
    let files = changed_files(
        &changes,
        Path::new(""),
        &scores(&[]),
        &scores(&[("a.rs", 70.0)]),
    );
    assert_eq!(files[0].before, None);
}

#[test]
fn gate_failure_names_each_file_over_tolerance() {
    let s = scope(
        vec![
            file("worse.rs", Some(80.0), 75.1),
            file("noise.rs", Some(75.2), 75.195),
            file("better.rs", Some(60.0), 65.0),
        ],
        10,
        10,
    );
    let msg = gate_failure(&s, 0.01, 90.0).expect("worse.rs dropped 4.9 points");
    assert!(
        msg.contains("worse.rs dropped 80.00 → 75.10 (-4.9000)"),
        "{msg}"
    );
    assert!(!msg.contains("noise.rs"), "{msg}");
    assert!(!msg.contains("better.rs"), "{msg}");
}

#[test]
fn gate_failure_ignores_new_files() {
    let s = scope(vec![file("new.rs", None, 5.0)], 0, 0);
    assert!(gate_failure(&s, 0.0, 90.0).is_none());
}

#[test]
fn gate_failure_passes_when_nothing_regressed() {
    let s = scope(vec![file("a.rs", Some(80.0), 80.0)], 12, 12);
    assert!(gate_failure(&s, 0.01, 90.0).is_none());
}

#[test]
fn gate_failure_catches_duplicated_lines_growth() {
    let s = scope(vec![], 120, 150);
    let msg = gate_failure(&s, 0.01, 90.0).expect("duplicated lines grew");
    assert!(
        msg.contains("duplicated lines grew 120 → 150 (+30)"),
        "{msg}"
    );
}

#[test]
fn gate_failure_allows_duplicated_lines_to_shrink() {
    let s = scope(vec![], 150, 120);
    assert!(gate_failure(&s, 0.01, 90.0).is_none());
}

#[test]
fn gate_failure_lists_every_reason() {
    let s = scope(vec![file("a.rs", Some(80.0), 70.0)], 0, 8);
    let msg = gate_failure(&s, 0.01, 90.0).unwrap();
    assert!(
        msg.contains("a.rs dropped") && msg.contains("duplicated lines grew"),
        "{msg}"
    );
}

#[test]
fn gate_failure_lets_files_above_the_baseline_drop() {
    let s = scope(vec![file("healthy.rs", Some(98.0), 92.0)], 0, 0);
    assert!(gate_failure(&s, 0.5, 90.0).is_none());
}

#[test]
fn gate_failure_catches_a_file_that_falls_below_the_baseline() {
    let s = scope(vec![file("slipping.rs", Some(91.0), 89.0)], 0, 0);
    let msg = gate_failure(&s, 0.5, 90.0).expect("ends below 90 after a 2-point drop");
    assert!(msg.contains("below the project score 90.00"), "{msg}");
}

#[test]
fn gate_failure_tolerates_a_small_drop_below_the_baseline() {
    // The #54 case: 75.2 → 75.1 in a project scoring 77.78.
    let s = scope(vec![file("currency_edit.ex", Some(75.2), 75.1)], 0, 0);
    assert!(gate_failure(&s, 0.5, 77.78).is_none());
    assert!(gate_failure(&s, 0.01, 77.78).is_some());
}
