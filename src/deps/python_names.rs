//! The names a Python module defines, the ones a change touches, and the
//! ones another file takes from it.
//!
//! Read by lines and indentation: a statement starts at the left margin and
//! holds what is indented under it. What a module offers are its top-level
//! names: functions, classes and assignments. A class changes as a whole.

use std::collections::BTreeSet;

use super::python::{keyword, spec};
use crate::string_mask::starts_in_string_mask;

/// What a top-level statement is to the names of its module.
#[derive(Debug, PartialEq)]
enum Kind {
    /// It defines this name.
    Defines(String),
    /// It changes no name: an import, a docstring, what runs only as a script.
    Neutral,
    /// It runs when the module is imported, and may concern every name.
    Runs,
}

/// A statement at the left margin, with the lines it spans, from 1.
#[derive(Debug)]
struct Statement {
    first: usize,
    last: usize,
    kind: Kind,
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The identifier `text` starts with.
fn ident_of(text: &str) -> &str {
    &text[..text.find(|c| !is_ident(c)).unwrap_or(text.len())]
}

/// Whether the line says nothing: it is blank or only a comment.
fn is_silent(line: &str) -> bool {
    let line = line.trim_start();
    line.is_empty() || line.starts_with('#')
}

/// Whether a line at the left margin carries on the statement before it:
/// it closes a bracket, or it is a branch of an `if` or a `try`.
fn carries_on(line: &str) -> bool {
    line.starts_with([')', ']', '}'])
        || ["else", "elif", "except", "finally"]
            .iter()
            .any(|word| keyword(line, word).is_some())
}

/// Whether every line under an `if` or a `try` is an import, so that the
/// statement only chooses where names come from.
fn only_imports(body: &[&str]) -> bool {
    body.iter()
        .map(|line| line.trim())
        .filter(|line| !is_silent(line) && !carries_on(line))
        .all(|line| {
            ["import", "from", "pass"]
                .iter()
                .any(|word| keyword(line, word).is_some() || line == *word)
        })
}

/// The name an assignment gives a value to: `X = 1`, `X: int = 1`, `X += 1`.
fn assigned(line: &str) -> Option<&str> {
    let name = ident_of(line);
    let rest = line[name.len()..].trim_start();
    let assigns = rest.starts_with(':')
        || rest.starts_with('=') && !rest.starts_with("==")
        || rest
            .strip_prefix(|c: char| "+-*/%|&^@".contains(c))
            .is_some_and(|after| after.starts_with('='));
    (!name.is_empty() && assigns).then_some(name)
}

/// What the statement that starts with `head` is, given the lines under it.
fn kind_of(head: &str, body: &[&str]) -> Kind {
    let defined = keyword(head, "async").map_or(head, str::trim_start);
    for word in ["def", "class"] {
        if let Some(rest) = keyword(defined, word) {
            return Kind::Defines(ident_of(rest.trim_start()).to_string());
        }
    }
    let script = head.starts_with("if __name__");
    let imports = keyword(head, "import").is_some() || keyword(head, "from").is_some();
    let chooses = (keyword(head, "if").is_some() || head.starts_with("try:")) && only_imports(body);
    if script || imports || chooses || head.starts_with(['"', '\'']) {
        return Kind::Neutral;
    }
    assigned(head).map_or(Kind::Runs, |name| Kind::Defines(name.to_string()))
}

/// The top-level statements of a module, in order.
fn statements(lines: &[&str]) -> Vec<Statement> {
    let Some(spec) = spec() else {
        return Vec::new();
    };
    let owned: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    let in_string = starts_in_string_mask(&owned, spec);
    let starts_one = |at: usize| {
        let line = lines[at];
        !in_string[at]
            && !is_silent(line)
            && !line.starts_with(char::is_whitespace)
            && !carries_on(line)
    };

    let mut starts: Vec<usize> = Vec::new();
    // A decorator belongs to the definition that follows it.
    let mut decorating = false;
    for at in (0..lines.len()).filter(|&at| starts_one(at)) {
        if !decorating {
            starts.push(at);
        }
        decorating = lines[at].starts_with('@');
    }

    let ends = starts.iter().skip(1).copied().chain([lines.len()]);
    starts
        .iter()
        .zip(ends)
        .map(|(&first, end)| {
            let head = (first..end)
                .find(|&at| starts_one(at) && !lines[at].starts_with('@'))
                .unwrap_or(first);
            Statement {
                first: first + 1,
                last: end,
                kind: kind_of(lines[head], &lines[head + 1..end]),
            }
        })
        .collect()
}

/// Whether `text` mentions `name` as a name of its own: a whole word that
/// is neither an attribute of something else nor a definition of its own.
pub fn mentions(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(at, _)| {
        let before = &text[..at];
        !before.ends_with(|c| is_ident(c) || c == '.')
            && !before.ends_with("def ")
            && !before.ends_with("class ")
            && !text[at + name.len()..].starts_with(is_ident)
    })
}

