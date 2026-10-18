//! Elixir modules: what a file defines and what it refers to.
//!
//! Dependencies in Elixir are between modules, not files, and most need no
//! import: `Booking.Insight.total(x)` is enough. So every module name in the
//! code is a reference, once comments and literals are set aside and
//! `alias` is undone. The reading is lexical. It does not see modules named
//! at run time (`apply/3`, configuration) nor those a macro generates. The
//! modules a Phoenix router names under the alias of a `scope` are read
//! apart, in `routes`.

use std::collections::HashMap;
use std::path::Path;

use super::elixir_directives::{Using, bare_calls, directives};
use super::elixir_literals::code_only;
pub(super) use super::elixir_literals::{interpolation_end, literal_end};
use super::elixir_using::Injection;
use super::heex;
use super::phoenix;

/// A use of a module, and of one of its functions when it is a call.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reference {
    pub module: String,
    pub function: Option<String>,
}

/// What an Elixir source file declares and uses.
#[derive(Debug, Default, PartialEq)]
pub struct ElixirFile {
    /// Modules and protocols defined, by full name.
    pub defines: Vec<String>,
    /// Modules referred to, sorted, without repeats. Names are full: aliases
    /// are expanded. They may name modules of other projects or of libraries.
    pub refs: Vec<Reference>,
    /// Modules any of whose functions the file may reach without naming
    /// them: imported whole, or a protocol it implements.
    pub whole: Vec<String>,
    /// The modules it `use`s.
    pub uses: Vec<Using>,
    /// Functions it calls without a module, and defines none of: what an
    /// `import` may answer.
    pub bare: Vec<String>,
}

/// Whether a file configures the build rather than belongs to the code:
/// `mix.exs`, `.formatter.exs` or a script of a `config` directory. Their
/// module names are data (docs groups, `config :app, App.Endpoint`), and
/// the module a `mix.exs` defines is no part of the application.
pub fn is_build_config(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let in_config = path
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|dir| dir == "config");
    matches!(name, "mix.exs" | ".formatter.exs") || (in_config && name.ends_with(".exs"))
}

pub(super) fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The dotted module name at the start of `text`: `Foo.Bar` in `Foo.Bar.baz(`.
pub(super) fn module_name(text: &str) -> &str {
    if !text.starts_with(|c: char| c.is_ascii_uppercase()) {
        return "";
    }
    let mut end = 0;
    for segment in text.split('.') {
        let name_len = segment.find(|c| !is_ident(c)).unwrap_or(segment.len());
        if !segment.starts_with(|c: char| c.is_ascii_uppercase()) {
            break;
        }
        // The dot before every segment but the first.
        end += name_len + usize::from(end > 0);
        if name_len < segment.len() {
            break;
        }
    }
    &text[..end]
}

/// Modules open at some point of a file, innermost last, each with the
/// indentation of its `defmodule`.
#[derive(Default)]
struct Nesting(Vec<(usize, String)>);

impl Nesting {
    /// Leave the modules a line indented by `indent` is no longer inside.
    fn leave(&mut self, indent: usize) {
        while self.0.last().is_some_and(|(open, _)| *open >= indent) {
            self.0.pop();
        }
    }

    fn current(&self) -> Option<&str> {
        self.0.last().map(|(_, name)| name.as_str())
    }

    /// Enter a module: its name is appended to the one it is nested in.
    fn enter(&mut self, indent: usize, name: &str) -> String {
        let full = match self.current() {
            Some(outer) => format!("{outer}.{name}"),
            None => name.to_string(),
        };
        self.0.push((indent, full.clone()));
        full
    }
}

/// The modules a file defines, and its code with `__MODULE__` replaced by
/// the module each line is in. Nesting is told by indentation, as
/// `mix format` leaves it.
fn modules_of(code: &str) -> (Vec<String>, String) {
    let mut nesting = Nesting::default();
    let mut defines = Vec::new();
    let mut resolved = String::with_capacity(code.len());

    for line in code.lines() {
        let content = line.trim_start();
        if !content.is_empty() {
            nesting.leave(line.len() - content.len());
        }
        let declared = ["defmodule ", "defprotocol "]
            .iter()
            .find_map(|keyword| content.strip_prefix(keyword))
            .map(|rest| module_name(rest.trim_start()))
            .filter(|name| !name.is_empty());
        if let Some(name) = declared {
            defines.push(nesting.enter(line.len() - content.len(), name));
        }
        match nesting.current() {
            Some(current) => resolved.push_str(&line.replace("__MODULE__", current)),
            None => resolved.push_str(line),
        }
        resolved.push('\n');
    }
    (defines, resolved)
}

/// Short names the file gives to modules: `alias Foo.Bar` makes `Bar`
/// stand for `Foo.Bar`. They are taken as holding for the whole file.
#[derive(Debug, Default, Clone)]
pub(super) struct Aliases(HashMap<String, String>);

impl Aliases {
    /// A name as written, with its first segment replaced when it is an alias.
    pub(super) fn expand(&self, name: &str) -> String {
        let (first, rest) = name.split_once('.').map_or((name, ""), |(f, r)| (f, r));
        match self.0.get(first) {
            Some(full) if rest.is_empty() => full.clone(),
            Some(full) => format!("{full}.{rest}"),
            None => name.to_string(),
        }
    }

    /// Take the aliases of `other` this file does not declare itself.
    pub(super) fn inherit(&mut self, other: &Aliases) {
        for (short, full) in &other.0 {
            self.0.entry(short.clone()).or_insert_with(|| full.clone());
        }
    }

