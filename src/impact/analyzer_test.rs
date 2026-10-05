use super::*;
use crate::git::ChangeKind;

fn change(path: &str, added: usize, deleted: usize) -> FileDiffStat {
    FileDiffStat {
        path: PathBuf::from(path),
        old_path: Some(PathBuf::from(path)),
        kind: ChangeKind::Modified,
        added,
        deleted,
        lines: Vec::new(),
        old_lines: Vec::new(),
        probe: Vec::new(),
    }
}

fn target(path: &str) -> Target {
    Target {
        path: PathBuf::from(path),
        history_path: PathBuf::from(path),
    }
}

/// A target, its commits, and the files sharing commits with it.
type Entry<'a> = (&'a str, usize, &'a [(&'a str, usize)]);

/// History where each entry is recorded as given.
fn history(entries: &[Entry<'_>]) -> CoChangeHistory {
    let mut history = CoChangeHistory::default();
    for (path, commits, shared) in entries {
        history.commits.insert(PathBuf::from(path), *commits);
        history.shared.insert(
            PathBuf::from(path),
            shared
                .iter()
                .map(|(other, n)| (PathBuf::from(other), *n))
                .collect(),
        );
    }
    history
}

const DEFAULTS: Thresholds = Thresholds {
    min_confidence: 0.5,
    min_shared: 3,
};

#[test]
fn diffusion_of_an_empty_change() {
    let d = compute_diffusion(&[]);
    assert_eq!(
        d,
        Diffusion {
            files: 0,
            directories: 0,
            subsystems: 0,
            lines_added: 0,
            lines_deleted: 0,
            entropy: 0.0,
        }
    );
}

#[test]
fn diffusion_counts_files_directories_and_subsystems() {
    let d = compute_diffusion(&[
        change("src/tc/mod.rs", 10, 2),
        change("src/tc/report.rs", 5, 0),
        change("src/git/mod.rs", 1, 1),
        change("docs/guide.md", 4, 0),
        change("README.md", 3, 3),
    ]);
    assert_eq!(d.files, 5);
    // src/tc, src/git, docs and the root
    assert_eq!(d.directories, 4);
    // src, docs and the root
    assert_eq!(d.subsystems, 3);
    assert_eq!(d.lines_added, 23);
    assert_eq!(d.lines_deleted, 6);
}

#[test]
fn root_files_share_one_subsystem() {
    let d = compute_diffusion(&[change("README.md", 1, 0), change("Cargo.toml", 1, 0)]);
    assert_eq!(d.subsystems, 1);
    assert_eq!(d.directories, 1);
}

#[test]
fn entropy_is_zero_for_a_single_file() {
    assert_eq!(compute_diffusion(&[change("a.rs", 40, 2)]).entropy, 0.0);
}

#[test]
fn entropy_is_one_when_lines_are_evenly_spread() {
    let d = compute_diffusion(&[
        change("a.rs", 5, 5),
        change("b.rs", 10, 0),
        change("c.rs", 0, 10),
    ]);
    assert!((d.entropy - 1.0).abs() < 1e-9, "got {}", d.entropy);
}

#[test]
fn entropy_drops_when_one_file_holds_most_of_the_change() {
    let d = compute_diffusion(&[
        change("a.rs", 98, 0),
        change("b.rs", 1, 0),
        change("c.rs", 1, 0),
    ]);
    assert!(d.entropy > 0.0 && d.entropy < 0.2, "got {}", d.entropy);
}

#[test]
fn entropy_ignores_files_without_modified_lines() {
    // A binary file counts no lines; it must not lower the entropy of the rest.
    let d = compute_diffusion(&[
        change("a.rs", 10, 0),
        change("b.rs", 10, 0),
        change("logo.png", 0, 0),
    ]);
    assert!((d.entropy - 1.0).abs() < 1e-9, "got {}", d.entropy);
    assert_eq!(d.files, 3);
}

#[test]
fn reports_a_file_that_usually_changes_with_a_target() {
    let h = history(&[("a.rs", 10, &[("b.rs", 8)])]);
    let missing = missing_co_changes(&h, &[target("a.rs")], |_| true, DEFAULTS);

    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].path, PathBuf::from("b.rs"));
    let best = missing[0].best();
    assert_eq!(best.path, PathBuf::from("a.rs"));
    assert_eq!((best.shared_commits, best.commits), (8, 10));
    assert!((best.confidence - 0.8).abs() < 1e-9);
}

