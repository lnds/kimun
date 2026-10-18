//! Shared function detection for complexity analyzers.
//!
//! Both cyclomatic and cognitive complexity analyzers need to detect function
//! boundaries (brace-scoped or indent-scoped). This module provides the shared
//! detection logic via the `FunctionDetectionMarkers` trait, avoiding ~200 lines
//! of duplication between `cycom/detection.rs` and `cogcom/detection.rs`.

mod body;
mod indented;
mod script;
mod shape;

use body::*;
use indented::*;
use script::*;
use shape::*;

use crate::loc::counter::LineKind;
use crate::util::{indent_level, mask_strings};

/// Trait for language marker structs that support function detection.
/// Both `ComplexityMarkers` and `CognitiveMarkers` implement this.
pub trait FunctionDetectionMarkers {
    fn function_markers(&self) -> &[&str];
    fn brace_scoped(&self) -> bool;
    fn function_syntax(&self) -> FunctionSyntax;
    fn line_comments(&self) -> &[&str];
}

/// Implement `FunctionDetectionMarkers` for a marker struct, from its
/// fields of the same names.
macro_rules! detection_markers {
    ($markers:ty) => {
        impl $crate::detection::FunctionDetectionMarkers for $markers {
            fn function_markers(&self) -> &[&str] {
                self.function_markers
            }
            fn brace_scoped(&self) -> bool {
                self.brace_scoped
            }
            fn function_syntax(&self) -> $crate::detection::FunctionSyntax {
                self.function_syntax
            }
            fn line_comments(&self) -> &[&str] {
                self.line_comments
            }
        }
    };
}
pub(crate) use detection_markers;

/// How a language writes its functions, beyond the markers.
#[derive(Clone, Copy, PartialEq)]
pub enum FunctionSyntax {
    /// With one of the markers or, failing that, in the C-family shape.
    Loose,
    /// Always with one of the markers.
    Keyword,
    /// As `Keyword`, in a language where `'a` names a lifetime instead of
    /// opening a character literal.
    KeywordWithLifetimes,
    /// As `Loose`, plus what JavaScript and TypeScript write: functions
    /// assigned to a name (`f = function`, `f = (x) => {`) and regular
    /// expression literals, whose braces are not blocks.
    Script,
    /// Fortran program units, closed by `end` (`unit_bodies::fortran`).
    Fortran,
    /// Pascal routines, closed by the `end` of their block
    /// (`unit_bodies::pascal`).
    Pascal,
}

impl FunctionSyntax {
    /// Whether a declaration may be recognised by its shape, without a
    /// marker.
    fn by_shape(self) -> bool {
        matches!(self, Self::Loose | Self::Script)
    }
}

/// A detected function body: name, start line, and the code lines within it.
pub struct FunctionBody<'a> {
    pub name: String,
    pub start_line: usize,
    pub code_lines: Vec<(usize, &'a str)>,
}

/// The code lines of a file, as `(index, line)`, that function detection
/// and complexity read. In a language that writes functions in the
/// C-family shape, preprocessor directives and the lines a trailing `\`
/// joins to them are left out: they choose what is compiled, they are not
/// paths through the program.
pub fn code_lines<'a>(
    lines: &'a [String],
    kinds: &[LineKind],
    markers: &dyn FunctionDetectionMarkers,
) -> Vec<(usize, &'a str)> {
    let preprocessed = markers.brace_scoped() && markers.function_syntax() == FunctionSyntax::Loose;
    let mut in_directive = false;
    let mut code = Vec::new();
    for (i, (line, kind)) in lines.iter().zip(kinds).enumerate() {
        let directive = preprocessed && (in_directive || line.trim_start().starts_with('#'));
        in_directive = directive && line.trim_end().ends_with('\\');
        if *kind == LineKind::Code && !directive {
            code.push((i, line.as_str()));
        }
    }
    code
}

