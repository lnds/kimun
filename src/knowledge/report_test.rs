use super::*;
use crate::knowledge::analyzer::{AuthorSummary, BusFactor, RiskLevel, compute_bus_factor};
use std::path::PathBuf;

fn sample_files() -> Vec<FileOwnership> {
    vec![
        FileOwnership {
            path: PathBuf::from("src/foo.rs"),
            language: "Rust".to_string(),
            total_lines: 731,
            primary_owner: "Lautaro".to_string(),
            primary_email: "lautaro@ruca.mapu".to_string(),
            ownership_pct: 94.0,
            contributors: 2,
            risk: RiskLevel::Critical,
            knowledge_loss: true,
        },
        FileOwnership {
            path: PathBuf::from("src/bar.rs"),
            language: "Rust".to_string(),
            total_lines: 241,
            primary_owner: "Caupolicán".to_string(),
            primary_email: "caupolican@ruca.mapu".to_string(),
            ownership_pct: 78.0,
            contributors: 3,
            risk: RiskLevel::High,
            knowledge_loss: false,
        },
    ]
}

#[test]
fn print_report_does_not_panic() {
    print_report(&sample_files(), 1);
}

#[test]
fn print_report_empty() {
    print_report(&[], 0);
}

#[test]
fn print_json_does_not_panic() {
    print_json(&sample_files()).unwrap();
}

#[test]
fn print_json_empty() {
    print_json(&[]).unwrap();
}

// ── print_summary_report ───────────────────────────────────────────────

fn sample_authors() -> Vec<AuthorSummary> {
    vec![
        AuthorSummary {
            author: "Alice".to_string(),
            files_owned: 5,
            total_lines: 1000,
            languages: vec!["Rust".to_string()],
            worst_risk: RiskLevel::Critical,
            knowledge_loss_files: 2,
        },
        AuthorSummary {
            author: "Bob".to_string(),
            files_owned: 3,
            total_lines: 500,
            languages: vec!["Python".to_string(), "Rust".to_string()],
            worst_risk: RiskLevel::Medium,
            knowledge_loss_files: 0,
        },
    ]
}

#[test]
fn print_summary_report_empty() {
    print_summary_report(&[]);
}

#[test]
fn print_summary_report_does_not_panic() {
    print_summary_report(&sample_authors());
}

#[test]
fn print_summary_json_empty() {
    print_summary_json(&[]).unwrap();
}

#[test]
fn print_summary_json_does_not_panic() {
    print_summary_json(&sample_authors()).unwrap();
}

// ── bus factor ─────────────────────────────────────────────────────────────

fn file(authors: &[(&str, usize)]) -> Vec<crate::git::BlameInfo> {
    authors
        .iter()
        .map(|(a, n)| crate::git::BlameInfo {
            author: a.to_string(),
            email: format!("{a}@x.org"),
            lines: *n,
            last_commit_time: 0,
        })
        .collect()
}

fn long_tail() -> BusFactor {
    let mut files = vec![file(&[("Ana", 30)]), file(&[("Bea", 30)])];
    for dev in ["c", "d", "e", "f", "g", "h", "i", "j"] {
        files.push(file(&[(dev, 5)]));
    }
    compute_bus_factor(&files, None, 50.0)
}

#[test]
fn bus_factor_report_no_data() {
    let bf = compute_bus_factor(&[], None, 50.0);
    assert_eq!(
        render_bus_factor_report(&bf, 20, None),
        "No blame data found for bus factor analysis.\n"
    );
}

#[test]
fn bus_factor_report_says_what_the_number_means() {
    let out = render_bus_factor_report(&long_tail(), 20, None);
    assert!(out.starts_with("Project Bus Factor: 2\n"), "{out}");
    assert!(
        out.contains(
            "If these 2 contributors leave, 60% of the code is in files with no author left."
        ),
        "{out}"
    );
    assert!(out.contains("Risk: HIGH"), "{out}");
    assert!(out.contains("threshold 50%"), "{out}");
    assert!(out.contains("30.0%") && out.contains("60.0%"), "{out}");
}

#[test]
fn bus_factor_report_singular() {
    let bf = compute_bus_factor(&[file(&[("Solo", 10)])], None, 50.0);
    let out = render_bus_factor_report(&bf, 20, None);
    assert!(out.contains("If this contributor leaves, 100%"), "{out}");
    assert!(out.contains("Risk: CRITICAL — one person"), "{out}");
}

#[test]
fn bus_factor_report_limits_rows_beyond_the_factor() {
    let out = render_bus_factor_report(&long_tail(), 3, None);
    assert!(out.contains(" 5  e "), "{out}");
    assert!(!out.contains(" 6  f "), "{out}");
    assert!(out.contains("5 more contributors (see --top)"), "{out}");
}

#[test]
fn bus_factor_report_with_inactive_authors() {
    let files = [file(&[("Ana", 90)]), file(&[("Bea", 10)])];
    let active = std::collections::HashSet::from(["bea@x.org".to_string()]);
    let bf = compute_bus_factor(&files, Some(&active), 50.0);
    let out = render_bus_factor_report(&bf, 20, Some("1y"));
    assert!(out.starts_with("Project Bus Factor: 0\n"), "{out}");
    assert!(
        out.contains("90% of the code is already in files whose authors have not committed in 1y."),
        "{out}"
    );
    assert!(
        out.contains("Risk: CRITICAL — the authors of most of the code are already gone"),
        "{out}"
    );
    assert!(out.contains("Ana (inactive)"), "{out}");
    assert!(!out.contains("If this"), "{out}");
}

#[test]
fn bus_factor_json_fields() {
    let json = serde_json::to_value(bus_factor_json(&long_tail())).unwrap();
    assert_eq!(json["factor"], 2);
    assert_eq!(json["threshold"], 50.0);
    assert_eq!(json["author_share"], 0.75);
    assert_eq!(json["total_files"], 10);
    assert_eq!(json["contributors"][1]["orphaned_pct"], 60.0);
    assert!(json["contributors"][2].get("orphaned_pct").is_none());
    assert!(json.get("orphaned_by_inactivity").is_none());
}

#[test]
fn bus_factor_printers_do_not_panic() {
    let bf = long_tail();
    print_bus_factor_report(&bf, 20, None);
    print_bus_factor_short(&bf);
    print_bus_factor_terse(&bf);
    print_bus_factor_json(&bf).unwrap();
}
