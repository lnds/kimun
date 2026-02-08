use serde::Serialize;

use crate::loc::counter::LineKind;

use super::markers::ComplexityMarkers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CyclomaticLevel {
    Simple,
    Moderate,
    Complex,
    HighlyComplex,
    Extreme,
}

impl CyclomaticLevel {
    pub fn from_complexity(c: usize) -> Self {
        match c {
            0..=5 => Self::Simple,
            6..=10 => Self::Moderate,
            11..=20 => Self::Complex,
            21..=50 => Self::HighlyComplex,
            _ => Self::Extreme,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Moderate => "moderate",
            Self::Complex => "complex",
            Self::HighlyComplex => "highly complex",
            Self::Extreme => "extreme",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionComplexity {
    pub name: String,
    pub start_line: usize, // 1-based
    pub complexity: usize,
    pub level: CyclomaticLevel,
}

#[derive(Debug, Clone)]
pub struct FileComplexity {
    pub functions: Vec<FunctionComplexity>,
    pub total_complexity: usize,
    pub max_complexity: usize,
    pub avg_complexity: f64,
    pub level: CyclomaticLevel,
}

/// Control-flow keywords that should NOT be treated as function definitions
/// in the C-family heuristic.
const CONTROL_KEYWORDS: &[&str] = &[
    "if", "for", "while", "switch", "else", "do", "catch", "return", "case",
];

pub fn analyze(
    lines: &[String],
    kinds: &[LineKind],
    markers: &ComplexityMarkers,
) -> Option<FileComplexity> {
    if lines.is_empty() || kinds.is_empty() {
        return None;
    }

    // Build list of (line_index, line_text) for code lines only
    let code_lines: Vec<(usize, &str)> = lines
        .iter()
        .zip(kinds)
        .enumerate()
        .filter(|(_, (_, k))| **k == LineKind::Code)
        .map(|(i, (line, _))| (i, line.as_str()))
        .collect();

    if code_lines.is_empty() {
        return None;
    }

    let functions = detect_functions(lines, &code_lines, markers);

    if functions.is_empty() {
        // Treat entire file as one implicit function
        let complexity = count_complexity_for_lines(&code_lines, markers);
        let level = CyclomaticLevel::from_complexity(complexity);
        return Some(FileComplexity {
            functions: vec![FunctionComplexity {
                name: "<file>".to_string(),
                start_line: 1,
                complexity,
                level,
            }],
            total_complexity: complexity,
            max_complexity: complexity,
            avg_complexity: complexity as f64,
            level,
        });
    }

    let total: usize = functions.iter().map(|f| f.complexity).sum();
    let max = functions.iter().map(|f| f.complexity).max().unwrap_or(0);
    let avg = total as f64 / functions.len() as f64;
    let level = CyclomaticLevel::from_complexity(max);

    Some(FileComplexity {
        functions,
        total_complexity: total,
        max_complexity: max,
        avg_complexity: avg,
        level,
    })
}

fn detect_functions(
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

fn detect_brace_scoped(
    code_lines: &[(usize, &str)],
    markers: &ComplexityMarkers,
    functions: &mut Vec<FunctionComplexity>,
) {
    let mut i = 0;
    while i < code_lines.len() {
        let (line_idx, line) = code_lines[i];
        let trimmed = line.trim();

        let is_function = if !markers.function_markers.is_empty() {
            markers.function_markers.iter().any(|m| trimmed.contains(m))
        } else {
            is_c_family_function(trimmed)
        };

        if is_function {
            let name = extract_function_name(trimmed, markers);
            let start_line = line_idx + 1; // 1-based

            // Find the opening brace
            let mut brace_depth: isize = 0;
            let mut found_open = false;
            let mut func_code_lines: Vec<(usize, &str)> = Vec::new();
            let mut j = i;

            while j < code_lines.len() {
                let (jidx, jline) = code_lines[j];
                func_code_lines.push((jidx, jline));

                for ch in jline.chars() {
                    if ch == '{' {
                        brace_depth += 1;
                        found_open = true;
                    } else if ch == '}' {
                        brace_depth -= 1;
                    }
                }

                if found_open && brace_depth <= 0 {
                    break;
                }
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

            i = j + 1;
        } else {
            i += 1;
        }
    }
}

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

fn is_c_family_function(trimmed: &str) -> bool {
    // Heuristic: line contains '(' and ends with '{' or ')',
    // and the first word is NOT a control keyword.
    if !trimmed.contains('(') {
        return false;
    }
    if !(trimmed.ends_with('{') || trimmed.ends_with(')')) {
        return false;
    }

    let first_word = trimmed.split_whitespace().next().unwrap_or("");
    // Strip type qualifiers/modifiers
    let first_word = first_word.trim_start_matches('*');

    !CONTROL_KEYWORDS.contains(&first_word)
}

fn extract_function_name(trimmed: &str, markers: &ComplexityMarkers) -> String {
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

    format!("<line {}>", trimmed.len())
}

fn count_complexity_for_lines(func_lines: &[(usize, &str)], markers: &ComplexityMarkers) -> usize {
    let mut complexity: usize = 1; // baseline

    for &(_, line) in func_lines {
        let trimmed = line.trim();
        complexity += count_line_complexity(trimmed, markers);
    }

    complexity
}

fn count_line_complexity(line: &str, markers: &ComplexityMarkers) -> usize {
    let mut count = 0;

    // Process multi-word keywords first, masking matched regions
    let mut masked = line.to_string();
    for kw in markers.keywords {
        if kw.contains(' ') {
            count += count_keyword(&masked, kw);
            // Mask matched regions to avoid double-counting
            masked = masked.replace(kw, &" ".repeat(kw.len()));
        }
    }

    // Single-word keywords
    for kw in markers.keywords {
        if !kw.contains(' ') {
            count += count_keyword(&masked, kw);
        }
    }

    // Operators (substring match)
    for op in markers.operators {
        count += count_operator(line, op);
    }

    count
}

fn count_keyword(line: &str, keyword: &str) -> usize {
    let kw_bytes = keyword.as_bytes();
    let kw_len = kw_bytes.len();
    let line_bytes = line.as_bytes();
    let line_len = line_bytes.len();
    let mut count = 0;
    let mut i = 0;

    while i + kw_len <= line_len {
        if &line_bytes[i..i + kw_len] == kw_bytes {
            // Check word boundary before
            let before_ok = i == 0 || !is_word_char(line_bytes[i - 1]);
            // Check word boundary after
            let after_ok = i + kw_len >= line_len || !is_word_char(line_bytes[i + kw_len]);
            if before_ok && after_ok {
                count += 1;
                i += kw_len;
                continue;
            }
        }
        i += 1;
    }

    count
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn count_operator(line: &str, operator: &str) -> usize {
    let mut count = 0;
    let mut start = 0;
    while let Some(pos) = line[start..].find(operator) {
        count += 1;
        start += pos + operator.len();
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust_markers() -> &'static ComplexityMarkers {
        super::super::markers::markers_for("Rust").unwrap()
    }

    fn python_markers() -> &'static ComplexityMarkers {
        super::super::markers::markers_for("Python").unwrap()
    }

    fn c_markers() -> &'static ComplexityMarkers {
        super::super::markers::markers_for("C").unwrap()
    }

    fn make_lines(code: &str) -> (Vec<String>, Vec<LineKind>) {
        let lines: Vec<String> = code.lines().map(String::from).collect();
        let kinds = vec![LineKind::Code; lines.len()];
        (lines, kinds)
    }

    #[test]
    fn simple_function_no_branches() {
        let (lines, kinds) = make_lines("fn main() {\n    let x = 1;\n}\n");
        let result = analyze(&lines, &kinds, rust_markers()).unwrap();
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].complexity, 1);
        assert_eq!(result.functions[0].level, CyclomaticLevel::Simple);
    }

    #[test]
    fn function_with_if() {
        let (lines, kinds) = make_lines("fn foo() {\n    if x > 0 {\n        bar();\n    }\n}\n");
        let result = analyze(&lines, &kinds, rust_markers()).unwrap();
        assert_eq!(result.functions[0].complexity, 2);
    }

    #[test]
    fn function_with_if_and_and() {
        let (lines, kinds) =
            make_lines("fn foo() {\n    if x > 0 && y > 0 {\n        bar();\n    }\n}\n");
        let result = analyze(&lines, &kinds, rust_markers()).unwrap();
        assert_eq!(result.functions[0].complexity, 3);
    }

    #[test]
    fn nested_if_and_for() {
        let (lines, kinds) = make_lines(
            "fn foo() {\n    if x > 0 {\n        for i in items {\n            bar();\n        }\n    }\n}\n",
        );
        let result = analyze(&lines, &kinds, rust_markers()).unwrap();
        assert_eq!(result.functions[0].complexity, 3);
    }

    #[test]
    fn word_boundary_notify_not_if() {
        let (lines, kinds) =
            make_lines("fn foo() {\n    notify();\n    ifdef();\n    life();\n}\n");
        let result = analyze(&lines, &kinds, rust_markers()).unwrap();
        assert_eq!(result.functions[0].complexity, 1);
    }

    #[test]
    fn else_if_counts_as_one() {
        let (lines, kinds) = make_lines(
            "fn foo() {\n    if x > 0 {\n        a();\n    } else if y > 0 {\n        b();\n    }\n}\n",
        );
        let result = analyze(&lines, &kinds, rust_markers()).unwrap();
        // 1 (base) + 1 (if) + 1 (else if) = 3
        assert_eq!(result.functions[0].complexity, 3);
    }

    #[test]
    fn two_rust_functions() {
        let (lines, kinds) = make_lines(
            "fn foo() {\n    if x > 0 {\n        a();\n    }\n}\nfn bar() {\n    b();\n}\n",
        );
        let result = analyze(&lines, &kinds, rust_markers()).unwrap();
        assert_eq!(result.functions.len(), 2);
        assert_eq!(result.functions[0].complexity, 2);
        assert_eq!(result.functions[0].name, "foo");
        assert_eq!(result.functions[1].complexity, 1);
        assert_eq!(result.functions[1].name, "bar");
    }

    #[test]
    fn python_function_by_indent() {
        let (lines, kinds) =
            make_lines("def foo():\n    if x > 0:\n        bar()\n\ndef baz():\n    pass\n");
        // Mark blank line as Blank
        let mut kinds = kinds;
        kinds[3] = LineKind::Blank;
        let result = analyze(&lines, &kinds, python_markers()).unwrap();
        assert_eq!(result.functions.len(), 2);
        assert_eq!(result.functions[0].name, "foo");
        assert_eq!(result.functions[0].complexity, 2); // 1 base + 1 if
        assert_eq!(result.functions[1].name, "baz");
        assert_eq!(result.functions[1].complexity, 1);
    }

    #[test]
    fn c_family_function_detection() {
        let (lines, kinds) = make_lines(
            "int main(int argc, char *argv[]) {\n    if (argc > 1) {\n        printf(\"hi\");\n    }\n    return 0;\n}\n",
        );
        let result = analyze(&lines, &kinds, c_markers()).unwrap();
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "main");
        assert_eq!(result.functions[0].complexity, 2);
    }

    #[test]
    fn file_with_no_functions_uses_implicit() {
        let (lines, kinds) = make_lines("let x = 1;\nif true { foo(); }\n");
        // Use markers without function markers (Haskell-like)
        let markers = super::super::markers::markers_for("Haskell").unwrap();
        let result = analyze(&lines, &kinds, markers).unwrap();
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "<file>");
    }