/// Whether a line holds one of the markers of a declaration, as the start
/// of a word: `myfunction(` holds no `function(`.
fn has_marker(trimmed: &str, markers: &dyn FunctionDetectionMarkers) -> bool {
    markers.function_markers().iter().any(|m| {
        trimmed.match_indices(m).any(|(at, _)| {
            !trimmed[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_' || c == '$')
        })
    })
}

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
/// receiver, `func (n *node) name(`, is skipped.
fn name_after(trimmed: &str, marker: &str) -> Option<String> {
    let mut after = &trimmed[trimmed.find(marker)? + marker.len()..];
    if after.starts_with('(') {
        after = after[closing_paren(after)? + 1..].trim_start();
    }
    let name: String = after
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
fn closing_paren(text: &str) -> Option<usize> {
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
fn name_before_paren(trimmed: &str) -> Option<String> {
    let mut before = trimmed[..trimmed.find('(')?].trim();
    if let Some(start) = type_arguments_start(before).filter(|_| before.ends_with('>')) {
        before = before[..start].trim_end();
    }
    let name = before.split_whitespace().next_back()?;
    let name = name.trim_start_matches('*');
    (!name.is_empty()).then(|| name.to_string())
}

/// Detect function boundaries and return their bodies.
/// Works for both brace-scoped and indent-scoped languages.
pub fn detect_function_bodies<'a>(
    all_lines: &[String],
    code_lines: &[(usize, &'a str)],
    markers: &dyn FunctionDetectionMarkers,
) -> Vec<FunctionBody<'a>> {
    match markers.function_syntax() {
        FunctionSyntax::Fortran => return crate::unit_bodies::fortran(code_lines),
        FunctionSyntax::Pascal => return crate::unit_bodies::pascal(code_lines),
        _ => {}
    }
    if markers.brace_scoped() {
        detect_brace_scoped(code_lines, markers)
    } else {
        detect_indent_scoped(all_lines, code_lines, markers)
    }
}

/// Declarations by markers or by their shape, over signatures that may
/// span lines; the body by brace depth.
fn detect_brace_scoped<'a>(
    code_lines: &[(usize, &'a str)],
    markers: &dyn FunctionDetectionMarkers,
) -> Vec<FunctionBody<'a>> {
    let masked: Vec<String> = code_lines.iter().map(|(_, l)| masked(l, markers)).collect();
    let inside = continuation_lines(&masked);
    let mut functions = Vec::new();
    let mut i = 0;
    while i < code_lines.len() {
        let Some(sig) = declaration_at(&masked, &inside, i, markers) else {
            i += 1;
            continue;
        };
        // In JavaScript a `=` binds the function to a name: it never
        // introduces an expression body.
        let expression_bodies = markers.function_syntax() != FunctionSyntax::Script;
        let (body, end) = find_function_body(code_lines, &masked, i, expression_bodies);
        // A declaration without a body is no function to measure.
        if !body.is_empty() {
            functions.push(FunctionBody {
                name: extract_function_name(&sig, markers),
                start_line: code_lines[i].0 + 1,
                code_lines: body,
            });
        }
        i = end + 1;
    }
    functions
}

/// The signature of the function declared at line `i`, if one is: the
/// line, or the lines it spans until its parentheses close, with a name
/// alone on the line above its parameters joined to them.
fn declaration_at(
    masked: &[String],
    inside: &[bool],
    i: usize,
    markers: &dyn FunctionDetectionMarkers,
) -> Option<String> {
    let line = masked[i].trim();
    // Continuations, annotations and attributes are not declarations.
    if inside[i] || line.starts_with(['@', '[']) {
        return None;
    }
    if has_marker(line, markers) {
        return Some(line.to_string());
    }
    if !markers.function_syntax().by_shape() {
        return None;
    }
    let (sig, end) = shaped_signature(masked, i);
    let next_opens = masked
        .get(end + 1)
        .is_some_and(|next| next.trim_start().starts_with('{'));
    let script = markers.function_syntax() == FunctionSyntax::Script;
    let declared = is_c_family_function(&sig, next_opens)
        || (script && sig.ends_with('{') && script_binding(&sig).is_some());
    declared.then_some(sig)
}

/// The signature at line `i`, with a name alone on its line joined to the
/// parameters on the next (`jvp_strtod` above `(struct ...)`), and the index
/// of its last line.
fn shaped_signature(masked: &[String], i: usize) -> (String, usize) {
    let line = masked[i].trim();
    let params_below = masked
        .get(i + 1)
        .is_some_and(|next| next.trim_start().starts_with('('));
    if is_identifier(line) && params_below {
        let (params, end) = signature(masked, i + 1);
        (format!("{line} {params}"), end)
    } else {
        signature(masked, i)
    }
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
