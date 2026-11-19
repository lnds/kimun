use super::*;

#[test]
fn cognitive_weights_sum_to_one() {
    for w in [COGNITIVE, COGNITIVE_0_35] {
        let total = w.cogcom + w.dup + w.indent + w.hal + w.size;
        assert!(
            (total - 1.0).abs() < 1e-10,
            "cognitive dimension weights must sum to 1.0, got {total}"
        );
    }
}

#[test]
fn the_model_of_v0_35_weighs_file_size_at_15_percent() {
    assert_eq!(ScoringModel::Cognitive.cognitive_weights().size, 0.10);
    assert_eq!(ScoringModel::Cognitive035.cognitive_weights().size, 0.15);
}

#[test]
fn legacy_weights_sum_to_one() {
    let total = W_MI + W_CYCOM + W_DUP_LEGACY + W_INDENT_LEGACY + W_HAL_LEGACY + W_SIZE_LEGACY;
    assert!(
        (total - 1.0).abs() < 1e-10,
        "legacy dimension weights must sum to 1.0, got {total}"
    );
}

#[test]
fn file_weights_legacy_match_constants() {
    let file_sum: f64 = FILE_WEIGHTS_LEGACY.iter().map(|(w, _)| w).sum();
    let expected = W_MI + W_CYCOM + W_INDENT_LEGACY + W_HAL_LEGACY + W_SIZE_LEGACY;
    assert!(
        (file_sum - expected).abs() < 1e-10,
        "FILE_WEIGHTS_LEGACY sum should match non-dup legacy constants"
    );
}

#[test]
fn weighted_mean_all_none() {
    let files = vec![FileMetrics {
        path: "a.rs".into(),
        code_lines: 100,
        max_cognitive: None,
        mi_score: None,
        mi_vs: None,
        max_complexity: None,
        indent_stddev: None,
        halstead_effort: None,
    }];
    let result = weighted_mean(&files, 100, |_| None);
    assert_eq!(result, None, "no file has the metric → no score");
}

#[test]
fn weighted_mean_total_loc_zero() {
    let files: Vec<FileMetrics> = vec![];
    let result = weighted_mean(&files, 0, |_| Some(80.0));
    assert_eq!(result, None, "total_loc=0 → no score");
}

#[test]
fn weighted_mean_single_file() {
    let files = vec![FileMetrics {
        path: "a.rs".into(),
        code_lines: 100,
        max_cognitive: Some(5),
        mi_score: None,
        mi_vs: None,
        max_complexity: None,
        indent_stddev: Some(1.0),
        halstead_effort: Some(1000.0),
    }];
    let result = weighted_mean(&files, 100, |f| f.max_cognitive.map(|c| c as f64)).unwrap();
    assert!(
        (result - 5.0).abs() < 0.01,
        "single file → same value, got {result}"
    );
}

#[test]
fn weighted_mean_loc_weighted() {
    let files = vec![
        FileMetrics {
            path: "small.rs".into(),
            code_lines: 10,
            max_cognitive: None,
            mi_score: None,
            mi_vs: None,
            max_complexity: None,
            indent_stddev: None,
            halstead_effort: None,
        },
        FileMetrics {
            path: "big.rs".into(),
            code_lines: 90,
            max_cognitive: None,
            mi_score: None,
            mi_vs: None,
            max_complexity: None,
            indent_stddev: None,
            halstead_effort: None,
        },
    ];
    let result = weighted_mean(&files, 100, |f| {
        if f.code_lines == 10 {
            Some(100.0)
        } else {
            Some(50.0)
        }
    })
    .unwrap();
    // (100*10 + 50*90) / 100 = (1000 + 4500) / 100 = 55
    assert!(
        (result - 55.0).abs() < 0.01,
        "LOC-weighted → 55, got {result}"
    );
}

#[test]
fn build_cognitive_dimensions_count() {
    let files = vec![FileMetrics {
        path: "a.rs".into(),
        code_lines: 100,
        max_cognitive: Some(5),
        mi_score: None,
        mi_vs: None,
        max_complexity: None,
        indent_stddev: Some(1.0),
        halstead_effort: Some(1000.0),
    }];
    let dims = build_dimensions(&files, 100, 5.0, &ScoringModel::Cognitive);
    assert_eq!(dims.len(), 5, "cognitive model should have 5 dimensions");
    assert_eq!(dims[0].name, "Cognitive Complexity");
}

#[test]
fn build_legacy_dimensions_count() {
    let files = vec![FileMetrics {
        path: "a.rs".into(),
        code_lines: 100,
        max_cognitive: None,
        mi_score: Some(80.0),
        mi_vs: None,
        max_complexity: Some(5),
        indent_stddev: Some(1.0),
        halstead_effort: Some(1000.0),
    }];
    let dims = build_dimensions(&files, 100, 5.0, &ScoringModel::Legacy);
    assert_eq!(dims.len(), 6, "legacy model should have 6 dimensions");
    assert_eq!(dims[0].name, "Maintainability Index");
    assert_eq!(dims[1].name, "Cyclomatic Complexity");
}

