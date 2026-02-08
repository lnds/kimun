use serde::Serialize;

use crate::loc::counter::LineKind;

/// Qualitative complexity classification based on indentation stddev.
///
/// Thresholds are initial heuristics inspired by Adam Tornhill's emphasis on
/// structural variance as a complexity signal ("Your Code as a Crime Scene").
/// They may need tuning based on real-world corpus analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplexityLevel {
    Low,
    Moderate,
    High,
    VeryHigh,
}

impl ComplexityLevel {
    pub fn from_stddev(stddev: f64) -> Self {
        if stddev < 2.0 {
            Self::Low
        } else if stddev < 4.0 {
            Self::Moderate
        } else if stddev < 6.0 {
            Self::High
        } else {
            Self::VeryHigh
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Moderate => "Moderate",
            Self::High => "High",
            Self::VeryHigh => "Very High",
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndentMetrics {
    pub code_lines: usize,
    pub stddev: f64,
    pub max_depth: usize,
    pub complexity: ComplexityLevel,
}

/// Count leading whitespace in a line, normalizing tabs to `tab_width` spaces.
pub fn indent_depth(line: &str, tab_width: usize) -> usize {
    let mut depth = 0;
    for ch in line.chars() {
        match ch {
            ' ' => depth += 1,
            '\t' => depth += tab_width,
            _ => break,
        }
    }
    depth
}

/// Calculate indentation metrics from lines and their classifications.
/// Only considers `LineKind::Code` lines.
pub fn analyze(lines: &[String], kinds: &[LineKind], tab_width: usize) -> Option<IndentMetrics> {
    let depths: Vec<usize> = lines
        .iter()
        .zip(kinds)
        .filter(|(_, k)| **k == LineKind::Code)
        .map(|(line, _)| indent_depth(line, tab_width))
        .collect();

    if depths.is_empty() {
        return None;
    }

    let max_depth = *depths.iter().max().unwrap();
    let stddev = calculate_stddev(&depths);
    let complexity = ComplexityLevel::from_stddev(stddev);

    Some(IndentMetrics {
        code_lines: depths.len(),
        stddev,
        max_depth,
        complexity,
    })
}

fn calculate_stddev(values: &[usize]) -> f64 {
    let n = values.len() as f64;
    if n <= 1.0 {
        return 0.0;
    }
    let mean = values.iter().sum::<usize>() as f64 / n;
    let variance = values
        .iter()
        .map(|&v| (v as f64 - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indent_depth_spaces() {
        assert_eq!(indent_depth("    code", 4), 4);
        assert_eq!(indent_depth("        code", 4), 8);
    }

    #[test]
    fn indent_depth_tabs() {
        assert_eq!(indent_depth("\tcode", 4), 4);
        assert_eq!(indent_depth("\t\tcode", 4), 8);
    }

    #[test]
    fn indent_depth_mixed() {
        assert_eq!(indent_depth("\t  code", 4), 6);
    }

    #[test]
    fn indent_depth_no_indent() {
        assert_eq!(indent_depth("code", 4), 0);
    }

    #[test]
    fn indent_depth_empty_line() {
        assert_eq!(indent_depth("", 4), 0);
    }

    #[test]
    fn analyze_basic_file() {
        let lines: Vec<String> = vec![
            "fn main() {",
            "    let x = 1;",
            "    if x > 0 {",
            "        println!(\"hi\");",
            "    }",
            "}",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let kinds = vec![LineKind::Code; 6];

        let m = analyze(&lines, &kinds, 4).unwrap();
        // depths = [0, 4, 4, 8, 4, 0] → mean=3.33, max=8
        assert_eq!(m.code_lines, 6);
        assert_eq!(m.max_depth, 8);
        assert!(m.stddev > 0.0);
    }

    #[test]
    fn analyze_filters_non_code() {
        let lines: Vec<String> = vec!["// comment", "", "fn main() {", "    let x = 1;", "}"]
            .into_iter()
            .map(String::from)
            .collect();
        let kinds = vec![
            LineKind::Comment,
            LineKind::Blank,
            LineKind::Code,
            LineKind::Code,
            LineKind::Code,
        ];

        let m = analyze(&lines, &kinds, 4).unwrap();
        assert_eq!(m.code_lines, 3);
    }

    #[test]
    fn analyze_empty_returns_none() {
        let m = analyze(&[], &[], 4);
        assert!(m.is_none());
    }

    #[test]
    fn analyze_all_comments_returns_none() {
        let lines = vec!["// comment".to_string()];
        let kinds = vec![LineKind::Comment];
        assert!(analyze(&lines, &kinds, 4).is_none());
    }

    #[test]
    fn analyze_uniform_indent_zero_stddev() {
        let lines: Vec<String> = vec!["    a();", "    b();", "    c();"]
            .into_iter()
            .map(String::from)
            .collect();
        let kinds = vec![LineKind::Code; 3];

        let m = analyze(&lines, &kinds, 4).unwrap();
        assert!((m.stddev - 0.0).abs() < 0.001);
        assert_eq!(m.max_depth, 4);
    }

    #[test]
    fn stddev_calculation() {
        // values [0, 4, 8] → mean=4, variance=((16+0+16)/2)=16, sd=4.0 (Bessel's correction)
        let m = calculate_stddev(&[0, 4, 8]);
        assert!((m - 4.0).abs() < 0.01);
    }

    #[test]
    fn stddev_single_value_is_zero() {
        assert_eq!(calculate_stddev(&[5]), 0.0);
    }

    #[test]
    fn stddev_empty_is_zero() {
        assert_eq!(calculate_stddev(&[]), 0.0);
    }

    #[test]
    fn complexity_level_thresholds() {
        assert_eq!(ComplexityLevel::from_stddev(0.0), ComplexityLevel::Low);
        assert_eq!(ComplexityLevel::from_stddev(1.99), ComplexityLevel::Low);
        assert_eq!(ComplexityLevel::from_stddev(2.0), ComplexityLevel::Moderate);
        assert_eq!(
            ComplexityLevel::from_stddev(3.99),
            ComplexityLevel::Moderate
        );
        assert_eq!(ComplexityLevel::from_stddev(4.0), ComplexityLevel::High);
        assert_eq!(ComplexityLevel::from_stddev(5.99), ComplexityLevel::High);
        assert_eq!(ComplexityLevel::from_stddev(6.0), ComplexityLevel::VeryHigh);
        assert_eq!(
            ComplexityLevel::from_stddev(10.0),
            ComplexityLevel::VeryHigh
        );
    }

    #[test]
    fn complexity_level_display() {
        assert_eq!(ComplexityLevel::Low.as_str(), "Low");
        assert_eq!(ComplexityLevel::Moderate.as_str(), "Moderate");
        assert_eq!(ComplexityLevel::High.as_str(), "High");
        assert_eq!(ComplexityLevel::VeryHigh.as_str(), "Very High");
    }

    #[test]
    fn complexity_level_serde() {
        assert_eq!(
            serde_json::to_string(&ComplexityLevel::VeryHigh).unwrap(),
            "\"very_high\""
        );
        assert_eq!(
            serde_json::to_string(&ComplexityLevel::Low).unwrap(),
            "\"low\""
        );
    }

    #[test]
    fn analyze_includes_complexity() {
        let lines: Vec<String> = vec!["    a();", "    b();", "    c();"]
            .into_iter()
            .map(String::from)
            .collect();
        let kinds = vec![LineKind::Code; 3];
        let m = analyze(&lines, &kinds, 4).unwrap();
        assert_eq!(m.complexity, ComplexityLevel::Low);
    }
}
