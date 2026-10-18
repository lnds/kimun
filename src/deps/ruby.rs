//! Ruby, read lexically: what a file requires, the constants it defines and
//! the constants it refers to.
//!
//! Classes and modules nest by indentation, as formatted code has them: a
//! definition holds the lines indented under it, up to its `end`. A module
//! or class opened only to hold other definitions (`module Shop` around
//! `class Cart`) is a namespace, not a definition of its own; one that
//! names a superclass, or holds nothing, is a definition. A constant
//! assigned at the level of a class or module (`VERSION = "1"`,
//! `Point = Struct.new(:x)`) is a definition too.

use std::collections::BTreeSet;

use super::ruby_literals::{Line, Tok, lines};

/// A file a `require` names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Require {
    /// Relative to the directory of the requiring file: `require_relative`,
    /// or a path built from `__dir__` or `__FILE__`.
    Relative(String),
    /// Looked up in the load path: `require`, `load`, `autoload`.
    LoadPath(String),
}

/// A constant written in code, with the namespaces it is looked up from,
/// innermost first; the outermost is the top level, `""`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reference {
    pub chain: Vec<String>,
    pub scopes: Vec<String>,
}

/// What a Ruby file requires, defines and refers to.
#[derive(Debug, Default, PartialEq)]
pub struct RubyFile {
    pub requires: Vec<Require>,
    /// The constants it defines, qualified (`Shop::Cart`).
    pub defines: Vec<String>,
    pub refs: BTreeSet<Reference>,
}

/// A class or module open at some indentation.
struct Frame {
    indent: usize,
    name: String,
    /// The namespaces a constant written inside is looked up from.
    scopes: Vec<String>,
    /// Whether it holds anything besides other definitions, or names a
    /// superclass.
    body: bool,
    /// Whether it holds other definitions.
    nested: bool,
    /// `class << self`: neither defines nor holds a namespace.
    singleton: bool,
}

/// `outer::name`, or `name` at the top level.
pub fn qualified(outer: &str, name: &str) -> String {
    if outer.is_empty() {
        name.to_string()
    } else {
        format!("{outer}::{name}")
    }
}

fn is_constant(word: &str) -> bool {
    word.starts_with(|c: char| c.is_ascii_uppercase())
}

fn word(token: Option<&Tok>) -> Option<&str> {
    match token {
        Some(Tok::Word(w)) => Some(w),
        _ => None,
    }
}

/// The chain of constants starting at `at` (`A::B::C`, `::A`), whether it
/// is absolute, and the index past it.
fn chain_at(tokens: &[Tok], at: usize) -> (Vec<String>, bool, usize) {
    let absolute = tokens.get(at) == Some(&Tok::Scope);
    let mut i = at + usize::from(absolute);
    let mut chain = Vec::new();
    while let Some(name) = word(tokens.get(i)).filter(|w| is_constant(w)) {
        chain.push(name.to_string());
        i += 1;
        if tokens.get(i) == Some(&Tok::Scope) && word(tokens.get(i + 1)).is_some_and(is_constant) {
            i += 1;
        } else {
            break;
        }
    }
    (chain, absolute, i)
}

/// Read a Ruby source.
pub fn read(source: &str) -> RubyFile {
    let mut reader = Reader {
        frames: Vec::new(),
        file: RubyFile::default(),
    };
    for line in lines(source).iter().filter(|l| !l.tokens.is_empty()) {
        reader.line(line);
    }
    while !reader.frames.is_empty() {
        reader.close();
    }
    reader.file
}

struct Reader {
    frames: Vec<Frame>,
    file: RubyFile,
}

impl Reader {
    fn scopes(&self) -> Vec<String> {
        self.frames
            .last()
            .map_or_else(|| vec![String::new()], |f| f.scopes.clone())
    }

    fn namespace(&self) -> &str {
        self.frames.last().map_or("", |f| f.name.as_str())
    }

    fn close(&mut self) {
        let Some(frame) = self.frames.pop() else {
            return;
        };
        if frame.singleton {
            return;
        }
        if frame.body || !frame.nested {
            self.file.defines.push(frame.name);
        }
    }

    fn line(&mut self, line: &Line) {
        let tokens = line.tokens.as_slice();
        let mut closed_by_end = false;
        while self.frames.last().is_some_and(|f| f.indent >= line.indent) {
            closed_by_end = word(tokens.first()) == Some("end");
            self.close();
        }
        let opens = matches!(word(tokens.first()), Some("class" | "module"))
            && tokens.get(1) != Some(&Tok::Punct(b'<'));
        if !opens
            && !closed_by_end
            && let Some(frame) = self.frames.last_mut()
        {
            frame.body = true;
        }
        let mut at = 0;
        while at < tokens.len() {
            at = self.statement(tokens, at, line.indent);
        }
    }

    /// One statement starting at `at`; the index past it.
    fn statement(&mut self, tokens: &[Tok], at: usize, indent: usize) -> usize {
        let end = (at..tokens.len())
            .find(|&i| tokens[i] == Tok::Punct(b';'))
            .unwrap_or(tokens.len());
        let statement = &tokens[at..end];
        let rest = match word(statement.first()) {
            Some("class" | "module") => self.definition(statement, &tokens[end..], indent),
            Some("require_relative" | "require" | "load" | "autoload") => {
                self.require(statement);
                0
            }
            _ => self.assignment(statement),
        };
        self.names(statement, rest);
        end + 1
    }