#[test]
fn score_file_cognitive_produces_issues() {
    let f = FileMetrics {
        path: "complex.rs".into(),
        code_lines: 2000,
        max_cognitive: Some(50),
        mi_score: None,
        mi_vs: None,
        max_complexity: None,
        indent_stddev: Some(4.0),
        halstead_effort: Some(2_000_000.0),
    };
    let fs = score_file(&f, &ScoringModel::Cognitive);
    assert!(!fs.issues.is_empty(), "complex file should have issues");
    assert!(fs.score < 50.0, "complex file should score low");
}

#[test]
fn score_file_legacy_produces_issues() {
    let f = FileMetrics {
        path: "complex.rs".into(),
        code_lines: 2000,
        max_cognitive: None,
        mi_score: Some(20.0),
        mi_vs: None,
        max_complexity: Some(50),
        indent_stddev: Some(4.0),
        halstead_effort: Some(2_000_000.0),
    };
    let fs = score_file(&f, &ScoringModel::Legacy);
    assert!(!fs.issues.is_empty(), "complex file should have issues");
    assert!(fs.score < 50.0, "complex file should score low");
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
    assert_eq!(compute_project_score(&[]), None);
}

#[test]
fn a_project_with_an_ungraded_dimension_is_scored_on_the_others() {
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
    assert_eq!(compute_project_score(&dims), Some(80.0));
}

// ── what was not measured is left out ───────────────────────────────────

/// A file with every metric, to change with struct update syntax.
fn measured(path: &str, code_lines: usize) -> FileMetrics {
    FileMetrics {
        path: path.into(),
        code_lines,
        max_cognitive: Some(12),
        mi_score: Some(62.0),
        mi_vs: Some(55.0),
        max_complexity: Some(9),
        indent_stddev: Some(1.7),
        halstead_effort: Some(30_000.0),
    }
}

/// A dimension with the score it has, if any.
fn dimension(name: &'static str, weight: f64, score: Option<f64>) -> DimensionScore {
    DimensionScore {
        name,
        weight,
        graded: score.map(Graded::of),
    }
}

/// The names of the dimensions without a score.
fn unscored(dims: &[DimensionScore]) -> Vec<&str> {
    dims.iter()
        .filter(|d| d.graded.is_none())
        .map(|d| d.name)
        .collect()
}

/// The weighted mean of the scores the dimensions have, worked out apart.
fn mean_of_the_graded(dims: &[DimensionScore]) -> f64 {
    let graded: Vec<(f64, f64)> = dims
        .iter()
        .filter_map(|d| Some((d.graded?.score, d.weight)))
        .collect();
    let sum: f64 = graded.iter().map(|(s, w)| s * w).sum();
    sum / graded.iter().map(|(_, w)| w).sum::<f64>()
}

/// The dimensions and the project score of the files, with 5% duplication.
fn dimensions_and_score(files: &[FileMetrics], model: &ScoringModel) -> (Vec<DimensionScore>, f64) {
    let total_loc = files.iter().map(|f| f.code_lines).sum();
    let dims = build_dimensions(files, total_loc, 5.0, model);
    let score = compute_project_score(&dims).expect("some dimension is graded");
    (dims, score)
}

#[test]
fn a_dimension_no_file_measures_has_no_score() {
    let files = [
        FileMetrics {
            halstead_effort: None,
            ..measured("a.rb", 40)
        },
        FileMetrics {
            halstead_effort: None,
            ..measured("b.rb", 60)
        },
    ];
    let (dims, score) = dimensions_and_score(&files, &ScoringModel::Cognitive);
    assert_eq!(dims.len(), 5);
    assert_eq!(unscored(&dims), ["Halstead Effort"]);
    assert!((score - mean_of_the_graded(&dims)).abs() < 1e-9, "{score}");
}

#[test]
fn the_same_rule_leaves_out_the_mi_and_halstead_of_the_legacy_model() {
    let files = [
        FileMetrics {
            mi_score: None,
            halstead_effort: None,
            ..measured("a.h", 40)
        },
        FileMetrics {
            mi_score: None,
            halstead_effort: None,
            ..measured("b.h", 60)
        },
    ];
    let (dims, score) = dimensions_and_score(&files, &ScoringModel::Legacy);
    assert_eq!(dims.len(), 6);
    assert_eq!(
        unscored(&dims),
        ["Maintainability Index", "Halstead Effort"]
    );
    assert!((score - mean_of_the_graded(&dims)).abs() < 1e-9, "{score}");
}

