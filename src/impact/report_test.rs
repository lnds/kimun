use super::*;
use crate::impact::analyzer::{Diffusion, Thresholds};
use std::path::PathBuf;

fn trigger(path: &str, shared: usize, commits: usize) -> Trigger {
    Trigger {
        path: PathBuf::from(path),
        shared_commits: shared,
        commits,
        confidence: shared as f64 / commits as f64,
    }
}

fn missing(path: &str, triggers: Vec<Trigger>) -> MissingCoChange {
    MissingCoChange {
        path: PathBuf::from(path),
        triggers,
    }
}

fn sample() -> Impact {
    Impact {
        since_ref: "main".to_string(),
        diffusion: Diffusion {
            files: 5,
            directories: 3,
            subsystems: 2,
            lines_added: 120,
            lines_deleted: 30,
            entropy: 0.8234,
        },
        thresholds: Thresholds {
            min_confidence: 0.5,
            min_shared: 3,
        },
        missing: vec![
            missing(
                "src/tc/report.rs",
                vec![
                    trigger("src/tc/mod.rs", 8, 10),
                    trigger("src/tc/analyzer.rs", 3, 6),
                ],
            ),
            missing("README.md", vec![trigger("src/cli.rs", 6, 12)]),
        ],
        without_history: vec![PathBuf::from("src/impact/mod.rs")],
        generated_skipped: 1,
        max_changeset: 30,
        skipped_commits: 0,
    }
}

fn empty() -> Impact {
    Impact {
        diffusion: Diffusion {
            files: 0,
            directories: 0,
            subsystems: 0,
            lines_added: 0,
            lines_deleted: 0,
            entropy: 0.0,
        },
        missing: vec![],
        without_history: vec![],
        generated_skipped: 0,
        ..sample()
    }
}

#[test]
fn report_shows_the_diffusion_block() {
    let out = render_report(&sample(), 20);
    assert!(
        out.starts_with("Change Impact — diff against main\n"),
        "{out}"
    );
    for (label, value) in [
        ("Files changed", "5"),
        ("Directories", "3"),
        ("Subsystems", "2"),
        ("Lines added", "120"),
        ("Lines deleted", "30"),
        ("Entropy", "0.82"),
    ] {
        let line = out
            .lines()
            .find(|l| l.trim_start().starts_with(label))
            .unwrap_or_else(|| panic!("no {label} line in:\n{out}"));
        assert!(
            line.split_whitespace().any(|w| w == value),
            "{label} should be {value}, got: {line}"
        );
    }
}

#[test]
fn report_lists_missing_files_with_their_evidence() {
    let out = render_report(&sample(), 20);
    let line = out
        .lines()
        .find(|l| l.trim_start().starts_with("src/tc/report.rs"))
        .expect("row for src/tc/report.rs");
    let cells: Vec<&str> = line.split_whitespace().collect();
    assert_eq!(
        cells,
        [
            "src/tc/report.rs",
            "0.80",
            "8/10",
            "src/tc/mod.rs",
            "(+1",
            "more)"
        ]
    );

    let line = out
        .lines()
        .find(|l| l.trim_start().starts_with("README.md"))
        .expect("row for README.md");
    let cells: Vec<&str> = line.split_whitespace().collect();
    assert_eq!(cells, ["README.md", "0.50", "6/12", "src/cli.rs"]);
}

#[test]
fn report_names_files_without_history_and_generated_ones() {
    let out = render_report(&sample(), 20);
    assert!(
        out.contains("No history before the diff (new or never committed): src/impact/mod.rs"),
        "{out}"
    );
    assert!(out.contains("Generated files ignored: 1"), "{out}");
}

#[test]
fn report_truncates_to_top_and_says_so() {
    let out = render_report(&sample(), 1);
    assert!(out.contains("src/tc/report.rs"));
    assert!(!out.contains("README.md"), "{out}");
    assert!(out.contains("2 missing files (1 shown)."), "{out}");
}

#[test]
fn report_without_missing_files_states_the_thresholds() {
    let impact = Impact {
        missing: vec![],
        ..sample()
    };
    let out = render_report(&impact, 20);
    assert!(
        out.contains("No missing co-changes (confidence >= 0.50, shared commits >= 3)."),
        "{out}"
    );
    assert!(!out.contains("Missing file"), "{out}");
}

#[test]
fn report_of_an_empty_diff() {
    assert_eq!(render_report(&empty(), 20), "No changes against 'main'.\n");
}

#[test]
fn report_of_a_diff_with_only_generated_files() {
    let impact = Impact {
        generated_skipped: 2,
        ..empty()
    };
    assert_eq!(
        render_report(&impact, 20),
        "No changes against 'main'.\nGenerated files ignored: 2\n"
    );
}

#[test]
fn short_line() {
    assert_eq!(
        render_short(&sample()),
        "impact files:5 dirs:3 subsystems:2 added:120 deleted:30 entropy:0.82 missing:2 max_confidence:0.80"
    );
    assert!(render_short(&empty()).ends_with("missing:0 max_confidence:0.00"));
}

#[test]
fn json_carries_every_trigger_and_rounds() {
    let json = serde_json::to_value(to_json(&sample(), 20)).unwrap();
    assert_eq!(json["since_ref"], "main");
    assert_eq!(json["diffusion"]["files"], 5);
    assert_eq!(json["diffusion"]["entropy"], 0.82);
    assert_eq!(json["generated_skipped"], 1);

    let radius = &json["logical_radius"];
    assert_eq!(radius["min_confidence"], 0.5);
    assert_eq!(radius["min_shared"], 3);
    assert_eq!(radius["total_missing"], 2);
    assert_eq!(radius["without_history"][0], "src/impact/mod.rs");

    let first = &radius["missing"][0];
    assert_eq!(first["path"], "src/tc/report.rs");
    assert_eq!(first["confidence"], 0.8);
    assert_eq!(first["shared_commits"], 8);
    assert_eq!(first["triggers"].as_array().unwrap().len(), 2);
    assert_eq!(first["triggers"][1]["path"], "src/tc/analyzer.rs");
    assert_eq!(first["triggers"][1]["commits"], 6);
}

#[test]
fn json_truncates_to_top_but_keeps_the_total() {
    let json = serde_json::to_value(to_json(&sample(), 1)).unwrap();
    let radius = &json["logical_radius"];
    assert_eq!(radius["missing"].as_array().unwrap().len(), 1);
    assert_eq!(radius["total_missing"], 2);
}

#[test]
fn printers_do_not_panic() {
    print_report(&sample(), 20);
    print_json(&sample(), 20).unwrap();
    print_short(&sample());
    print_terse(&sample());
}

#[test]
fn report_says_how_many_commits_were_ignored() {
    let impact = Impact {
        skipped_commits: 4,
        ..sample()
    };
    let out = render_report(&impact, 20);
    assert!(
        out.contains("Commits ignored for touching more than 30 files: 4"),
        "{out}"
    );
    assert!(!render_report(&sample(), 20).contains("Commits ignored"));

    let json = serde_json::to_value(to_json(&impact, 20)).unwrap();
    assert_eq!(json["logical_radius"]["max_changeset"], 30);
    assert_eq!(json["logical_radius"]["skipped_commits"], 4);
}
