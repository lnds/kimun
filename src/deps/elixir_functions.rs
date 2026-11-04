//! Elixir functions: where each one is in a file, and which ones a change
//! to some lines affects.

use std::collections::{BTreeMap, BTreeSet};

use super::elixir::{Aliases, is_ident};
use super::elixir_directives::ident_of;
use super::elixir_literals::code_only;
use super::graph::{Narrowed, Unnarrowed};

/// Keywords that define something callable, and whether it is public.
const DEFINITIONS: [(&str, bool); 7] = [
    ("def ", true),
    ("defp ", false),
    ("defmacro ", true),
    ("defmacrop ", false),
    ("defdelegate ", true),
    ("defguard ", true),
    ("defguardp ", false),
];

/// One clause of a function, and the lines it spans, from 1.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub public: bool,
    /// First line: the attributes right above the definition are part of it.
    pub first_line: usize,
    pub last_line: usize,
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The name and visibility a line defines, when it opens a definition.
fn definition(line: &str) -> Option<(&str, bool)> {
    let content = line.trim_start();
    DEFINITIONS.iter().find_map(|&(keyword, public)| {
        let name = ident_of(content.strip_prefix(keyword)?.trim_start());
        (!name.is_empty()).then_some((name, public))
    })
}

/// Keywords that, at the indentation of a definition, carry its body on.
const CONTINUATIONS: [&str; 4] = ["rescue", "catch", "after", "else"];

/// The last line of the definition opened at `start`: the one before the
/// next line indented no deeper, or that line when it is its `end`. A
/// `rescue` or the like at its indentation is still part of it.
fn last_line_of(lines: &[&str], start: usize) -> usize {
    let indent = indent_of(lines[start]);
    let carries_on = |line: &str| indent_of(line) == indent && CONTINUATIONS.contains(&line.trim());
    let next = (start + 1..lines.len()).find(|&i| {
        !lines[i].trim().is_empty() && indent_of(lines[i]) <= indent && !carries_on(lines[i])
    });
    match next {
        // An `end` further out closes what the definition is in, not it.
        Some(i) if lines[i].trim() == "end" && indent_of(lines[i]) == indent => i,
        // Blank lines after it separate it from what follows.
        Some(i) => (start..i)
            .rev()
            .find(|&j| !lines[j].trim().is_empty())
            .unwrap_or(start),
        None => lines.len() - 1,
    }
}

/// What describes the function below it: attributes, and the `attr` and
/// `slot` a Phoenix component declares its assigns with. Any other attribute
/// is the module's, and a change to it may concern every function.
const FUNCTION_DECLARATIONS: [&str; 10] = [
    "@doc",
    "@spec",
    "@impl",
    "@deprecated",
    "@dialyzer",
    "@decorate",
    "attr ",
    "attr(",
    "slot ",
    "slot(",
];

/// The first line of the definition at `start`, counting what describes it
/// right above. The text of a `@doc` is blank by now, and a declaration may
/// run over indented lines or close with an `end`: those lie between, and
/// only a declaration moves the start up.
fn first_line_of(lines: &[&str], start: usize) -> usize {
    let indent = indent_of(lines[start]);
    let describes = |line: &str| {
        let content = line.trim();
        indent_of(line) == indent && FUNCTION_DECLARATIONS.iter().any(|d| content.starts_with(d))
    };
    let lies_between = |line: &str| {
        let content = line.trim();
        content.is_empty() || indent_of(line) > indent || content == "end"
    };
    let mut first = start;
    let mut above = start;
    while above > 0 && (describes(lines[above - 1]) || lies_between(lines[above - 1])) {
        above -= 1;
        if describes(lines[above]) {
            first = above;
        }
    }
    first
}

/// The functions of a file, clause by clause, in order.
pub fn functions(source: &str) -> Vec<Function> {
    let code = code_only(source);
    let lines: Vec<&str> = code.lines().collect();
    (0..lines.len())
        .filter_map(|i| {
            let (name, public) = definition(lines[i])?;
            Some(Function {
                name: name.to_string(),
                public,
                first_line: first_line_of(&lines, i) + 1,
                last_line: last_line_of(&lines, i) + 1,
            })
        })
        .collect()
}

