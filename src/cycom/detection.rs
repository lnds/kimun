//! Function detection for cyclomatic complexity analysis.
//!
//! Delegates to the shared `detection` module for function boundary detection,
//! then computes per-function cyclomatic complexity.

use crate::detection;

use super::analyzer::{CyclomaticLevel, FunctionComplexity, count_complexity_for_lines};
use super::markers::ComplexityMarkers;

/// Extract the function name from a declaration line.
/// Delegates to the shared detection module. Used by tests.
#[cfg(test)]
pub fn extract_function_name(trimmed: &str, markers: &ComplexityMarkers) -> String {
    detection::extract_function_name(trimmed, markers)
}

/// Detect function boundaries and compute per-function cyclomatic
/// complexity. The code outside the functions comes first as `<file>`
/// when the file has no function or that code has a decision point.
pub fn detect_functions(
    all_lines: &[String],
    code_lines: &[(usize, &str)],
    markers: &ComplexityMarkers,
) -> Vec<FunctionComplexity> {
    let bodies = detection::detect_function_bodies(all_lines, code_lines, markers);
    let outside = detection::outside_functions(code_lines, &bodies);
    let outside = count_complexity_for_lines(&outside, markers);
    let file_level =
        (bodies.is_empty() || outside > 1).then_some(("<file>".to_string(), 1, outside));
    let functions = bodies.into_iter().map(|body| {
        let complexity = count_complexity_for_lines(&body.code_lines, markers);
        (body.name, body.start_line, complexity)
    });
    file_level
        .into_iter()
        .chain(functions)
        .map(|(name, start_line, complexity)| FunctionComplexity {
            name,
            start_line,
            complexity,
            level: CyclomaticLevel::from_complexity(complexity),
        })
        .collect()
}

#[cfg(test)]
#[path = "detection_test.rs"]
mod tests;
