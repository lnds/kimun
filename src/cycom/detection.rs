/// Function detection and complexity analysis for cyclomatic metrics.
///
/// Detects function boundaries using either brace-scoped languages
/// (C, Rust, Java, etc.) or indent-scoped languages (Python, Ruby),
/// then counts control-flow keywords per function body.
use crate::util::mask_strings;

use super::markers::ComplexityMarkers;

/// Control-flow keywords that should NOT be treated as function definitions
/// in the C-family heuristic.
const CONTROL_KEYWORDS: &[&str] = &[
    "if", "for", "while", "switch", "else", "do", "catch", "return", "case",
];

/// Check whether a line is a function declaration, using explicit markers
/// or the C-family heuristic as fallback.
fn is_function_declaration(trimmed: &str, markers: &ComplexityMarkers) -> bool {
    if !markers.function_markers.is_empty() {
        markers.function_markers.iter().any(|m| trimmed.contains(m))
    } else {
        is_c_family_function(trimmed)
    }
}

/// Heuristic for C/C++/Java/C# function detection: line contains '(' and
/// ends with '{' or ')', and the first word is NOT a control keyword.
///
/// Known limitations:
/// - Multiline declarations where '{' is on a separate line are missed.
/// - Function pointers (e.g., `void (*fp)(int)`) may be misdetected.
/// - C++ constructor initializer lists are not handled.
/// - Macros that look like functions (e.g., `DEFINE_TEST(name)`) are
///   treated as functions.
fn is_c_family_function(trimmed: &str) -> bool {
    if !trimmed.contains('(') {
        return false;
    }
    if !(trimmed.ends_with('{') || trimmed.ends_with(')')) {
        return false;
    }

    let first_word = trimmed.split_whitespace().next().unwrap_or("");
    let first_word = first_word.trim_start_matches('*');
    !CONTROL_KEYWORDS.contains(&first_word)
}

/// Extract the function name from a declaration line using language-specific
/// markers or the C-family heuristic (token before the first `(`).
pub fn extract_function_name(trimmed: &str, markers: &ComplexityMarkers) -> String {
    for marker in markers.function_markers {
        if let Some(pos) = trimmed.find(marker) {
            let after = &trimmed[pos + marker.len()..];
            let name: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return name;
            }
        }
    }

    // C-family heuristic: name is the token before '('
    if let Some(paren_pos) = trimmed.find('(') {
        let before = trimmed[..paren_pos].trim();
        if let Some(name) = before.split_whitespace().next_back() {
            let name = name.trim_start_matches('*');
            if !name.is_empty() {
                return name.to_string();
            }
        }
    }

    "<anonymous>".to_string()
}

/// Starting from `code_lines[start]`, collect all lines belonging to the
/// function body by tracking brace depth (string/char literals are masked).
/// Returns `(func_code_lines, end_index)` where `end_index` is the index
/// of the closing brace line in `code_lines`.
fn find_function_body<'a>(
    code_lines: &[(usize, &'a str)],
    start: usize,
    line_comments: &[&str],
) -> (Vec<(usize, &'a str)>, usize) {
    let mut brace_depth: isize = 0;
    let mut found_open = false;
    let mut func_code_lines: Vec<(usize, &str)> = Vec::new();
    let mut j = start;

    while j < code_lines.len() {
        let (jidx, jline) = code_lines[j];
        func_code_lines.push((jidx, jline));

        let masked = mask_strings(jline, line_comments);
        for ch in masked.bytes() {
            if ch == b'{' {
                brace_depth += 1;
                found_open = true;
            } else if ch == b'}' {
                brace_depth -= 1;
            }
        }

        if found_open && brace_depth == 0 {
            break;
        }
        j += 1;
    }

    (func_code_lines, j)
}

/// Count the indentation level of a line in spaces (tabs count as 4).
fn indent_level(line: &str) -> usize {
    let mut spaces = 0;
    for ch in line.chars() {
        match ch {
            ' ' => spaces += 1,
            '\t' => spaces += 4,
            _ => break,
        }
    }
    spaces
}

use super::analyzer::{CyclomaticLevel, FunctionComplexity, count_complexity_for_lines};

/// Detect function boundaries and compute per-function cyclomatic complexity.
/// Delegates to brace-scoped or indent-scoped detection based on the language.
pub fn detect_functions(
    all_lines: &[String],
    code_lines: &[(usize, &str)],
    markers: &ComplexityMarkers,
) -> Vec<FunctionComplexity> {
    let mut functions = Vec::new();

    if markers.brace_scoped {
        detect_brace_scoped(code_lines, markers, &mut functions);
    } else {
        detect_indent_scoped(all_lines, code_lines, markers, &mut functions);
    }

    functions
}

/// Walk code lines, find function declarations by markers or C-family
/// heuristic, track brace depth to determine body extent, count complexity.
fn detect_brace_scoped(
    code_lines: &[(usize, &str)],
    markers: &ComplexityMarkers,
    functions: &mut Vec<FunctionComplexity>,
) {
    let mut i = 0;
    while i < code_lines.len() {
        let (line_idx, line) = code_lines[i];
        let trimmed = line.trim();

        if is_function_declaration(trimmed, markers) {
            let name = extract_function_name(trimmed, markers);
            let (func_code_lines, end) = find_function_body(code_lines, i, markers.line_comments);
            let complexity = count_complexity_for_lines(&func_code_lines, markers);
            let level = CyclomaticLevel::from_complexity(complexity);
            functions.push(FunctionComplexity {
                name,
                start_line: line_idx + 1,
                complexity,
                level,
            });
            i = end + 1;
        } else {
            i += 1;
        }
    }
}

/// Walk code lines for indent-scoped languages (Python, Ruby), using
/// indentation level to determine where function bodies end.
fn detect_indent_scoped(
    all_lines: &[String],
    code_lines: &[(usize, &str)],
    markers: &ComplexityMarkers,
    functions: &mut Vec<FunctionComplexity>,
) {
    let mut i = 0;
    while i < code_lines.len() {
        let (line_idx, line) = code_lines[i];
        let trimmed = line.trim();

        let is_function = markers
            .function_markers
            .iter()
            .any(|m| trimmed.starts_with(m));

        if is_function {
            let name = extract_function_name(trimmed, markers);
            let start_line = line_idx + 1;
            let base_indent = indent_level(line);

            let mut func_code_lines: Vec<(usize, &str)> = vec![(line_idx, line)];
            let mut j = i + 1;

            while j < code_lines.len() {
                let (jidx, jline) = code_lines[j];
                let jline_full = &all_lines[jidx];
                if indent_level(jline_full) <= base_indent && !jline.trim().is_empty() {
                    break;
                }
                func_code_lines.push((jidx, jline));
                j += 1;
            }

            let complexity = count_complexity_for_lines(&func_code_lines, markers);
            let level = CyclomaticLevel::from_complexity(complexity);
            functions.push(FunctionComplexity {
                name,
                start_line,
                complexity,
                level,
            });

            i = j;
        } else {
            i += 1;
        }
    }
}

#[cfg(test)]
#[path = "detection_test.rs"]
mod tests;
