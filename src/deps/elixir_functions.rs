//! Elixir functions: where each one is in a file, and which ones a change
//! to some lines affects.

use std::collections::BTreeSet;

use super::elixir::{Aliases, code_only, is_ident};

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

/// The function name at the start of `text`, with its `?` or `!`.
fn ident_of(text: &str) -> &str {
    let end = text
        .find(|c| !is_ident(c) && c != '?' && c != '!')
        .unwrap_or(text.len());
    &text[..end]
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
fn calls_local(body: &str, name: &str) -> bool {
    body.match_indices(name).any(|(at, _)| {
        let before = body[..at].chars().next_back().unwrap_or(' ');
        let after = body[at + name.len()..].chars().next().unwrap_or(' ');
        !is_ident(before) && before != '.' && before != ':' && matches!(after, '(' | '/')
    })
}

/// Module attributes that describe the module or its types. A change to one
/// alters no function.
const DESCRIPTIVE_ATTRIBUTES: [&str; 5] = ["moduledoc", "typedoc", "type", "typep", "opaque"];

/// What a touched line outside every function means for the functions.
enum Outside<'a> {
    /// Nothing: a blank or a comment, an `alias` or a `require` (which only
    /// name what the functions use), a descriptive attribute.
    Neutral,
    /// A module attribute: the functions that read it change with it.
    Attribute(&'a str),
    /// Anything else (`use`, `import`, `defstruct`, a schema): it may
    /// concern every function.
    Module,
}

/// Classify a line outside every function. `line` is the code as written
/// and `used` the same line once `alias` statements are blanked.
fn outside<'a>(line: &'a str, used: &str) -> Outside<'a> {
    let content = line.trim();
    if used.trim().is_empty() || content.starts_with("require ") {
        return Outside::Neutral;
    }
    match content.strip_prefix('@').map(ident_of) {
        Some(name) if DESCRIPTIVE_ATTRIBUTES.contains(&name) => Outside::Neutral,
        Some(name) if !name.is_empty() => Outside::Attribute(name),
        _ => Outside::Module,
    }
}

/// Whether `body` reads the module attribute `name`.
fn reads_attribute(body: &str, name: &str) -> bool {
    body.match_indices(name)
        .any(|(at, _)| body[..at].ends_with('@') && !body[at + name.len()..].starts_with(is_ident))
}

/// The public functions a change to `lines` affects: the ones it touches,
/// those that read a module attribute it touches, and those that reach any
/// of them through local calls.
///
/// `Err` with the first line that may concern every function, when the
/// change touches one: a `use`, an `import`, a struct, an attribute no
/// function reads. Then nothing tells which callers are affected.
pub fn changed_functions(source: &str, lines: &[usize]) -> Result<BTreeSet<String>, usize> {
    let code = code_only(source);
    let (_, without_aliases) = Aliases::of(&code);
    let text: Vec<&str> = code.lines().collect();
    let used: Vec<&str> = without_aliases.lines().collect();
    let functions = functions(source);
    let body = |f: &Function| text[f.first_line - 1..f.last_line].join("\n");
    let holder = |line: usize| {
        functions
            .iter()
            .find(|f| (f.first_line..=f.last_line).contains(&line))
    };

    let mut changed: BTreeSet<&str> = BTreeSet::new();
    for &line in lines {
        let Some(written) = text.get(line.wrapping_sub(1)) else {
            continue;
        };
        if let Some(function) = holder(line) {
            changed.insert(&function.name);
            continue;
        }
        match outside(written, used.get(line - 1).copied().unwrap_or_default()) {
            Outside::Neutral => {}
            Outside::Module => return Err(line),
            Outside::Attribute(name) => {
                let readers: Vec<&str> = functions
                    .iter()
                    .filter(|f| reads_attribute(&body(f), name))
                    .map(|f| f.name.as_str())
                    .collect();
                if readers.is_empty() {
                    return Err(line);
                }
                changed.extend(readers);
            }
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

    Ok(functions
        .iter()
        .filter(|f| f.public && changed.contains(f.name.as_str()))
        .map(|f| f.name.clone())
        .collect())
}

#[cfg(test)]
#[path = "elixir_functions_test.rs"]
mod tests;
