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

/// Detect function boundaries and compute per-function cyclomatic complexity.
pub fn detect_functions(
    all_lines: &[String],
    code_lines: &[(usize, &str)],
    markers: &ComplexityMarkers,
) -> Vec<FunctionComplexity> {
    let bodies = detection::detect_function_bodies(all_lines, code_lines, markers);
    bodies
        .into_iter()
        .map(|body| {
            let complexity = count_complexity_for_lines(&body.code_lines, markers);
            let level = CyclomaticLevel::from_complexity(complexity);
            FunctionComplexity {
                name: body.name,
                start_line: body.start_line,
                complexity,
                level,
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "detection_test.rs"]
mod tests;
