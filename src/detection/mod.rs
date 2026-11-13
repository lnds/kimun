//! Shared function detection for complexity analyzers.
//!
//! Both cyclomatic and cognitive complexity analyzers need to detect function
//! boundaries (brace-scoped or indent-scoped). This module provides the shared
//! detection logic via the `FunctionDetectionMarkers` trait, avoiding ~200 lines
//! of duplication between `cycom/detection.rs` and `cogcom/detection.rs`.

mod body;
mod declarations;
mod indented;
mod names;
mod script;
mod shape;
mod signature;

pub(crate) use body::literal_len;
use body::*;
pub(crate) use declarations::without_impl_for;
use declarations::*;
use indented::*;
pub(crate) use names::closing_paren;
pub use names::extract_function_name;
use names::*;
use script::*;
pub(crate) use script::{regex_end, value_may_start};
use shape::*;
use signature::*;

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

/// The code lines that belong to no function body: what a file runs or
/// declares outside its functions.
pub fn outside_functions<'a>(
    code_lines: &[(usize, &'a str)],
    bodies: &[FunctionBody<'_>],
) -> Vec<(usize, &'a str)> {
    let inside: std::collections::HashSet<usize> = bodies
        .iter()
        .flat_map(|body| body.code_lines.iter().map(|(i, _)| *i))
        .collect();
    code_lines
        .iter()
        .filter(|(i, _)| !inside.contains(i))
        .copied()
        .collect()
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
        // A member declared by its keyword may have no body: an abstract
        // method of a Kotlin or Swift interface ends without `;`.
        let optional_body = has_marker(masked[i].trim(), markers);
        let (body, end) =
            find_function_body(code_lines, &masked, i, expression_bodies, optional_body);
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
        // `fun interface Action {` declares a type.
        return (!is_type_declaration(line)).then(|| line.to_string());
    }
    let syntax = markers.function_syntax();
    syntax
        .by_shape()
        .then(|| shaped_declaration(masked, i, syntax))
        .flatten()
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