    /// `class A::B < C`, `module A`, `class << self`; the index of the
    /// statement past the name, from which constants are references.
    fn definition(&mut self, statement: &[Tok], after: &[Tok], indent: usize) -> usize {
        if statement.get(1) == Some(&Tok::Punct(b'<')) {
            let frame = Frame {
                indent,
                name: self.namespace().to_string(),
                scopes: self.scopes(),
                body: true,
                nested: false,
                singleton: true,
            };
            self.frames.push(frame);
            return statement.len();
        }
        let (chain, absolute, past) = chain_at(statement, 1);
        if chain.is_empty() {
            return 1;
        }
        let outer = if absolute { "" } else { self.namespace() };
        let name = qualified(outer, &chain.join("::"));
        if let Some(parent) = self.frames.last_mut() {
            parent.nested = true;
        }
        // `class Error < StandardError; end`: defined and closed at once.
        let closed = after.iter().skip(1).any(|t| word(Some(t)) == Some("end"));
        if closed {
            self.file.defines.push(name);
            return past;
        }
        let mut scopes = vec![name.clone()];
        scopes.extend(self.scopes());
        // References on the line of the definition, a superclass, are
        // looked up from outside it.
        let outside = self.scopes();
        self.refs_in(&statement[past..], &outside);
        self.frames.push(Frame {
            indent,
            name,
            scopes,
            body: statement.get(past) == Some(&Tok::Punct(b'<')),
            nested: false,
            singleton: false,
        });
        statement.len()
    }

    /// `NAME = …` or `NAME ||= …` defines `NAME` where it is written.
    fn assignment(&mut self, statement: &[Tok]) -> usize {
        let Some(name) = word(statement.first()).filter(|w| is_constant(w)) else {
            return 0;
        };
        let assigns = match &statement[1..] {
            [Tok::Punct(b'='), Tok::Punct(b'=' | b'~' | b'>'), ..] => false,
            [Tok::Punct(b'='), ..] => true,
            [Tok::Punct(b'|'), Tok::Punct(b'|'), Tok::Punct(b'='), ..] => true,
            _ => false,
        };
        if !assigns {
            return 0;
        }
        let name = qualified(self.namespace(), name);
        self.file.defines.push(name);
        1
    }

    fn require(&mut self, statement: &[Tok]) {
        let keyword = word(statement.first()).unwrap_or("");
        let args = match statement.get(1) {
            Some(Tok::Punct(b'(')) => &statement[2..],
            _ => &statement[1..],
        };
        let required = match (keyword, args) {
            ("autoload", [Tok::Symbol(_), Tok::Punct(b','), Tok::Str(path, false), ..]) => {
                Some(Require::LoadPath(path.clone()))
            }
            ("autoload", _) => None,
            ("require_relative", [Tok::Str(path, false), ..]) => {
                Some(Require::Relative(path.clone()))
            }
            (_, [Tok::Str(path, false), ..]) => Some(Require::LoadPath(path.clone())),
            _ => built_path(args),
        };
        self.file.requires.extend(required);
    }

    /// The constants written in `statement` from `from` on.
    fn names(&mut self, statement: &[Tok], from: usize) {
        let scopes = self.scopes();
        self.refs_in(&statement[from.min(statement.len())..], &scopes);
    }

    fn refs_in(&mut self, tokens: &[Tok], scopes: &[String]) {
        let mut i = 0;
        while i < tokens.len() {
            let after_dot = i > 0 && tokens[i - 1] == Tok::Punct(b'.');
            let starts = match &tokens[i] {
                Tok::Scope => word(tokens.get(i + 1)).is_some_and(is_constant),
                Tok::Word(w) => is_constant(w),
                _ => false,
            };
            if !starts || after_dot {
                i += 1;
                continue;
            }
            let (chain, absolute, past) = chain_at(tokens, i);
            let scopes = if absolute {
                vec![String::new()]
            } else {
                scopes.to_vec()
            };
            self.file.refs.insert(Reference { chain, scopes });
            i = past;
        }
    }
}

/// A path built from where the file is: `File.expand_path('../x', __FILE__)`,
/// `File.join(__dir__, 'x')`, `"#{__dir__}/x"`, relative to its directory.
fn built_path(args: &[Tok]) -> Option<Require> {
    let names: Vec<&str> = args.iter().filter_map(|t| word(Some(t))).collect();
    let from_file = names.contains(&"__FILE__");
    if !from_file && !names.contains(&"__dir__") {
        return None;
    }
    let parts: Vec<&str> = args
        .iter()
        .filter_map(|t| match t {
            Tok::Str(text, _) => Some(text.trim_matches('/')),
            _ => None,
        })
        .filter(|p| !p.is_empty())
        .collect();
    let path = parts.join("/");
    // `expand_path('../x', __FILE__)` starts from the file itself.
    let path = match from_file && names.contains(&"expand_path") && !names.contains(&"dirname") {
        true => path.strip_prefix("../")?.to_string(),
        false => path,
    };
    (!path.is_empty()).then_some(Require::Relative(path))
}

#[cfg(test)]
#[path = "ruby_test.rs"]
mod tests;
