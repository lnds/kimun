use super::*;

#[test]
fn score_is_red_below_70_and_green_from_90() {
    assert!(score_heat(30.0) <= -1.0 + 1e-9);
    assert!(score_heat(69.0) < 0.0);
    assert_eq!(score_heat(80.0), 0.0);
    assert!(score_heat(90.0) > 0.0);
    assert_eq!(score_heat(100.0), 1.0);
}

#[test]
fn small_hotspots_are_fine_and_the_top_one_is_reddest() {
    assert_eq!(hotspot_heat(5, 100), 0.0);
    assert_eq!(hotspot_heat(50, 100), -0.5);
    assert_eq!(hotspot_heat(100, 100), -1.0);
    assert_eq!(hotspot_heat(0, 0), 0.0);
}

#[test]
fn levels_map_to_heats() {
    assert_eq!(churn_heat(ChurnLevel::High), -1.0);
    assert_eq!(churn_heat(ChurnLevel::Low), 0.0);
    assert!(age_heat(AgeStatus::Active) > 0.0);
    assert_eq!(age_heat(AgeStatus::Stale), 0.0);
    assert_eq!(risk_heat(RiskLevel::Low, true), -1.0);
    assert_eq!(risk_heat(RiskLevel::Medium, false), -0.33);
}

#[test]
fn score_shades_key_files_by_their_path_under_the_root() {
    let file = FileScore {
        path: PathBuf::from("/repo/src/a.rs"),
        score: 50.0,
        grade: crate::score::analyzer::Grade::D,
        loc: 10,
        issues: vec![],
    };
    let (shades, legend) = by_score(Path::new("/repo"), &[file]);
    let shade = &shades[Path::new("src/a.rs")];
    assert_eq!(shade.heat, -0.5);
    assert_eq!(shade.label, "score 50.0 (D)");
    assert_eq!(legend.len(), 3);
}
