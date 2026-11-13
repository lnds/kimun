//! The name of a declared function: after its marker, bound to it in a
//! script, or the token before its parameters.

use super::*;

/// Extract the function name from a declaration (one line, or a signature
/// joined over its lines) using language-specific markers or the C-family
/// heuristic (token before the first `(`).
pub fn extract_function_name(trimmed: &str, markers: &dyn FunctionDetectionMarkers) -> String {
    if markers.function_syntax() == FunctionSyntax::Script {
        return script_name(trimmed);
    }
    let mut function_markers = markers.function_markers().iter();
    function_markers
        .find_map(|marker| name_after(trimmed, marker))
        .or_else(|| name_before_paren(trimmed))
        .unwrap_or_else(|| "<anonymous>".to_string())
}

/// The identifier that follows `marker` in a declaration, if any. A Go
/// receiver, `func (n *node) name(`, is skipped, and so are type
/// parameters and the receiver of an extension or a singleton method
/// (`fun <T> List<T>.name(`, `def self.name`).
fn name_after(trimmed: &str, marker: &str) -> Option<String> {
    let mut after = &trimmed[trimmed.find(marker)? + marker.len()..];
    if after.starts_with('(') {
        after = after[closing_paren(after)? + 1..].trim_start();
    }
    let after = without_type_arguments(after);
    let target = after
        .trim_start()
        .split(|c: char| c.is_whitespace() || "(,:".contains(c))
        .next()?;
    let name: String = target
        .rsplit('.')
        .next()?
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// Where the `<` that opens the type arguments ending `text` is.
fn type_arguments_start(text: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, ch) in text.char_indices().rev() {
        match ch {
            '>' => depth += 1,
            '<' if depth == 1 => return Some(i),
            '<' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Where the parenthesis that opens `text` closes.
pub(crate) fn closing_paren(text: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' if depth == 1 => return Some(i),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// C-family heuristic: the name is the token before the first `(`, without
/// the type arguments of a generic (`basePath<P extends string>(`).
pub(super) fn name_before_paren(trimmed: &str) -> Option<String> {
    let mut before = trimmed[..trimmed.find('(')?].trim();
    if let Some(start) = type_arguments_start(before).filter(|_| before.ends_with('>')) {
        before = before[..start].trim_end();
    }
    let name = before.split_whitespace().next_back()?;
    let name = name.trim_start_matches('*');
    (!name.is_empty()).then(|| name.to_string())
}
