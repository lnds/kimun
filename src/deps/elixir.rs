//! Elixir modules: what a file defines and what it refers to.
//!
//! Dependencies in Elixir are between modules, not files, and most need no
//! import: `Billing.Invoice.total(x)` is enough. So every module name in the
//! code is a reference, once comments and literals are set aside and
//! `alias` is undone. The reading is lexical. It does not see modules named
//! at run time (`apply/3`, configuration), those a macro generates, nor the
//! alias a Phoenix router gives its `scope`.

use std::collections::{BTreeSet, HashMap};

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
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The index past `closer`, looked for from `from`. A string that
/// `interpolates` also honours escapes and skips `#{...}` whole, since the
/// code inside may hold quotes of its own.
fn closing(chars: &[char], from: usize, closer: &[char], interpolates: bool) -> usize {
    let mut i = from;
    while i < chars.len() {
        if interpolates && chars[i] == '\\' {
            i += 2;
        } else if interpolates && chars[i..].starts_with(&['#', '{']) {
            i = interpolation_end(chars, i + 2);
        } else if chars[i..].starts_with(closer) {
            return i + closer.len();
        } else {
            i += 1;
        }
    }
    chars.len()
}

/// The index past the `}` that closes an interpolation opened before `from`.
fn interpolation_end(chars: &[char], from: usize) -> usize {
    let mut depth = 1usize;
    let mut i = from;
    while i < chars.len() {
        match chars[i] {
            '"' => {
                i = closing(chars, i + 1, &['"'], true);
                continue;
            }
            '{' => depth += 1,
            '}' if depth == 1 => return i + 1,
            '}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    chars.len()
}

/// The index past the sigil that starts at `i` (`~r/.../`, `~S"""..."""`),
/// or `i` when what follows `~` is not one.
fn sigil_end(chars: &[char], i: usize) -> usize {
    let name_end = i
        + 1
        + chars[i + 1..]
            .iter()
            .take_while(|c| c.is_ascii_alphabetic())
            .count();
    let Some(&open) = chars.get(name_end).filter(|_| name_end > i + 1) else {
        return i;
    };
    // An uppercase sigil takes its content verbatim.
    let interpolates = chars[i + 1].is_ascii_lowercase();
    let heredoc = [open; 3];
    if matches!(open, '"' | '\'') && chars[name_end..].starts_with(&heredoc) {
        return closing(chars, name_end + 3, &heredoc, interpolates);
    }
    let close = match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        '"' | '\'' | '/' | '|' => open,
        _ => return i,
    };
    closing(chars, name_end + 1, &[close], interpolates)
}

/// The index past the comment, string, sigil or character literal that
/// starts at `i`, or `i` when none does.
fn literal_end(chars: &[char], i: usize) -> usize {
    let after_ident = i > 0 && is_ident(chars[i - 1]);
    match chars[i..] {
        ['#', ..] => i + chars[i..].iter().take_while(|&&c| c != '\n').count(),
        [q @ ('"' | '\''), b, c, ..] if q == b && q == c => closing(chars, i + 3, &[q; 3], true),
        [q @ ('"' | '\''), ..] => closing(chars, i + 1, &[q], true),
        ['~', ..] => sigil_end(chars, i),
        // `?x` is a character; in `valid?(x)` the mark belongs to the name.
        ['?', '\\', _, ..] if !after_ident => i + 3,
        ['?', _, ..] if !after_ident => i + 2,
        _ => i,
    }
}

/// The source with comments and literals blanked out. Line breaks and
/// columns are kept, so indentation still tells how deep a line is.
fn code_only(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < chars.len() {
        let end = literal_end(&chars, i).min(chars.len());
        if end == i {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        out.extend(
            chars[i..end]
                .iter()
                .map(|&c| if c == '\n' { '\n' } else { ' ' }),
        );
        i = end;
    }
    out
}

/// The dotted module name at the start of `text`: `Foo.Bar` in `Foo.Bar.baz(`.
fn module_name(text: &str) -> &str {
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
#[derive(Default)]
struct Aliases(HashMap<String, String>);

impl Aliases {
    /// A name as written, with its first segment replaced when it is an alias.
    fn expand(&self, name: &str) -> String {
        let (first, rest) = name.split_once('.').map_or((name, ""), |(f, r)| (f, r));
        match self.0.get(first) {
            Some(full) if rest.is_empty() => full.clone(),
            Some(full) => format!("{full}.{rest}"),
            None => name.to_string(),
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
    fn of(code: &str) -> (Self, String) {
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
fn references(code: &str, aliases: &Aliases) -> Vec<Reference> {
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

/// Read an Elixir source file.
pub fn parse(source: &str) -> ElixirFile {
    let (defines, code) = modules_of(&code_only(source));
    let (aliases, used) = Aliases::of(&code);
    ElixirFile {
        refs: references(&used, &aliases),
        defines,
    }
}

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

/// The last line of the definition opened at `start`: the one before the
/// next line indented no deeper, or that line when it is its `end`.
fn last_line_of(lines: &[&str], start: usize) -> usize {
    let indent = indent_of(lines[start]);
    let next = (start + 1..lines.len())
        .find(|&i| !lines[i].trim().is_empty() && indent_of(lines[i]) <= indent);
    match next {
        Some(i) if lines[i].trim() == "end" => i,
        // Blank lines after it separate it from what follows.
        Some(i) => (start..i)
            .rev()
            .find(|&j| !lines[j].trim().is_empty())
            .unwrap_or(start),
        None => lines.len() - 1,
    }
}

/// Attributes that describe the function below them. Any other attribute is
/// the module's, and a change to it may concern every function.
const FUNCTION_ATTRIBUTES: [&str; 6] = [
    "@doc",
    "@spec",
    "@impl",
    "@deprecated",
    "@dialyzer",
    "@decorate",
];

/// The first line of the definition at `start`, counting the attributes
/// that describe it right above. The text of a `@doc` is blank by now, so
/// blank lines between them are part of it; those above the first are not.
fn first_line_of(lines: &[&str], start: usize) -> usize {
    let indent = indent_of(lines[start]);
    let describes = |line: &str| {
        let content = line.trim();
        indent_of(line) == indent && FUNCTION_ATTRIBUTES.iter().any(|a| content.starts_with(a))
    };
    let mut first = start;
    let mut above = start;
    while above > 0 && (lines[above - 1].trim().is_empty() || describes(lines[above - 1])) {
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

/// The public functions a change to `lines` affects: the ones it touches,
/// and those that reach a touched private function through local calls.
///
/// `None` when the change touches code outside every function (an `alias`,
/// a `use`, a module attribute, a struct): then nothing tells which callers
/// are affected, and all of them may be.
pub fn changed_functions(source: &str, lines: &[usize]) -> Option<BTreeSet<String>> {
    let code = code_only(source);
    let text: Vec<&str> = code.lines().collect();
    let functions = functions(source);
    let holder = |line: usize| {
        functions
            .iter()
            .find(|f| (f.first_line..=f.last_line).contains(&line))
    };

    let mut changed: BTreeSet<&str> = BTreeSet::new();
    for &line in lines {
        let is_code = text
            .get(line.wrapping_sub(1))
            .is_some_and(|l| !l.trim().is_empty());
        match holder(line) {
            Some(function) => {
                changed.insert(&function.name);
            }
            None if is_code => return None,
            None => {}
        }
    }

    // A function that calls a changed one behaves differently too.
    let body = |f: &Function| text[f.first_line - 1..f.last_line].join("\n");
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

    Some(
        functions
            .iter()
            .filter(|f| f.public && changed.contains(f.name.as_str()))
            .map(|f| f.name.clone())
            .collect(),
    )
}

#[cfg(test)]
#[path = "elixir_test.rs"]
mod tests;
