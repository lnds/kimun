//! The long_parameter_list smell: how many parameters a function declares.

use crate::detection::{
    FunctionBody, FunctionDetectionMarkers, closing_paren as closing, detect_function_bodies,
};
use crate::loc::counter::LineKind;
use crate::util::mask_strings;

use super::analyzer::{SmellInstance, SmellKind};

/// Detect functions with more than `max_params` parameters.
///
/// The parameters are those of the list that follows the function's name,
/// joined over the lines it spans, and only its top-level commas separate
/// them: commas inside type arguments, patterns, literals or nested calls
/// do not.
pub fn detect_long_params(
    lines: &[String],
    kinds: &[LineKind],
    markers: &dyn FunctionDetectionMarkers,
    max_params: usize,
) -> Vec<SmellInstance> {
    let code_lines = crate::detection::code_lines(lines, kinds, markers);
    detect_function_bodies(lines, &code_lines, markers)
        .iter()
        .filter_map(|func| {
            let signature = collect_signature(func, markers.line_comments())?;
            let count = count_params(&signature, &func.name)?;
            (count > max_params).then(|| SmellInstance {
                kind: SmellKind::LongParameterList,
                line: func.start_line,
                detail: format!(
                    "function `{}` has {} params (max {})",
                    func.name, count, max_params
                ),
            })
        })
        .collect()
}

/// Join the code lines of a function, literals and comments blanked, until
/// the parameter list closes. `None` if no `(` is found.
fn collect_signature(func: &FunctionBody<'_>, line_comments: &[&str]) -> Option<String> {
    let mut sig = String::new();
    for &(_, code_line) in &func.code_lines {
        let masked = mask_strings(code_line, line_comments);
        if sig.is_empty() && !masked.contains('(') {
            continue;
        }
        sig.push_str(&masked);
        sig.push(' ');
        let open = sig.find('(')?;
        if closing(&sig[open..]).is_some() {
            return Some(sig);
        }
    }
    (!sig.is_empty()).then_some(sig)
}

/// The number of parameters in the list after `name` in `signature`, or
/// after its first `(` when the name is not found before one. `None` for a
/// list that does not close.
fn count_params(signature: &str, name: &str) -> Option<usize> {
    let open = params_open(signature, name)?;
    let list = &signature[open..];
    let close = closing(list)?;
    let inner = list[1..close].trim().trim_end_matches(',').trim();
    if inner.is_empty() {
        return Some(0);
    }
    Some(top_level_commas(inner) + 1)
}

/// Where the parameter list of `name` opens: the `(` after the name and any
/// type parameters (`Node<K, V>(`), or the first `(` of the signature.
fn params_open(signature: &str, name: &str) -> Option<usize> {
    let after_name = signature
        .match_indices(name)
        .filter(|(at, _)| !preceded_by_word(signature, *at))
        .find_map(|(at, _)| {
            let params = skip_type_parameters(&signature[at + name.len()..]);
            // A suffix of the signature: where it starts is where `(` is.
            params
                .starts_with('(')
                .then_some(signature.len() - params.len())
        });
    after_name.or_else(|| signature.find('('))
}

fn preceded_by_word(text: &str, at: usize) -> bool {
    text[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_')
}

/// `text` without the whitespace and the `<...>` type parameters it starts
/// with.
fn skip_type_parameters(text: &str) -> &str {
    let text = text.trim_start();
    if !text.starts_with('<') {
        return text;
    }
    let mut depth = 0;
    for (i, ch) in text.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' if depth == 1 => return text[i + 1..].trim_start(),
            '>' => depth -= 1,
            _ => {}
        }
    }
    text
}

/// The commas of `inner` outside parentheses, brackets, braces, type
/// arguments and binary patterns (`<<...>>`).
fn top_level_commas(inner: &str) -> usize {
    let bytes = inner.as_bytes();
    let mut depth = 0i32;
    let mut commas = 0;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b',' if depth == 0 => commas += 1,
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'>' if !is_arrow(bytes, i) => depth -= 1,
            _ => {}
        }
    }
    commas
}

/// Whether the `>` at `i` ends an arrow (`->`, `=>`) or a comparison
/// (`>=`), rather than closing type arguments.
fn is_arrow(bytes: &[u8], i: usize) -> bool {
    let before = i.checked_sub(1).map(|p| bytes[p]);
    matches!(before, Some(b'-' | b'=')) || bytes.get(i + 1) == Some(&b'=')
}

#[cfg(test)]
#[path = "params_test.rs"]
mod tests;
