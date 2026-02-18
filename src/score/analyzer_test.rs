use super::*;

#[test]
fn test_score_to_grade_all_boundaries() {
    assert_eq!(score_to_grade(100.0), Grade::APlusPlus);
    assert_eq!(score_to_grade(97.0), Grade::APlusPlus);
    assert_eq!(score_to_grade(96.9), Grade::APlus);
    assert_eq!(score_to_grade(93.0), Grade::APlus);
    assert_eq!(score_to_grade(92.9), Grade::A);
    assert_eq!(score_to_grade(90.0), Grade::A);
    assert_eq!(score_to_grade(89.9), Grade::AMinus);
    assert_eq!(score_to_grade(87.0), Grade::AMinus);
    assert_eq!(score_to_grade(86.9), Grade::BPlus);
    assert_eq!(score_to_grade(83.0), Grade::BPlus);
    assert_eq!(score_to_grade(82.9), Grade::B);
    assert_eq!(score_to_grade(80.0), Grade::B);
    assert_eq!(score_to_grade(79.9), Grade::BMinus);
    assert_eq!(score_to_grade(77.0), Grade::BMinus);
    assert_eq!(score_to_grade(76.9), Grade::CPlus);
    assert_eq!(score_to_grade(73.0), Grade::CPlus);
    assert_eq!(score_to_grade(72.9), Grade::C);
    assert_eq!(score_to_grade(70.0), Grade::C);
    assert_eq!(score_to_grade(69.9), Grade::CMinus);
    assert_eq!(score_to_grade(67.0), Grade::CMinus);
    assert_eq!(score_to_grade(66.9), Grade::DPlus);
    assert_eq!(score_to_grade(63.0), Grade::DPlus);
    assert_eq!(score_to_grade(62.9), Grade::D);
    assert_eq!(score_to_grade(60.0), Grade::D);
    assert_eq!(score_to_grade(59.9), Grade::DMinus);
    assert_eq!(score_to_grade(57.0), Grade::DMinus);
    assert_eq!(score_to_grade(56.9), Grade::F);
    assert_eq!(score_to_grade(50.0), Grade::F);
    assert_eq!(score_to_grade(49.9), Grade::FMinus);
    assert_eq!(score_to_grade(40.0), Grade::FMinus);
    assert_eq!(score_to_grade(39.9), Grade::FMinusMinus);
    assert_eq!(score_to_grade(0.0), Grade::FMinusMinus);
}

#[test]
fn test_compute_project_score() {
    let dims = vec![
        DimensionScore {
            name: "A",
            weight: 0.60,
            score: 80.0,
            grade: Grade::B,
        },
        DimensionScore {
            name: "B",
            weight: 0.40,
            score: 60.0,
            grade: Grade::D,
        },
    ];
    let s = compute_project_score(&dims);
    assert!((s - 72.0).abs() < 0.01, "expected 72, got {s}");
}

#[test]
fn test_compute_project_score_empty() {
    assert!((compute_project_score(&[]) - 0.0).abs() < 0.01);
}
