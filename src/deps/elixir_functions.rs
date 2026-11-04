//! Elixir functions: where each one is in a file, and which ones a change
//! to some lines affects.

use std::collections::BTreeSet;

use super::elixir::{Aliases, is_ident};
use super::elixir_attributes::{Attributes, Readers};
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

pub(super) fn indent_of(line: &str) -> usize {
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

/// Whether a touched line outside every function and every attribute
/// changes none of them: a blank or a comment, an `alias` or a `require`,
/// which only name what the functions use. Anything else (`use`, `import`,
/// `defstruct`, a schema) may concern every function. `line` is the code as
/// written and `used` the same line once `alias` statements are blanked.
fn is_neutral(line: &str, used: &str) -> bool {
    used.trim().is_empty() || line.trim().starts_with("require ")
}

/// Add to `changed` every function of `bodies` that reaches one of them
/// through local calls: it behaves differently too.
fn carry_to_callers<'a>(bodies: &'a [(&Function, String)], changed: &mut BTreeSet<&'a str>) {
    loop {
        let callers: Vec<&str> = bodies
            .iter()
            .filter(|(f, _)| !changed.contains(f.name.as_str()))
            .filter(|(_, body)| changed.iter().any(|name| calls_local(body, name)))
            .map(|(f, _)| f.name.as_str())
            .collect();
        if callers.is_empty() {
            return;
        }
        changed.extend(callers);
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
    let bodies: Vec<(&Function, String)> = functions
        .iter()
        .map(|f| (f, text[f.first_line - 1..f.last_line].join("\n")))
        .collect();
    let attributes = Attributes::of(&text, &raw, &functions);
    let holder = |line: usize| {
        functions
            .iter()
            .find(|f| (f.first_line..=f.last_line).contains(&line))
    };

    let mut changed: BTreeSet<&str> = BTreeSet::new();
    let mut read = Readers::new();
    for &line in lines.iter().filter(|&&l| (1..=text.len()).contains(&l)) {
        if let Some(function) = holder(line) {
            changed.insert(&function.name);
        } else if let Some(readers) = attributes.readers(line, &bodies) {
            for (attribute, by) in readers? {
                read.entry(attribute).or_default().extend(by);
            }
        } else if !is_neutral(
            text[line - 1],
            used.get(line - 1).copied().unwrap_or_default(),
        ) {
            return Err(attributes.outside(line));
        }
    }
    let readers = read.values().flatten();
    changed.extend(
        functions
            .iter()
            .filter(|f| readers.clone().any(|r| *r == f.name))
            .map(|f| f.name.as_str()),
    );
    carry_to_callers(&bodies, &mut changed);

    Ok(Narrowed {
        functions: functions
            .iter()
            .filter(|f| f.public && changed.contains(f.name.as_str()))
            .map(|f| f.name.clone())
            .collect(),
        attributes: read,
    })
}

#[cfg(test)]
#[path = "elixir_functions_test.rs"]
mod tests;
