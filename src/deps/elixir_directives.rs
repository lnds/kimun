//! Elixir directives that reach functions of another module without naming
//! them: `import`, `use` and `defimpl`.
//!
//! `import Foo, only: [bar: 1]` calls `Foo.bar` wherever `bar` is written
//! bare. Without `only:`, or with `except:`, which functions the file calls
//! cannot be told without the module, nor what a protocol asks of its
//! implementations: the file depends on the module as a whole. What
//! `use Foo` injects is read from `Foo` (`elixir_using`).

use std::collections::BTreeSet;

use super::elixir::{Aliases, Reference, is_ident, module_name};
use super::elixir_functions::functions;

/// What the directives of a file tell.
#[derive(Debug, Default, PartialEq)]
pub struct Directives {
    /// Functions imported by name, as calls on their module.
    pub calls: Vec<Reference>,
    /// Modules any of whose functions the file may reach.
    pub whole: Vec<String>,
    /// The modules the file `use`s.
    pub uses: Vec<Using>,
}

/// A `use` of a module: `use Foo`, or `use Foo, :name`, which runs the
/// function `name` of `Foo` by convention.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Using {
    pub module: String,
    pub option: Option<String>,
}

const KEYWORDS: [&str; 3] = ["import", "use", "defimpl"];

/// The directives of `code`, with comments and literals blanked and
/// `alias` statements removed, read with the aliases of the file.
pub fn directives(code: &str, aliases: &Aliases) -> Directives {
    let mut found = Directives::default();
    let mut at = 0;
    for line in code.split_inclusive('\n') {
        let start = at;
        at += line.len();
        let content = line.trim_start();
        let Some((keyword, rest)) = KEYWORDS.iter().find_map(|&k| {
            let rest = content.strip_prefix(k)?;
            rest.starts_with([' ', '\t'])
                .then_some((k, rest.trim_start()))
        }) else {
            continue;
        };
        let name = module_name(rest);
        if name.is_empty() {
            continue;
        }
        let module = aliases.expand(name);
        let after = start + (line.len() - rest.len()) + name.len();
        let options = statement(&code[after..]);
        if keyword == "use" {
            let option = options.trim_start().strip_prefix(',').map(str::trim_start);
            let option = option.and_then(|o| o.strip_prefix(':')).map(ident_of);
            found.uses.push(Using {
                module,
                option: option.filter(|o| !o.is_empty()).map(str::to_string),
            });
            continue;
        }
        match imported(keyword, &options) {
            Some(names) => found.calls.extend(names.into_iter().map(|f| Reference {
                module: module.clone(),
                function: Some(f),
            })),
            None => found.whole.push(module),
        }
    }
    found.calls.sort();
    found.calls.dedup();
    found.whole.sort();
    found.whole.dedup();
    found.uses.sort();
    found.uses.dedup();
    found
}

/// The options of a directive, given the text after its module name: to
/// the end of the line, on past a trailing comma or an open bracket.
fn statement(text: &str) -> String {
    let mut depth = 0usize;
    let mut end = text.len();
    for (i, c) in text.char_indices() {
        match c {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth = depth.saturating_sub(1),
            '\n' if depth == 0 && !text[..i].trim_end().ends_with(',') => {
                end = i;
                break;
            }
            _ => {}
        }
    }
    text[..end].to_string()
}

/// The functions an `import` takes by name, or `None` when the directive
/// may reach any function of the module.
fn imported(keyword: &str, options: &str) -> Option<Vec<String>> {
    if keyword != "import" || options.contains("except:") {
        return None;
    }
    let list = options
        .split_once("only:")?
        .1
        .trim_start()
        .strip_prefix('[')?;
    let list = &list[..list.find(']').unwrap_or(list.len())];
    let names = list
        .split(',')
        .map(|item| item.trim().trim_start_matches(['{', ':']))
        .map(ident_of)
        .filter(|name| name.starts_with(|c: char| c.is_ascii_lowercase() || c == '_'))
        .map(str::to_string)
        .collect();
    Some(names)
}

/// The functions `code` calls without naming a module (`name(` or
/// `&name/`) that `source` does not define: those an `import` may answer.
pub fn bare_calls(code: &str, source: &str) -> Vec<String> {
    let defined: BTreeSet<String> = functions(source).into_iter().map(|f| f.name).collect();
    let mut names = BTreeSet::new();
    let mut before = ' ';
    let mut rest = code;
    while let Some(c) = rest.chars().next() {
        let name = ident_of(rest);
        let starts = c.is_ascii_lowercase() || c == '_';
        if name.is_empty() || !starts || is_ident(before) || matches!(before, '.' | ':' | '@') {
            before = c;
            rest = &rest[c.len_utf8()..];
            continue;
        }
        let after = &rest[name.len()..];
        let called = after.starts_with('(') || (before == '&' && after.starts_with('/'));
        if called && !defined.contains(name) {
            names.insert(name.to_string());
        }
        before = '_';
        rest = after;
    }
    names.into_iter().collect()
}

/// The function name at the start of `text`, with its `?` or `!`.
pub(super) fn ident_of(text: &str) -> &str {
    let end = text
        .find(|c| !is_ident(c) && c != '?' && c != '!')
        .unwrap_or(text.len());
    &text[..end]
}

#[cfg(test)]
#[path = "elixir_directives_test.rs"]
mod tests;
