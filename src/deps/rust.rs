//! Rust source, read lexically: the modules a file declares and the paths
//! it names.
//!
//! A dependency in Rust is a path: `use crate::git::GitRepo`, a qualified
//! `super::analyzer::run(x)`, a call on a child module. `mod x;` only says
//! where a module lives, so it is kept apart, to place files in the tree
//! of their crate; `crates` resolves the paths against that tree. Nothing
//! is expanded: what a macro generates or names is not seen.

use super::rust_literals::code_and_strings;

/// A `mod` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ModDecl {
    /// The inline modules it is inside, outermost first.
    pub scope: Vec<String>,
    pub name: String,
    /// The file a `#[path]` attribute names.
    pub path: Option<String>,
    /// Whether the body is in this file.
    pub inline: bool,
}

/// A path as written, with the inline modules around it.
#[derive(Debug, Clone, PartialEq)]
pub struct PathRef {
    pub scope: Vec<String>,
    pub segments: Vec<String>,
}

/// What a Rust source file declares and names.
#[derive(Debug, Default, PartialEq)]
pub struct RustFile {
    pub mods: Vec<ModDecl>,
    /// Paths of `use` declarations and of code. Most name other crates or
    /// local items; which do is told when they are resolved.
    pub paths: Vec<PathRef>,
    /// The names `use` brings in, each with the path it stands for.
    pub imports: Vec<(String, PathRef)>,
    /// The types `impl` blocks are for. One defined in another file gets
    /// part of what it offers from this one.
    pub impls: Vec<PathRef>,
    /// Whether the file holds tests of its own.
    pub has_tests: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tok<'a> {
    Word(&'a str),
    /// `::`
    Sep,
    /// A string literal, by its place among those of the file.
    Str(usize),
    Punct(char),
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The tokens of `code`, which has comments and literals blanked.
fn tokens(code: &str) -> Vec<Tok<'_>> {
    let mut toks = Vec::new();
    let mut strings = 0;
    let mut rest = code;
    while let Some(c) = rest.chars().next() {
        let len = if c.is_whitespace() {
            c.len_utf8()
        } else if is_word(c) {
            let len = rest.find(|c| !is_word(c)).unwrap_or(rest.len());
            // A number is no name: `0::` starts no path.
            if !c.is_numeric() {
                toks.push(Tok::Word(&rest[..len]));
            }
            len
        } else if rest.starts_with("::") {
            toks.push(Tok::Sep);
            2
        } else if c == '"' {
            toks.push(Tok::Str(strings));
            strings += 1;
            rest[1..].find('"').map_or(rest.len(), |close| close + 2)
        } else {
            toks.push(Tok::Punct(c));
            c.len_utf8()
        };
        rest = &rest[len..];
    }
    toks
}

struct Parser<'a> {
    toks: Vec<Tok<'a>>,
    strings: Vec<String>,
    at: usize,
    /// How many braces are open.
    depth: usize,
    /// The inline modules open, each with the depth its body closes at.
    open: Vec<(String, usize)>,
    /// What the last `#[path]` named, for the `mod` that follows.
    path_attr: Option<String>,
    out: RustFile,
}

