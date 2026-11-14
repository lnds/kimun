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
    assert_eq!(baseline.graded, default.graded);
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
    let (b, e) = (baseline.graded.unwrap(), experimental.graded.unwrap());
    assert!(text.contains(&format!("{:.1}", b.score)), "{text}");
    assert!(
        text.contains(&format!("{:+.1}", e.score - b.score)),
        "{text}"
    );
}

#[test]
fn the_mi_dimension_counts_each_file_once() {
    let dir = tempfile::tempdir().unwrap();
    // One large tangled file and four small clean ones.
    let big: String = (0..300)
        .map(|i| format!("pub fn f{i}(x: u32) -> u32 {{\n    if x > {i} && x < {i} * 7 {{\n        return x * {i};\n    }}\n    0\n}}\n"))
        .collect();
    std::fs::write(dir.path().join("big.rs"), big).unwrap();
    for n in 0..4 {
        std::fs::write(
            dir.path().join(format!("s{n}.rs")),
            "pub fn g() -> u32 {\n    1\n}\n",
        )
        .unwrap();
    }
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let measured = measure(&cfg, 6, &ScoringModel::ExperimentalMi);
    let experimental = snapshot_of(&measured, 5, &ScoringModel::ExperimentalMi).score;
    let mi = experimental
        .dimensions
        .iter()
        .find(|d| d.name == "Maintainability Index")
        .unwrap();
    // Weighted by lines the large file would dominate; per file the four
    // small files hold four fifths of the mean.
    let mi = mi.graded.unwrap().score;
    assert!(mi > 70.0, "{mi}");
}

#[test]
fn the_comparison_of_nothing_to_grade_shows_dashes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("README.md"), "# Notes\n\nSome text.\n").unwrap();
    let filter = ExcludeFilter::default();
    let cfg = WalkConfig::new(dir.path(), false, &filter);
    let measured = measure(&cfg, 6, &ScoringModel::ExperimentalMi);
    let baseline = snapshot_of(&measured, 5, &ScoringModel::Cognitive).score;
    let experimental = snapshot_of(&measured, 5, &ScoringModel::ExperimentalMi).score;
    assert_eq!(
        comparison(&baseline, &experimental),
        "\n Experimental: Maintainability Index (Visual Studio) in place of Halstead Effort.\n \
         Default model: -  \u{2192}  experimental: -\n"
    );
}
