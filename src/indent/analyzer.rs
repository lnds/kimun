use serde::Serialize;

use crate::loc::counter::LineKind;

/// Qualitative complexity classification based on indentation stddev
/// measured in logical indentation levels (4 spaces = 1 level).
///
/// Thresholds are initial heuristics inspired by Adam Tornhill's emphasis on
/// structural variance as a complexity signal ("Your Code as a Crime Scene",
/// Chapter 6). Tornhill's example: Configuration.java has sd=1.63 which he
/// calls "not too bad". They may need tuning based on real-world corpus analysis.
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
        if stddev < 1.0 {
            Self::Low
        } else if stddev < 1.5 {
            Self::Moderate
        } else if stddev < 2.0 {
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
    pub total_indent: usize,
    pub complexity: ComplexityLevel,
}

/// Count leading whitespace as logical indentation levels.
/// Each tab counts as one level; spaces are accumulated and divided by `tab_width`.
/// This matches Adam Tornhill's `complexity_analysis.py` approach where
/// "four spaces or one tab counts as one logical indentation."
pub fn indent_depth(line: &str, tab_width: usize) -> usize {
    let mut spaces = 0;
    for ch in line.chars() {
        match ch {
            ' ' => spaces += 1,
            '\t' => spaces += tab_width,
            _ => break,
        }
    }
    spaces / tab_width
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
    let total_indent: usize = depths.iter().sum();
    let stddev = calculate_stddev(&depths);
    let complexity = ComplexityLevel::from_stddev(stddev);

    Some(IndentMetrics {
        code_lines: depths.len(),
        stddev,
        max_depth,
        total_indent,
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
#[path = "analyzer_test.rs"]
mod tests;