impl<'a> Parser<'a> {
    fn peek(&self, ahead: usize) -> Option<Tok<'a>> {
        self.toks.get(self.at + ahead).copied()
    }

    fn scope(&self) -> Vec<String> {
        self.open.iter().map(|(name, _)| name.clone()).collect()
    }

    fn run(mut self) -> RustFile {
        while let Some(tok) = self.peek(0) {
            match tok {
                Tok::Punct('#') => self.attribute(),
                Tok::Word("mod") => self.module(),
                Tok::Word("impl") if self.starts_item() => self.implementation(),
                Tok::Word("use") => {
                    self.at += 1;
                    self.use_tree(&[]);
                }
                Tok::Word(_) => self.path(),
                other => self.punct(other),
            }
        }
        self.out
    }

    /// Braces tell where an inline module ends; a statement that ends
    /// leaves no attribute pending.
    fn punct(&mut self, tok: Tok) {
        self.at += 1;
        match tok {
            Tok::Punct('{') => self.depth += 1,
            Tok::Punct('}') => {
                self.depth = self.depth.saturating_sub(1);
                if self.open.last().is_some_and(|(_, at)| *at == self.depth) {
                    self.open.pop();
                }
            }
            Tok::Punct(';') => {}
            _ => return,
        }
        self.path_attr = None;
    }

    /// `#[...]`: a `path` for the module that follows, or the mark of a
    /// test (`#[test]`, `#[tokio::test]`, `#[rstest]`, `#[test_case(..)]`).
    fn attribute(&mut self) {
        self.at += 1;
        if self.peek(0) == Some(Tok::Punct('!')) {
            self.at += 1;
        }
        if self.peek(0) != Some(Tok::Punct('[')) {
            return;
        }
        self.at += 1;
        let mut last = "";
        let mut words = 0;
        while let Some(Tok::Word(_) | Tok::Sep) = self.peek(0) {
            if let Some(Tok::Word(word)) = self.peek(0) {
                last = word;
                words += 1;
            }
            self.at += 1;
        }
        self.out.has_tests |= last.starts_with("test") || last.ends_with("test");
        if let (true, "path", Some(Tok::Punct('=')), Some(Tok::Str(n))) =
            (words == 1, last, self.peek(0), self.peek(1))
        {
            self.path_attr = self.strings.get(n).cloned();
        }
        let mut brackets = 1;
        while let (true, Some(tok)) = (brackets > 0, self.peek(0)) {
            brackets += usize::from(tok == Tok::Punct('['));
            brackets -= usize::from(tok == Tok::Punct(']'));
            self.at += 1;
        }
    }

    /// `mod name;` or `mod name { ... }`.
    fn module(&mut self) {
        let (Some(Tok::Word(name)), Some(next)) = (self.peek(1), self.peek(2)) else {
            self.at += 1;
            return;
        };
        let inline = match next {
            Tok::Punct(';') => false,
            Tok::Punct('{') => true,
            _ => {
                self.at += 1;
                return;
            }
        };
        self.out.mods.push(ModDecl {
            scope: self.scope(),
            name: name.to_string(),
            path: self.path_attr.take(),
            inline,
        });
        self.at += 3;
        if inline {
            self.open.push((name.to_string(), self.depth));
            self.depth += 1;
        }
    }

    /// Whether the token here starts an item, rather than sit inside a
    /// type as `impl Trait` does.
    fn starts_item(&self) -> bool {
        match self.at.checked_sub(1).map(|before| self.toks[before]) {
            None | Some(Tok::Punct('{' | '}' | ';' | ']')) => true,
            Some(Tok::Word(word)) => matches!(word, "unsafe" | "default"),
            _ => false,
        }
    }

    /// Skip the generics that open here, if any do. The `>` of an arrow
    /// closes nothing.
    fn skip_generics(&mut self) {
        let mut open = 0usize;
        while let Some(tok) = self.peek(0) {
            let arrow = self.at > 0 && self.toks[self.at - 1] == Tok::Punct('-');
            match tok {
                Tok::Punct('<') => open += 1,
                Tok::Punct('>') if open > 0 && !arrow => open -= 1,
                _ if open == 0 => return,
                _ => {}
            }
            self.at += 1;
        }
    }

    /// `impl Type` or `impl Trait for Type`: the type it is for.
    fn implementation(&mut self) {
        self.at += 1;
        self.skip_generics();
        let mut target = self.read_path();
        self.skip_generics();
        if self.peek(0) == Some(Tok::Word("for")) {
            self.at += 1;
            target = self.read_path();
        }
        if let Some(path) = target {
            self.out.impls.push(path);
        }
    }

    /// The path that starts here, noted as a path of the file when it has
    /// more than one segment.
    fn read_path(&mut self) -> Option<PathRef> {
        let mut segments = Vec::new();
        while let Some(Tok::Word(word)) = self.peek(0) {
            segments.push(word.to_string());
            self.at += 1;
            match (self.peek(0), self.peek(1)) {
                (Some(Tok::Sep), Some(Tok::Word(_))) => self.at += 1,
                _ => break,
            }
        }
        let path = PathRef {
            scope: self.scope(),
            segments,
        };
        if path.segments.len() > 1 {
            self.out.paths.push(path.clone());
        }
        (!path.segments.is_empty()).then_some(path)
    }

    /// A path in code: `crate::util::parse(x)`, `Kind::Test`.
    fn path(&mut self) {
        // `$crate` belongs to whoever expands the macro.
        let after_dollar = self.at > 0 && self.toks[self.at - 1] == Tok::Punct('$');
        let noted = self.out.paths.len();
        self.read_path();
        if after_dollar {
            self.out.paths.truncate(noted);
        }
    }

    /// What follows `use`, or one branch of a group: a path that ends in a
    /// name, a glob or a group.
    fn use_tree(&mut self, prefix: &[String]) {
        let mut segments = prefix.to_vec();
        loop {
            match self.peek(0) {
                Some(Tok::Word("as")) => {
                    let alias = self.peek(1);
                    self.at += 2;
                    return match alias {
                        Some(Tok::Word(name)) => self.import(segments, name),
                        _ => self.import(segments, "_"),
                    };
                }
                Some(Tok::Word(word)) => {
                    segments.push(word.to_string());
                    self.at += 1;
                }
                Some(Tok::Sep) => self.at += 1,
                Some(Tok::Punct('*')) => {
                    self.at += 1;
                    return self.import(segments, "_");
                }
                Some(Tok::Punct('{')) => {
                    self.at += 1;
                    return self.use_group(&segments);
                }
                _ => {
                    let name = segments.last().cloned().unwrap_or_default();
                    return self.import(segments, &name);
                }
            }
        }
    }

    /// The branches of `{a, b::c, self}`, up to the brace that closes it.
    fn use_group(&mut self, prefix: &[String]) {
        loop {
            match self.peek(0) {
                Some(Tok::Punct('}')) => {
                    self.at += 1;
                    return;
                }
                Some(Tok::Punct(',')) => self.at += 1,
                Some(Tok::Word(_) | Tok::Sep | Tok::Punct('*' | '{')) => self.use_tree(prefix),
                _ => return,
            }
        }
    }

    /// Record what a `use` names. `self` at the end stands for the module
    /// before it; `_` is no name to refer to later.
    fn import(&mut self, mut segments: Vec<String>, name: &str) {
        let mut name = name.to_string();
        if segments.last().is_some_and(|last| last == "self") && segments.len() > 1 {
            segments.pop();
            if name == "self" {
                name = segments.last().cloned().unwrap_or_default();
            }
        }
        if segments.is_empty() {
            return;
        }
        let path = PathRef {
            scope: self.scope(),
            segments,
        };
        if name != "_" && path.segments.len() > 1 {
            self.out.imports.push((name, path.clone()));
        }
        self.out.paths.push(path);
    }
}

/// Read a Rust source file.
pub fn parse(source: &str) -> RustFile {
    let (code, strings) = code_and_strings(source);
    let parser = Parser {
        toks: tokens(&code),
        strings,
        at: 0,
        depth: 0,
        open: Vec::new(),
        path_attr: None,
        out: RustFile::default(),
    };
    parser.run()
}

#[cfg(test)]
#[path = "rust_test.rs"]
mod tests;