    #[test]
    fn threshold_boundaries() {
        assert_eq!(CyclomaticLevel::from_complexity(1), CyclomaticLevel::Simple);
        assert_eq!(CyclomaticLevel::from_complexity(5), CyclomaticLevel::Simple);
        assert_eq!(
            CyclomaticLevel::from_complexity(6),
            CyclomaticLevel::Moderate
        );
        assert_eq!(
            CyclomaticLevel::from_complexity(10),
            CyclomaticLevel::Moderate
        );
        assert_eq!(
            CyclomaticLevel::from_complexity(11),
            CyclomaticLevel::Complex
        );
        assert_eq!(
            CyclomaticLevel::from_complexity(20),
            CyclomaticLevel::Complex
        );
        assert_eq!(
            CyclomaticLevel::from_complexity(21),
            CyclomaticLevel::HighlyComplex
        );
        assert_eq!(
            CyclomaticLevel::from_complexity(50),
            CyclomaticLevel::HighlyComplex
        );
        assert_eq!(
            CyclomaticLevel::from_complexity(51),
            CyclomaticLevel::Extreme
        );
    }

    #[test]
    fn empty_input_returns_none() {
        let markers = rust_markers();
        assert!(analyze(&[], &[], markers).is_none());
    }

    #[test]
    fn all_comments_returns_none() {
        let lines = vec!["// comment".to_string()];
        let kinds = vec![LineKind::Comment];
        assert!(analyze(&lines, &kinds, rust_markers()).is_none());
    }