/// The names a module defines at its top level.
pub fn defined(source: &str) -> Vec<String> {
    let lines: Vec<&str> = source.lines().collect();
    let names: BTreeSet<String> = statements(&lines)
        .into_iter()
        .filter_map(|statement| match statement.kind {
            Kind::Defines(name) => Some(name),
            _ => None,
        })
        .collect();
    names.into_iter().collect()
}

/// The names of a module a change to `lines` affects: the ones it touches
/// and those that use any of them.
///
/// `Err` with the first line that may concern every name, when the change
/// touches one: code that runs when the module is imported.
pub fn changed(source: &str, lines: &[usize]) -> Result<BTreeSet<String>, usize> {
    let text: Vec<&str> = source.lines().collect();
    let statements = statements(&text);
    let body = |s: &Statement| text[s.first - 1..s.last].join("\n");

    let mut changed: BTreeSet<&str> = BTreeSet::new();
    for &line in lines {
        let holder = statements
            .iter()
            .find(|s| (s.first..=s.last).contains(&line));
        let Some(statement) = holder else { continue };
        // A comment or a blank line between two statements changes neither.
        let trailing = text[line - 1..statement.last].iter().all(|l| is_silent(l));
        match &statement.kind {
            _ if trailing => {}
            Kind::Defines(name) => {
                changed.insert(name.as_str());
            }
            Kind::Neutral => {}
            Kind::Runs => return Err(line),
        }
    }

    // A name that uses a changed one behaves differently too.
    loop {
        let users: Vec<&str> = statements
            .iter()
            .filter_map(|s| match &s.kind {
                Kind::Defines(name) if !changed.contains(name.as_str()) => Some((s, name)),
                _ => None,
            })
            .filter(|(s, _)| changed.iter().any(|name| mentions(&body(s), name)))
            .map(|(_, name)| name.as_str())
            .collect();
        if users.is_empty() {
            break;
        }
        changed.extend(users);
    }
    Ok(changed.into_iter().map(str::to_string).collect())
}

/// What a file takes from a module it knows as `bound`.
#[derive(Debug, Default, PartialEq)]
pub struct Taken {
    /// The names read on the module: `parse` in `bound.parse`.
    pub names: BTreeSet<String>,
    /// Whether the module is used as a value, so that any name may be read.
    pub whole: bool,
}

/// What the `code` of a file takes from the module it knows as `bound`,
/// a name or a dotted path.
pub fn taken(code: &[String], bound: &str) -> Taken {
    let mut taken = Taken::default();
    for line in code {
        for (at, _) in line.match_indices(bound) {
            let after = &line[at + bound.len()..];
            if line[..at].ends_with(|c| is_ident(c) || c == '.') || after.starts_with(is_ident) {
                continue;
            }
            match after.strip_prefix('.').map(ident_of) {
                Some(name) if !name.is_empty() => {
                    taken.names.insert(name.to_string());
                }
                _ => taken.whole = true,
            }
        }
    }
    taken
}

#[cfg(test)]
#[path = "python_names_test.rs"]
mod tests;
