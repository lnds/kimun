//! Elixir module attributes set outside functions: the lines each value
//! spans, the functions that read it, directly or through another
//! attribute, and the attributes the compiler reads instead.

use std::collections::{BTreeMap, BTreeSet};

use super::elixir::is_ident;
use super::elixir_directives::ident_of;
use super::elixir_functions::{Function, indent_of};
use super::graph::Unnarrowed;

/// Module attributes that describe the module or its types. A change to one
/// alters no function.
const DESCRIPTIVE_ATTRIBUTES: [&str; 5] = ["moduledoc", "typedoc", "type", "typep", "opaque"];

/// Module attributes the compiler, or the macros of a library, read while
/// the module compiles: a change to one may alter any function.
const COMPILE_TIME_ATTRIBUTES: [&str; 18] = [
    "behaviour",
    "derive",
    "before_compile",
    "after_compile",
    "after_verify",
    "on_definition",
    "on_load",
    "compile",
    "external_resource",
    "enforce_keys",
    "fallback_to_any",
    "dialyzer",
    "nifs",
    "vsn",
    "primary_key",
    "foreign_key_type",
    "schema_prefix",
    "timestamps_opts",
];

/// The macros a Phoenix router declares its routes with.
const ROUTE_MACROS: [&str; 17] = [
    "scope",
    "pipe_through",
    "pipeline",
    "plug",
    "get",
    "post",
    "put",
    "patch",
    "delete",
    "options",
    "head",
    "match",
    "forward",
    "resources",
    "live",
    "live_session",
    "live_dashboard",
];

/// The name of the module attribute a line of code sets, if it sets one.
fn attribute_set(line: &str) -> Option<&str> {
    let name = ident_of(line.trim_start().strip_prefix('@')?);
    (!name.is_empty()).then_some(name)
}

/// A module attribute set outside every function, and the lines its value
/// spans, from 1.
struct AttributeSpan<'a> {
    name: &'a str,
    first_line: usize,
    last_line: usize,
}

/// Whether a line of code holds nothing but what closes a value opened
/// above: brackets, a heredoc's quotes (blank by now), commas.
fn only_closes(code: &str) -> bool {
    code.trim()
        .chars()
        .all(|c| matches!(c, ')' | ']' | '}' | '>' | ','))
}

/// The module attributes set outside `functions`, each with the lines its
/// value runs over: the lines indented deeper below it, and the one at its
/// indentation that closes them. `raw` is the source and `code` the same
/// lines with literals blanked.
fn attribute_spans<'a>(
    code: &[&'a str],
    raw: &[&str],
    functions: &[Function],
) -> Vec<AttributeSpan<'a>> {
    let in_function = |i: usize| {
        functions
            .iter()
            .any(|f| (f.first_line..=f.last_line).contains(&(i + 1)))
    };
    let mut spans = Vec::new();
    for (start, line) in code.iter().enumerate() {
        let Some(name) = attribute_set(line).filter(|_| !in_function(start)) else {
            continue;
        };
        let indent = indent_of(raw[start]);
        let mut last = start;
        for (i, next) in raw.iter().enumerate().skip(start + 1) {
            if next.trim().is_empty() {
                continue;
            }
            if indent_of(next) > indent {
                last = i;
                continue;
            }
            if only_closes(code[i]) && !next.trim_start().starts_with('#') {
                last = i;
            }
            break;
        }
        spans.push(AttributeSpan {
            name,
            first_line: start + 1,
            last_line: last + 1,
        });
    }
    spans
}

/// Whether `body` reads the module attribute `name`.
fn reads_attribute(body: &str, name: &str) -> bool {
    body.match_indices(name)
        .any(|(at, _)| body[..at].ends_with('@') && !body[at + name.len()..].starts_with(is_ident))
}

/// Whether the source is a Phoenix router, whose routes are macros read
/// when it compiles.
fn is_router(code: &[&str]) -> bool {
    code.iter().any(|line| {
        let line = line.trim();
        line.starts_with("use ") && (line.contains("Phoenix.Router") || line.ends_with(":router"))
    })
}

/// Whether a line of a router calls one of the macros of its routes.
fn declares_routes(line: &str) -> bool {
    let content = line.trim_start();
    ROUTE_MACROS.iter().any(|m| {
        content
            .strip_prefix(m)
            .is_some_and(|rest| rest.starts_with([' ', '(']))
    })
}

/// The attributes that change with `name`: itself, and every attribute
/// whose value reads one of them.
fn attributes_reached<'a>(
    name: &'a str,
    spans: &[AttributeSpan<'a>],
    text: &[&str],
) -> BTreeSet<&'a str> {
    let mut reached = BTreeSet::from([name]);
    loop {
        let more: Vec<&str> = spans
            .iter()
            .filter(|s| !reached.contains(s.name))
            .filter(|s| {
                let value = text[s.first_line - 1..s.last_line].join("\n");
                reached.iter().any(|r| reads_attribute(&value, r))
            })
            .map(|s| s.name)
            .collect();
        if more.is_empty() {
            return reached;
        }
        reached.extend(more);
    }
}

/// Who reads each attribute a change touches, by name with its `@`.
pub(super) type Readers = BTreeMap<String, BTreeSet<String>>;

/// The attributes of a source, set outside its functions.
pub(super) struct Attributes<'a> {
    spans: Vec<AttributeSpan<'a>>,
    /// The code of the source, line by line, literals blanked.
    text: &'a [&'a str],
    router: bool,
}

impl<'a> Attributes<'a> {
    /// The attributes of the source whose code is `text` and whose lines as
    /// written are `raw`.
    pub(super) fn of(text: &'a [&'a str], raw: &[&str], functions: &[Function]) -> Self {
        Attributes {
            spans: attribute_spans(text, raw, functions),
            text,
            router: is_router(text),
        }
    }

    /// Why a touched line outside every function and attribute may concern
    /// every function.
    pub(super) fn outside(&self, line: usize) -> Unnarrowed {
        match self.router && declares_routes(self.text[line - 1]) {
            true => Unnarrowed::Routes(line),
            false => Unnarrowed::Outside(line),
        }
    }

    /// What a change to `line` comes to when the line is part of an
    /// attribute's value: the attributes that change with it and the
    /// functions of `bodies` that read each. `None` when it is part of none.
    pub(super) fn readers(
        &self,
        line: usize,
        bodies: &[(&Function, String)],
    ) -> Option<Result<Readers, Unnarrowed>> {
        let span = self
            .spans
            .iter()
            .find(|s| (s.first_line..=s.last_line).contains(&line))?;
        let name = span.name;
        if DESCRIPTIVE_ATTRIBUTES.contains(&name) {
            return Some(Ok(Readers::new()));
        }
        if COMPILE_TIME_ATTRIBUTES.contains(&name) {
            return Some(Err(Unnarrowed::CompileTime(line, format!("@{name}"))));
        }
        let readers: Readers = attributes_reached(name, &self.spans, self.text)
            .into_iter()
            .filter_map(|attribute| {
                let by: BTreeSet<String> = bodies
                    .iter()
                    .filter(|(_, body)| reads_attribute(body, attribute))
                    .map(|(f, _)| f.name.clone())
                    .collect();
                (!by.is_empty()).then(|| (format!("@{attribute}"), by))
            })
            .collect();
        Some(match readers.is_empty() {
            true => Err(Unnarrowed::Unread(line, format!("@{name}"))),
            false => Ok(readers),
        })
    }
}
