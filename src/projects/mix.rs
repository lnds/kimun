//! Reader for `mix.exs`.
//!
//! A `mix.exs` is Elixir code, read here as text: comments are dropped, then
//! the dependency tuples `{:name, ...}` are picked out. A dependency list
//! built at run time is seen only in part, and what was missed is counted.

use std::path::PathBuf;

use super::{DepTarget, Manifest, RawDep, Scope};

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The source without its comments. A `#` inside a string, or as the
/// character literal `?#`, is not one.
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        match c {
            '#' if !in_string => while chars.next_if(|&n| n != '\n').is_some() {},
            '"' => {
                in_string = !in_string;
                out.push(c);
            }
            // An escaped character in a string, or a character literal.
            '\\' | '?' if in_string == (c == '\\') => {
                out.push(c);
                out.extend(chars.next());
            }
            _ => out.push(c),
        }
    }
    out
}

/// The text after each `key:` in `text`, where `key` is a whole word.
fn values_of<'a>(text: &'a str, key: &'a str) -> impl Iterator<Item = &'a str> {
    text.match_indices(key).filter_map(move |(at, _)| {
        let rest = text[at + key.len()..].strip_prefix(':')?;
        let whole_word = !text[..at].ends_with(is_ident);
        (whole_word && !rest.starts_with(':')).then(|| rest.trim_start())
    })
}

/// The identifier at the start of `text`.
fn ident(text: &str) -> &str {
    let end = text.find(|c| !is_ident(c)).unwrap_or(text.len());
    &text[..end]
}

/// The atom `:name` at the start of `text`.
fn atom(text: &str) -> Option<&str> {
    let name = ident(text.strip_prefix(':')?);
    (!name.is_empty()).then_some(name)
}

/// The content of the string literal at the start of `text`. `None` for
/// anything else, and for a string with interpolation.
fn literal(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('"')?;
    let content = &rest[..rest.find('"')?];
    (!content.contains("#{")).then_some(content)
}

/// The characters of `text` that are outside string literals, with their index.
fn outside_strings(text: &str) -> impl Iterator<Item = (usize, char)> + '_ {
    let mut in_string = false;
    let mut escaped = false;
    text.char_indices().filter(move |&(_, c)| {
        let was_escaped = std::mem::take(&mut escaped);
        match c {
            _ if was_escaped => false,
            '\\' if in_string => {
                escaped = true;
                false
            }
            '"' => {
                in_string = !in_string;
                false
            }
            _ => !in_string,
        }
    })
}

/// The index of the bracket that closes the one opened just before `text`.
fn closing(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in outside_strings(text) {
        match c {
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' if depth == 0 => return Some(i),
            '}' | ']' | ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Every tuple `{:name, ...}` of `text`, as its name and the rest of it.
fn tuples(text: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let inside = &rest[open + 1..];
        let parsed = atom(inside.trim_start()).zip(closing(inside));
        match parsed {
            Some((name, close)) => {
                found.push((name, &inside[..close]));
                rest = &inside[close..];
            }
            None => rest = inside,
        }
    }
    found
}

/// The scope the options of a dependency give it.
fn scope(options: &str) -> Scope {
    let is = |key, value: &str| values_of(options, key).any(|v| v.starts_with(value));
    let only = values_of(options, "only").next().map(|v| {
        let end = v.find([']', '}']).unwrap_or(v.len());
        &v[..end]
    });
    match only {
        Some(envs) if !envs.contains(":prod") => Scope::Dev,
        _ if is("runtime", "false") => Scope::Build,
        _ if is("optional", "true") => Scope::Optional,
        _ => Scope::Runtime,
    }
}

/// Read a `mix.exs`. A file without `app:` is an umbrella root and declares
/// no project.
pub fn read(source: &str) -> Manifest {
    let text = strip_comments(source);

    let mut deps = Vec::new();
    for (name, options) in tuples(&text) {
        let path = values_of(options, "path").next().and_then(literal);
        let in_umbrella = values_of(options, "in_umbrella").any(|v| v.starts_with("true"));
        let target = match path {
            Some(path) => DepTarget::Path(PathBuf::from(path)),
            None if in_umbrella => DepTarget::Name(name.to_string()),
            None => continue,
        };
        deps.push(RawDep {
            target,
            scope: scope(options),
        });
    }

    // Every `path:` that did not end up as a dependency read above.
    let paths = values_of(&text, "path").count();
    let read = deps
        .iter()
        .filter(|d| matches!(d.target, DepTarget::Path(_)))
        .count();

    Manifest {
        name: values_of(&text, "app").find_map(atom).map(str::to_string),
        deps,
        unread: paths.saturating_sub(read),
        is_workspace_root: values_of(&text, "apps_path").next().is_some(),
        ..Manifest::default()
    }
}

#[cfg(test)]
#[path = "mix_test.rs"]
mod tests;
