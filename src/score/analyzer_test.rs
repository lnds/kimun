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
            graded: Some(Graded::of(80.0)),
        },
        DimensionScore {
            name: "B",
            weight: 0.40,
            graded: Some(Graded::of(60.0)),
        },
    ];
    let s = compute_project_score(&dims).unwrap();
    assert!((s - 72.0).abs() < 0.01, "expected 72, got {s}");
}

#[test]
fn test_compute_project_score_empty() {
    assert_eq!(compute_project_score(&[]), Some(0.0));
}

#[test]
fn a_project_with_an_ungraded_dimension_has_no_score() {
    let dims = vec![
        DimensionScore {
            name: "A",
            weight: 0.60,
            graded: Some(Graded::of(80.0)),
        },
        DimensionScore {
            name: "B",
            weight: 0.40,
            graded: None,
        },
    ];
    assert_eq!(compute_project_score(&dims), None);
}

#[test]
fn graded_of_a_score_carries_its_grade() {
    assert_eq!(
        Graded::of(91.4),
        Graded {
            score: 91.4,
            grade: Grade::A
        }
    );
}

#[test]
fn nothing_to_grade_is_shown_as_a_dash() {
    assert_eq!(score_text(None, 1), "-");
    assert_eq!(grade_text(None), "-");
    assert_eq!(score_text(Some(Graded::of(84.25)), 1), "84.2");
    assert_eq!(score_text(Some(Graded::of(84.25)), 2), "84.25");
    assert_eq!(grade_text(Some(Graded::of(84.25))), "B+");
}

#[test]
fn grade_numeric_rank_ordering() {
    // A++ is the best (highest rank), F-- is the worst (lowest rank)
    assert!(Grade::APlusPlus.numeric_rank() > Grade::APlus.numeric_rank());
    assert!(Grade::APlus.numeric_rank() > Grade::A.numeric_rank());
    assert!(Grade::B.numeric_rank() > Grade::BMinus.numeric_rank());
    assert!(Grade::BMinus.numeric_rank() > Grade::CPlus.numeric_rank());
    assert!(Grade::F.numeric_rank() > Grade::FMinus.numeric_rank());
    assert!(Grade::FMinus.numeric_rank() > Grade::FMinusMinus.numeric_rank());
    assert_eq!(Grade::APlusPlus.numeric_rank(), 15);
    assert_eq!(Grade::FMinusMinus.numeric_rank(), 0);
}

#[test]
fn grade_parse_roundtrip() {
    let grades = [
        "A++", "A+", "A", "A-", "B+", "B", "B-", "C+", "C", "C-", "D+", "D", "D-", "F", "F-", "F--",
    ];
    for s in grades {
        let g = Grade::parse(s).expect(s);
        assert_eq!(g.as_str(), s);
    }
}

#[test]
fn grade_parse_invalid() {
    assert!(Grade::parse("X").is_err());
    assert!(Grade::parse("").is_err());
    assert!(Grade::parse("b-").is_err()); // case-sensitive
}
