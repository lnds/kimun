use super::*;

#[test]
fn cognitive_weights_sum_to_one() {
    let total = W_COGCOM + W_DUP + W_INDENT + W_HAL + W_SIZE;
    assert!(
        (total - 1.0).abs() < 1e-10,
        "cognitive dimension weights must sum to 1.0, got {total}"
    );
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
fn file_weights_match_cognitive_constants() {
    let file_sum: f64 = FILE_WEIGHTS.iter().map(|(w, _)| w).sum();
    let expected = W_COGCOM + W_INDENT + W_HAL + W_SIZE;
    assert!(
        (file_sum - expected).abs() < 1e-10,
        "FILE_WEIGHTS sum should match non-dup cognitive constants"
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
        max_complexity: None,
        indent_stddev: None,
        halstead_effort: None,
    }];
    let result = weighted_mean(&files, 100, |_| None);
    assert!((result - 0.0).abs() < 0.01, "all None → 0, got {result}");
}

#[test]
fn weighted_mean_total_loc_zero() {
    let files: Vec<FileMetrics> = vec![];
    let result = weighted_mean(&files, 0, |_| Some(80.0));
    assert!((result - 0.0).abs() < 0.01, "total_loc=0 → 0, got {result}");
}

#[test]
fn weighted_mean_single_file() {
    let files = vec![FileMetrics {
        path: "a.rs".into(),
        code_lines: 100,
        max_cognitive: Some(5),
        mi_score: None,
        max_complexity: None,
        indent_stddev: Some(1.0),
        halstead_effort: Some(1000.0),
    }];
    let result = weighted_mean(&files, 100, |f| f.max_cognitive.map(|c| c as f64));
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
            max_complexity: None,
            indent_stddev: None,
            halstead_effort: None,
        },
        FileMetrics {
            path: "big.rs".into(),
            code_lines: 90,
            max_cognitive: None,
            mi_score: None,
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
    });
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
        max_complexity: Some(50),
        indent_stddev: Some(4.0),
        halstead_effort: Some(2_000_000.0),
    };
    let fs = score_file(&f, &ScoringModel::Legacy);
    assert!(!fs.issues.is_empty(), "complex file should have issues");
    assert!(fs.score < 50.0, "complex file should score low");
}
