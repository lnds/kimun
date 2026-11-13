//! Function detection for cognitive complexity analysis.
//!
//! Delegates to the shared `detection` module for function boundary detection,
//! then computes per-function cognitive complexity.

use crate::detection;

use super::analyzer::{
    CognitiveLevel, FunctionCognitive, count_cognitive_for_lines, count_cognitive_outside,
};
use super::markers::CognitiveMarkers;

/// Extract the function name from a declaration line.
/// Delegates to the shared detection module. Used by tests.
#[cfg(test)]
pub fn extract_function_name(trimmed: &str, markers: &CognitiveMarkers) -> String {
    detection::extract_function_name(trimmed, markers)
}

/// Detect function boundaries and compute per-function cognitive
/// complexity. The code outside the functions comes first as `<file>`
/// when the file has no function or that code adds complexity.
pub fn detect_functions(
    all_lines: &[String],
    code_lines: &[(usize, &str)],
    markers: &CognitiveMarkers,
) -> Vec<FunctionCognitive> {
    let bodies = detection::detect_function_bodies(all_lines, code_lines, markers);
    let outside = detection::outside_functions(code_lines, &bodies);
    let outside = count_cognitive_outside(&outside, markers);
    let file_level =
        (bodies.is_empty() || outside > 0).then_some(("<file>".to_string(), 1, outside));
    let functions = bodies.into_iter().map(|body| {
        let complexity = count_cognitive_for_lines(&body.code_lines, markers);
        (body.name, body.start_line, complexity)
    });
    file_level
        .into_iter()
        .chain(functions)
        .map(|(name, start_line, complexity)| FunctionCognitive {
            name,
            start_line,
            complexity,
            level: CognitiveLevel::from_complexity(complexity),
        })
        .collect()
}

#[cfg(test)]
#[path = "detection_test.rs"]
mod tests;