/// Whether `body` calls the local function `name`: `name(`, or `&name/`.
pub(super) fn calls_local(body: &str, name: &str) -> bool {
    body.match_indices(name).any(|(at, _)| {
        let before = body[..at].chars().next_back().unwrap_or(' ');
        let after = body[at + name.len()..].chars().next().unwrap_or(' ');
        !is_ident(before) && before != '.' && before != ':' && matches!(after, '(' | '/')
    })
}

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

/// Whether a touched line outside every function and every attribute
/// changes none of them: a blank or a comment, an `alias` or a `require`,
/// which only name what the functions use. Anything else (`use`, `import`,
/// `defstruct`, a schema) may concern every function. `line` is the code as
/// written and `used` the same line once `alias` statements are blanked.
fn is_neutral(line: &str, used: &str) -> bool {
    used.trim().is_empty() || line.trim().starts_with("require ")
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
fn is_router(code: &str) -> bool {
    code.lines().any(|line| {
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

/// What a change to `lines` comes to: the public functions it touches,
/// those that read a module attribute it touches (or an attribute that
/// reads it), and those that reach any of them through local calls.
///
/// `Err` with the first line that may concern every function, when the
/// change touches one: a `use`, an `import`, a struct, an attribute the
/// compiler reads or that no function reads, the routes of a router. Then
/// nothing tells which callers are affected.
pub fn changed_functions(source: &str, lines: &[usize]) -> Result<Narrowed, Unnarrowed> {
    let code = code_only(source);
    let (_, without_aliases) = Aliases::of(&code);
    let text: Vec<&str> = code.lines().collect();
    let raw: Vec<&str> = source.lines().collect();
    let used: Vec<&str> = without_aliases.lines().collect();
    let functions = functions(source);
    let spans = attribute_spans(&text, &raw, &functions);
    let router = is_router(&code);
    let body = |f: &Function| text[f.first_line - 1..f.last_line].join("\n");
    let holder = |line: usize| {
        functions
            .iter()
            .find(|f| (f.first_line..=f.last_line).contains(&line))
    };

    let mut changed: BTreeSet<&str> = BTreeSet::new();
    let mut attributes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for &line in lines {
        let Some(written) = text.get(line.wrapping_sub(1)) else {
            continue;
        };
        if let Some(function) = holder(line) {
            changed.insert(&function.name);
            continue;
        }
        let span = spans
            .iter()
            .find(|s| (s.first_line..=s.last_line).contains(&line));
        let Some(name) = span.map(|s| s.name) else {
            if is_neutral(written, used.get(line - 1).copied().unwrap_or_default()) {
                continue;
            }
            return Err(match router && declares_routes(written) {
                true => Unnarrowed::Routes(line),
                false => Unnarrowed::Outside(line),
            });
        };
        if DESCRIPTIVE_ATTRIBUTES.contains(&name) {
            continue;
        }
        if COMPILE_TIME_ATTRIBUTES.contains(&name) {
            return Err(Unnarrowed::CompileTime(line, format!("@{name}")));
        }
        let mut read = false;
        for attribute in attributes_reached(name, &spans, &text) {
            let readers: BTreeSet<String> = functions
                .iter()
                .filter(|f| reads_attribute(&body(f), attribute))
                .map(|f| f.name.clone())
                .collect();
            if readers.is_empty() {
                continue;
            }
            read = true;
            changed.extend(
                functions
                    .iter()
                    .filter(|f| readers.contains(&f.name))
                    .map(|f| f.name.as_str()),
            );
            attributes
                .entry(format!("@{attribute}"))
                .or_default()
                .extend(readers);
        }
        if !read {
            return Err(Unnarrowed::Unread(line, format!("@{name}")));
        }
    }

    // A function that calls a changed one behaves differently too.
    loop {
        let callers: Vec<&str> = functions
            .iter()
            .filter(|f| !changed.contains(f.name.as_str()))
            .filter(|f| changed.iter().any(|name| calls_local(&body(f), name)))
            .map(|f| f.name.as_str())
            .collect();
        if callers.is_empty() {
            break;
        }
        changed.extend(callers);
    }

    let functions = functions
        .iter()
        .filter(|f| f.public && changed.contains(f.name.as_str()))
        .map(|f| f.name.clone())
        .collect();
    Ok(Narrowed {
        functions,
        attributes,
    })
}

#[cfg(test)]
#[path = "elixir_functions_test.rs"]
mod tests;
