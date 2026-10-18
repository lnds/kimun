use super::*;
use crate::walk::ExcludeFilter;

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let body: String = (0..40)
        .map(|i| format!("pub fn f{i}(x: u32) -> u32 {{\n    if x > {i} {{\n        return x;\n    }}\n    0\n}}\n"))
        .collect();
    std::fs::write(dir.path().join("lib.rs"), body).unwrap();
    std::fs::write(
        dir.path().join("small.rs"),
        "pub fn g() -> u32 {\n    1\n}\n",
    )
    .unwrap();
    dir
}

#[test]
fn experimental_model_replaces_halstead_with_mi() {
    let dir = project();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let measured = measure(&cfg, 6, &ScoringModel::ExperimentalMi);
    let baseline = snapshot_of(&measured, 5, &ScoringModel::Cognitive).score;
    let experimental = snapshot_of(&measured, 5, &ScoringModel::ExperimentalMi).score;

    let names = |s: &ProjectScore| s.dimensions.iter().map(|d| d.name).collect::<Vec<_>>();
    assert!(names(&baseline).contains(&"Halstead Effort"));
    assert!(names(&experimental).contains(&"Maintainability Index"));
    assert!(!names(&experimental).contains(&"Halstead Effort"));
    assert_eq!(baseline.files_analyzed, experimental.files_analyzed);
    let weight =
        |s: &ProjectScore, n: &str| s.dimensions.iter().find(|d| d.name == n).unwrap().weight;
    assert_eq!(
        weight(&baseline, "Halstead Effort"),
        weight(&experimental, "Maintainability Index")
    );
}

#[test]
fn the_baseline_from_one_measurement_is_the_default_score() {
    let dir = project();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let measured = measure(&cfg, 6, &ScoringModel::ExperimentalMi);
    let baseline = snapshot_of(&measured, 5, &ScoringModel::Cognitive).score;
    let default = super::super::compute_score(&cfg, 5, 6, &ScoringModel::Cognitive).unwrap();
    assert!((baseline.score - default.score).abs() < 1e-9);
}

#[test]
fn comparison_names_both_scores_and_the_change() {
    let dir = project();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let measured = measure(&cfg, 6, &ScoringModel::ExperimentalMi);
    let baseline = snapshot_of(&measured, 5, &ScoringModel::Cognitive).score;
    let experimental = snapshot_of(&measured, 5, &ScoringModel::ExperimentalMi).score;
    let text = comparison(&baseline, &experimental);
    assert!(text.contains("in place of Halstead Effort"), "{text}");
    assert!(text.contains(&format!("{:.1}", baseline.score)), "{text}");
    assert!(
        text.contains(&format!("{:+.1}", experimental.score - baseline.score)),
        "{text}"
    );
}