    fn add(&mut self, full: String, short: Option<&str>) {
        let short = short
            .or_else(|| full.rsplit('.').next())
            .unwrap_or_default()
            .to_string();
        if short != full {
            self.0.insert(short, full);
        }
    }

    /// Record what one `alias` statement declares, given the text after the
    /// keyword: `Foo.Bar`, `Foo.Bar, as: B` or `Foo.{Bar, Baz.Qux}`.
    fn declare(&mut self, statement: &str) {
        let base = module_name(statement);
        if base.is_empty() {
            return;
        }
        let base_full = self.expand(base);
        let rest = &statement[base.len()..];

        if let Some(group) = rest.strip_prefix(".{") {
            let members = &group[..group.find('}').unwrap_or(group.len())];
            for member in members.split(',').map(|m| module_name(m.trim())) {
                if !member.is_empty() {
                    self.add(format!("{base_full}.{member}"), None);
                }
            }
            return;
        }
        let renamed = rest
            .trim_start()
            .strip_prefix(',')
            .and_then(|options| options.trim_start().strip_prefix("as:"))
            .map(|name| module_name(name.trim_start()))
            .filter(|name| !name.is_empty());
        self.add(base_full, renamed);
    }

    /// Every alias declared in `code`, in order, so that one may build on
    /// another, and the code without those statements. Naming a module to
    /// shorten it is not using it: what uses it is the code that follows.
    pub(super) fn of(code: &str) -> (Self, String) {
        let mut aliases = Self::default();
        let mut rest = code.to_string();
        for (at, _) in code.match_indices("alias") {
            let whole_word = !code[..at].ends_with(is_ident);
            let statement = code[at + "alias".len()..].strip_prefix(char::is_whitespace);
            let Some(statement) = statement.filter(|_| whole_word) else {
                continue;
            };
            aliases.declare(statement.trim_start());
            let end = at + statement_len(&code[at..]);
            rest.replace_range(at..end, &blank(&code[at..end]));
        }
        (aliases, rest)
    }
}

/// The length of the `alias` statement at the start of `text`: up to the
/// `}` of a group, which may be lines below, or else to the end of the line.
fn statement_len(text: &str) -> usize {
    let line = text.find('\n').unwrap_or(text.len());
    match text[..line].contains(".{") {
        true => text.find('}').map_or(text.len(), |close| close + 1),
        false => line,
    }
}

/// `text` with everything but its line breaks turned into spaces, byte for
/// byte, so that positions in it stay where they were.
fn blank(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\n' => "\n".to_string(),
            _ => " ".repeat(c.len_utf8()),
        })
        .collect()
}

/// The function called on a module, given the text after its name:
/// `total` in `.total(x)`.
fn called(after: &str) -> Option<&str> {
    let name = after.strip_prefix('.')?;
    let end = name
        .find(|c| !is_ident(c) && c != '?' && c != '!')
        .unwrap_or(name.len());
    name.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
        .then(|| &name[..end])
}

/// Every module name in `code`, with the function called on it if any.
pub(super) fn references(code: &str, aliases: &Aliases) -> Vec<Reference> {
    let mut refs = Vec::new();
    let mut rest = code;
    let mut before = ' ';
    while let Some(c) = rest.chars().next() {
        let name = module_name(rest);
        // A name starts a word: not the tail of an identifier or of an atom.
        if name.is_empty() || is_ident(before) || before == ':' || before == '.' {
            before = c;
            rest = &rest[c.len_utf8()..];
            continue;
        }
        let after = &rest[name.len()..];
        // `Key: value` is a keyword, not a module.
        let is_keyword = after.starts_with(':') && !after.starts_with("::");
        if !is_keyword {
            refs.push(Reference {
                module: aliases.expand(name),
                function: called(after).map(str::to_string),
            });
        }
        before = '_';
        rest = after;
    }
    refs.sort();
    refs.dedup();
    refs
}

/// Read an Elixir source file that renders no template kept apart.
#[cfg(test)]
pub fn parse(source: &str) -> ElixirFile {
    parse_with(source, &[])
}

/// Read an Elixir source file that renders `templates` kept in files of
/// their own. What a template refers to, the file uses: those written in it
/// as `~H` and those given, read with the aliases of the file.
pub fn parse_with(source: &str, templates: &[String]) -> ElixirFile {
    read(source, templates, &Injection::default())
}

/// Read an Elixir source file as `parse_with` does, with what the modules
/// it `use`s inject into it: aliases its code is read with, references and
/// imports.
pub fn read(source: &str, templates: &[String], injected: &Injection) -> ElixirFile {
    let (defines, code) = modules_of(&code_only(source));
    let (mut aliases, mut used) = Aliases::of(&code);
    aliases.inherit(&injected.aliases);
    let mut refs = references(&used, &aliases);
    let directives = directives(&used, &aliases);
    refs.extend(directives.calls);
    for template in heex::embedded(source).iter().chain(templates) {
        let template = code_only(&heex::code(template));
        refs.extend(references(&template, &aliases));
        used.push_str(&template);
    }
    refs.extend(phoenix::views_of(&defines));
    refs.extend(injected.refs.iter().cloned());
    refs.sort();
    refs.dedup();
    let mut whole = directives.whole;
    whole.extend(injected.whole.iter().cloned());
    whole.sort();
    whole.dedup();
    ElixirFile {
        bare: bare_calls(&used, source),
        defines,
        refs,
        whole,
        uses: directives.uses,
    }
}

#[cfg(test)]
#[path = "elixir_test.rs"]
mod tests;