#[test]
fn confidence_is_relative_to_the_changed_file() {
    // b.rs changed 3 times, always with a.rs, which changed 100 times. A
    // symmetric strength would be 1.0; changing a.rs calls for b.rs 3% of the time.
    let h = history(&[("a.rs", 100, &[("b.rs", 3)])]);
    let missing = missing_co_changes(&h, &[target("a.rs")], |_| true, DEFAULTS);
    assert!(missing.is_empty());

    // Seen from b.rs, a.rs is always expected.
    let h = history(&[("b.rs", 3, &[("a.rs", 3)])]);
    let missing = missing_co_changes(&h, &[target("b.rs")], |_| true, DEFAULTS);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].best().confidence, 1.0);
}

#[test]
fn thresholds_are_inclusive() {
    let h = history(&[("a.rs", 6, &[("at.rs", 3), ("below_conf.rs", 2)])]);
    let missing = missing_co_changes(&h, &[target("a.rs")], |_| true, DEFAULTS);
    // 3/6 = 0.5 with 3 shared commits passes; 2/6 does not.
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].path, PathBuf::from("at.rs"));
}

#[test]
fn min_shared_rejects_thin_evidence() {
    // 2/2 is full confidence on two commits.
    let h = history(&[("a.rs", 2, &[("b.rs", 2)])]);
    assert!(missing_co_changes(&h, &[target("a.rs")], |_| true, DEFAULTS).is_empty());

    let loose = Thresholds {
        min_confidence: 0.5,
        min_shared: 2,
    };
    assert_eq!(
        missing_co_changes(&h, &[target("a.rs")], |_| true, loose).len(),
        1
    );
}

#[test]
fn candidates_rejected_by_the_filter_are_dropped() {
    let h = history(&[("a.rs", 4, &[("kept.rs", 4), ("gone.rs", 4)])]);
    let missing = missing_co_changes(
        &h,
        &[target("a.rs")],
        |p| p != Path::new("gone.rs"),
        DEFAULTS,
    );
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].path, PathBuf::from("kept.rs"));
}

#[test]
fn several_triggers_for_one_file_are_merged_strongest_first() {
    let h = history(&[
        ("a.rs", 10, &[("shared.rs", 6)]),
        ("b.rs", 5, &[("shared.rs", 5)]),
    ]);
    let missing = missing_co_changes(&h, &[target("a.rs"), target("b.rs")], |_| true, DEFAULTS);

    assert_eq!(missing.len(), 1);
    let triggers: Vec<&Path> = missing[0]
        .triggers
        .iter()
        .map(|t| t.path.as_path())
        .collect();
    assert_eq!(triggers, [Path::new("b.rs"), Path::new("a.rs")]);
    assert_eq!(missing[0].best().confidence, 1.0);
}

#[test]
fn missing_files_are_sorted_by_confidence_then_shared_then_path() {
    let h = history(&[(
        "a.rs",
        10,
        &[
            ("low.rs", 5),
            ("high.rs", 9),
            ("z_tie.rs", 7),
            ("b_tie.rs", 7),
        ],
    )]);
    let missing = missing_co_changes(&h, &[target("a.rs")], |_| true, DEFAULTS);
    let order: Vec<&Path> = missing.iter().map(|m| m.path.as_path()).collect();
    assert_eq!(
        order,
        [
            Path::new("high.rs"),
            Path::new("b_tie.rs"),
            Path::new("z_tie.rs"),
            Path::new("low.rs"),
        ]
    );
}

#[test]
fn renamed_target_reads_history_under_its_old_path() {
    let h = history(&[("old.rs", 4, &[("b.rs", 4)])]);
    let renamed = Target {
        path: PathBuf::from("new.rs"),
        history_path: PathBuf::from("old.rs"),
    };
    let missing = missing_co_changes(&h, &[renamed], |_| true, DEFAULTS);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].best().path, PathBuf::from("new.rs"));
}

#[test]
fn target_without_history_predicts_nothing() {
    let h = history(&[]);
    assert!(missing_co_changes(&h, &[target("a.rs")], |_| true, DEFAULTS).is_empty());
}
