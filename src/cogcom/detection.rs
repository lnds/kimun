//! Function detection for cognitive complexity analysis.
//!
//! Delegates to the shared `detection` module for function boundary detection,
//! then computes per-function cognitive complexity.

use crate::detection;

use super::analyzer::{CognitiveLevel, FunctionCognitive, count_cognitive_for_lines};
use super::markers::CognitiveMarkers;

/// Extract the function name from a declaration line.
/// Delegates to the shared detection module. Used by tests.
#[cfg(test)]
pub fn extract_function_name(trimmed: &str, markers: &CognitiveMarkers) -> String {
    detection::extract_function_name(trimmed, markers)
}

/// Detect function boundaries and compute per-function cognitive complexity.
pub fn detect_functions(
    all_lines: &[String],
    code_lines: &[(usize, &str)],
    markers: &CognitiveMarkers,
) -> Vec<FunctionCognitive> {
    let bodies = detection::detect_function_bodies(all_lines, code_lines, markers);
    bodies
        .into_iter()
        .map(|body| {
            let complexity = count_cognitive_for_lines(&body.code_lines, markers);
            let level = CognitiveLevel::from_complexity(complexity);
            FunctionCognitive {
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