#[test]
fn the_experimental_mi_has_no_score_when_no_file_has_one() {
    let files = [
        FileMetrics {
            mi_vs: None,
            ..measured("a.rb", 40)
        },
        FileMetrics {
            mi_vs: None,
            ..measured("b.rb", 60)
        },
    ];
    let (dims, score) = dimensions_and_score(&files, &ScoringModel::ExperimentalMi);
    assert_eq!(dims[3].name, "Maintainability Index");
    assert_eq!(unscored(&dims), ["Maintainability Index"]);
    assert!((score - mean_of_the_graded(&dims)).abs() < 1e-9, "{score}");
}

#[test]
fn a_file_is_scored_on_the_metrics_it_has() {
    let f = FileMetrics {
        halstead_effort: None,
        ..measured("types.ts", 120)
    };
    let w = COGNITIVE;
    let cognitive = score_file(&f, &ScoringModel::Cognitive).score;
    let left_out = (normalize_cognitive(12) * w.cogcom
        + normalize_indent(1.7) * w.indent
        + normalize_file_size(120) * w.size)
        / (w.cogcom + w.indent + w.size);
    assert!((cognitive - left_out).abs() < 1e-9, "{cognitive}");
    let halfway = (normalize_cognitive(12) * w.cogcom
        + normalize_indent(1.7) * w.indent
        + 50.0 * w.hal
        + normalize_file_size(120) * w.size)
        / w.file_sum();
    assert!(
        (cognitive - halfway).abs() > 1.0,
        "{cognitive} is {halfway}"
    );

    let f = FileMetrics {
        mi_score: None,
        halstead_effort: None,
        ..measured("cfg.h", 120)
    };
    let legacy = score_file(&f, &ScoringModel::Legacy).score;
    let left_out = (normalize_complexity(9) * W_CYCOM
        + normalize_indent(1.7) * W_INDENT_LEGACY
        + normalize_file_size(120) * W_SIZE_LEGACY)
        / (W_CYCOM + W_INDENT_LEGACY + W_SIZE_LEGACY);
    assert!((legacy - left_out).abs() < 1e-9, "{legacy}");
    let halfway = (50.0 * W_MI
        + normalize_complexity(9) * W_CYCOM
        + normalize_indent(1.7) * W_INDENT_LEGACY
        + 50.0 * W_HAL_LEGACY
        + normalize_file_size(120) * W_SIZE_LEGACY)
        / FILE_WEIGHTS_LEGACY.iter().map(|(w, _)| w).sum::<f64>();
    assert!((legacy - halfway).abs() > 1.0, "{legacy} is {halfway}");
}

#[test]
fn a_file_with_every_metric_scores_as_before() {
    let f = measured("lib.rs", 120);
    let w = COGNITIVE;
    assert_eq!(
        score_file(&f, &ScoringModel::Cognitive).score,
        (normalize_cognitive(12) * w.cogcom
            + normalize_indent(1.7) * w.indent
            + normalize_halstead(30_000.0, 120) * w.hal
            + normalize_file_size(120) * w.size)
            / w.file_sum()
    );
    assert_eq!(
        score_file(&f, &ScoringModel::Legacy).score,
        (normalize_mi(62.0) * W_MI
            + normalize_complexity(9) * W_CYCOM
            + normalize_indent(1.7) * W_INDENT_LEGACY
            + normalize_halstead(30_000.0, 120) * W_HAL_LEGACY
            + normalize_file_size(120) * W_SIZE_LEGACY)
            / FILE_WEIGHTS_LEGACY.iter().map(|(w, _)| w).sum::<f64>()
    );
}

#[test]
fn a_project_with_every_dimension_scores_as_before() {
    // 0.3 + 0.6 + 0.1 is just under 1.0: the plain sum is not divided by it.
    let dims = [
        dimension("A", 0.3, Some(81.3)),
        dimension("B", 0.6, Some(92.7)),
        dimension("C", 0.1, Some(64.9)),
    ];
    assert_eq!(
        compute_project_score(&dims),
        Some(81.3 * 0.3 + 92.7 * 0.6 + 64.9 * 0.1)
    );
}

#[test]
fn a_mean_leaves_out_the_terms_without_a_score() {
    assert_eq!(measured_mean(&[], 1.0), None);
    assert_eq!(measured_mean(&[(None, 0.6), (None, 0.4)], 1.0), None);
    assert_eq!(
        measured_mean(&[(Some(80.0), 0.6), (None, 0.4)], 1.0),
        Some(80.0)
    );
    assert_eq!(
        measured_mean(&[(Some(80.0), 0.5), (Some(60.0), 0.5)], 2.0),
        Some(35.0)
    );
}