    #[test]
    fn operator_counting() {
        assert_eq!(count_operator("x && y && z", "&&"), 2);
        assert_eq!(count_operator("x || y", "||"), 1);
        assert_eq!(count_operator("no operators", "&&"), 0);
    }

    #[test]
    fn keyword_word_boundary() {
        assert_eq!(count_keyword("if x > 0", "if"), 1);
        assert_eq!(count_keyword("notify()", "if"), 0);
        assert_eq!(count_keyword("elif x", "if"), 0);
        assert_eq!(count_keyword("if_something", "if"), 0);
    }

    #[test]
    fn level_display() {
        assert_eq!(CyclomaticLevel::Simple.as_str(), "simple");
        assert_eq!(CyclomaticLevel::Moderate.as_str(), "moderate");
        assert_eq!(CyclomaticLevel::Complex.as_str(), "complex");
        assert_eq!(CyclomaticLevel::HighlyComplex.as_str(), "highly complex");
        assert_eq!(CyclomaticLevel::Extreme.as_str(), "extreme");
    }

    #[test]
    fn level_serde() {
        assert_eq!(
            serde_json::to_string(&CyclomaticLevel::HighlyComplex).unwrap(),
            "\"highly_complex\""
        );
    }

    #[test]
    fn aggregation_stats() {
        let (lines, kinds) = make_lines(
            "fn foo() {\n    if x > 0 {\n        a();\n    }\n}\nfn bar() {\n    b();\n}\n",
        );
        let result = analyze(&lines, &kinds, rust_markers()).unwrap();
        assert_eq!(result.total_complexity, 3); // 2 + 1
        assert_eq!(result.max_complexity, 2);
        assert!((result.avg_complexity - 1.5).abs() < 0.01);
    }
}
