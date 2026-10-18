//! C#, where a `using` names a namespace, not a file, and a namespace spans
//! many files and projects. A file uses the types it names.
//!
//! The types of a file are those it declares in its namespaces (block or
//! file-scoped), outside other types. A file sees by their simple name the
//! types of each namespace it is in and of those around it, of each
//! `using`, and of each `global using` written at or above its directory;
//! an alias (`using X = A.B;`) names a type or a namespace; `using static`
//! uses the type it names. `[Name]` also names the attribute `NameAttribute`.
//! A partial type is declared by each of its files, and a use of it uses
//! them all. Types of the framework and of packages are external.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::c_like::{Token, tokens};
use super::c_like_reader::{Cursor, Reader, chain_at};
use super::graph::Source;
use super::types::{Scope, Types, nearest_all, qualified};

/// What a C# file declares, sees and names.
#[derive(Debug, Default, PartialEq)]
pub struct CSharpFile {
    /// The namespaces it declares types in.
    pub namespaces: Vec<String>,
    /// Its types, qualified.
    pub declares: Vec<String>,
    pub usings: Vec<String>,
    /// What `global using` brings to every file of its project.
    pub global_usings: Vec<String>,
    /// The types of `using static`.
    pub statics: Vec<String>,
    pub aliases: Vec<(String, String)>,
    /// The chains of names its code writes.
    pub names: Vec<Vec<String>>,
}

const DECLARATIONS: &[&str] = &["class", "struct", "interface", "enum", "record"];

/// What an open brace belongs to.
#[derive(Debug, Clone, PartialEq)]
enum Frame {
    Namespace(String),
    Type,
    Block,
}

/// Read a C# source.
pub fn read(source: &str) -> CSharpFile {
    let tokens = tokens(source);
    let mut reader = CSharpReader {
        cursor: Cursor {
            tokens: &tokens,
            at: 0,
        },
        frames: Vec::new(),
        file_namespace: String::new(),
        pending: None,
        file: CSharpFile::default(),
    };
    reader.read_all();
    reader.file
}

/// A C# source being read, token by token.
struct CSharpReader<'t> {
    cursor: Cursor<'t>,
    frames: Vec<Frame>,
    /// The namespace a file-scoped `namespace A.B;` sets.
    file_namespace: String,
    /// What the next brace opens, when a declaration said.
    pending: Option<Frame>,
    file: CSharpFile,
}

impl<'t> Reader<'t> for CSharpReader<'t> {
    fn cursor(&mut self) -> &mut Cursor<'t> {
        &mut self.cursor
    }

    /// A brace opens what a declaration said, or a body; a `;` ends a
    /// declaration that has none.
    fn punct(&mut self, mark: u8) {
        match mark {
            b'{' => {
                let frame = self.pending.take().unwrap_or(Frame::Block);
                self.frames.push(frame);
            }
            b'}' => {
                self.frames.pop();
            }
            b';' => self.pending = None,
            _ => {}
        }
    }

    /// Outside every type and body: in namespaces only.
    fn outside(&self) -> bool {
        self.frames.iter().all(|f| matches!(f, Frame::Namespace(_)))
    }

    /// A word outside every type: a `using`, a namespace, a declaration or
    /// a name.
    fn top_level(&mut self, word: &'t str) -> usize {
        match word {
            "global" if self.word(self.cursor.at + 1) == Some("using") => {
                self.using(self.cursor.at + 2, true)
            }
            "using" => self.using(self.cursor.at + 1, false),
            "namespace" => self.namespace_declaration(),
            "delegate" => self.delegate(),
            "record" if matches!(self.word(self.cursor.at + 1), Some("struct" | "class")) => {
                self.declaration(self.cursor.at + 2)
            }
            word if DECLARATIONS.contains(&word) => self.declaration(self.cursor.at + 1),
            _ => self.name(),
        }
    }

    /// The chain of names that starts here; an attribute names its class.
    fn name(&mut self) -> usize {
        let (names, end, _) = chain_at(self.cursor.tokens, self.cursor.at);
        let attribute =
            self.cursor.at > 0 && self.cursor.tokens[self.cursor.at - 1] == Token::Punct(b'[');
        if attribute && let Some((last, qualifier)) = names.split_last() {
            let mut class = qualifier.to_vec();
            class.push(format!("{last}Attribute"));
            self.file.names.push(class);
        }
        self.file.names.push(names);
        end
    }
}

impl CSharpReader<'_> {
    fn word(&self, at: usize) -> Option<&str> {
        match self.cursor.tokens.get(at) {
            Some(Token::Word(word)) => Some(word),
            _ => None,
        }
    }

    /// The namespace declarations are in here.
    fn namespace(&self) -> String {
        let inner = self.frames.iter().rev().find_map(|f| match f {
            Frame::Namespace(name) => Some(name.clone()),
            _ => None,
        });
        inner.unwrap_or_else(|| self.file_namespace.clone())
    }

    /// `using A.B;`, `using static A.B.C;`, `using X = A.B;`, from `at`.
    fn using(&mut self, at: usize, global: bool) -> usize {
        if self.word(at) == Some("static") {
            let (names, end, _) = chain_at(self.cursor.tokens, at + 1);
            self.file.statics.push(names.join("."));
            return end;
        }
        if let (Some(alias), Some(Token::Punct(b'='))) =
            (self.word(at), self.cursor.tokens.get(at + 1))
        {
            let (names, end, _) = chain_at(self.cursor.tokens, at + 2);
            self.file.aliases.push((alias.to_string(), names.join(".")));
            return end;
        }
        let (names, end, _) = chain_at(self.cursor.tokens, at);
        if !names.is_empty() {
            let into = match global {
                true => &mut self.file.global_usings,
                false => &mut self.file.usings,
            };
            into.push(names.join("."));
        }
        end.max(at)
    }

    /// `namespace A.B {` or `namespace A.B;`, inside the namespace around.
    fn namespace_declaration(&mut self) -> usize {
        let (names, end, _) = chain_at(self.cursor.tokens, self.cursor.at + 1);
        let name = qualified(&self.namespace(), &names.join("."));
        self.file.namespaces.push(name.clone());
        match self.cursor.tokens.get(end) {
            Some(Token::Punct(b';')) => self.file_namespace = name,
            _ => self.pending = Some(Frame::Namespace(name)),
        }
        end
    }

    /// A type named at `at`, whose body is the next brace.
    fn declaration(&mut self, at: usize) -> usize {
        match self.word(at) {
            Some(name) => {
                let declared = qualified(&self.namespace(), name);
                self.file.declares.push(declared);
                self.pending = Some(Frame::Type);
                at + 1
            }
            None => self.cursor.at + 1,
        }
    }

    /// `delegate R Name<T>(…);`: the name is the word before the type
    /// parameters or the parameters.
    fn delegate(&mut self) -> usize {
        let open = (self.cursor.at..self.cursor.tokens.len())
            .find(|&i| matches!(self.cursor.tokens[i], Token::Punct(b'(' | b'<' | b';')));
        match open.and_then(|i| self.word(i - 1)) {
            Some(name) => {
                let declared = qualified(&self.namespace(), name);
                self.file.declares.push(declared);
                open.unwrap_or(self.cursor.at + 1)
            }
            None => self.cursor.at + 1,
        }
    }
}

/// A namespace and those around it: `A.B.C`, `A.B`, `A`.
fn with_parents(namespace: &str) -> impl Iterator<Item = String> + '_ {
    let mut cut = Some(namespace.len());
    std::iter::from_fn(move || {
        let end = cut?;
        let name = &namespace[..end];
        cut = name.rfind('.');
        (!name.is_empty()).then(|| name.to_string())
    })
}

/// The C# files of a graph, read, and the types they declare.
pub struct Namespaces {
    paths: Vec<PathBuf>,
    files: Vec<Option<CSharpFile>>,
    types: Types,
    /// Each `global using`, with the directory it holds for.
    global: Vec<(PathBuf, String)>,
}

impl Namespaces {
    pub fn of(sources: &[Source]) -> Self {
        let files: Vec<Option<CSharpFile>> = sources
            .iter()
            .map(|s| (s.language == "C#").then(|| read(&s.text)))
            .collect();
        let mut types = Types::default();
        let mut global = Vec::new();
        for (at, file) in files.iter().enumerate() {
            let Some(file) = file else { continue };
            for declared in &file.declares {
                types.declare(declared.clone(), at);
            }
            let dir = sources[at].path.parent().unwrap_or(Path::new(""));
            global.extend(
                file.global_usings
                    .iter()
                    .map(|u| (dir.to_path_buf(), u.clone())),
            );
        }
        let paths = sources.iter().map(|s| s.path.clone()).collect();
        Namespaces {
            paths,
            files,
            types,
            global,
        }
    }

    /// What the file at `at` sees.
    fn scope(&self, at: usize, file: &CSharpFile) -> Scope {
        let path = &self.paths[at];
        let enclosing: Vec<String> = file
            .namespaces
            .iter()
            .flat_map(|n| with_parents(n))
            .collect();
        let globals = self.global.iter().filter(|(dir, _)| path.starts_with(dir));
        // The global namespace is seen from everywhere.
        let mut whole = vec![String::new()];
        whole.extend(enclosing.iter().cloned());
        whole.extend(file.usings.iter().cloned());
        whole.extend(globals.map(|(_, using)| using.clone()));
        Scope {
            whole,
            aliases: file.aliases.iter().cloned().collect(),
            enclosing,
        }
    }

    /// The files the C# file at `at` uses. `None` when it is not C#.
    pub fn uses(&self, at: usize) -> Option<BTreeMap<usize, BTreeSet<String>>> {
        let file = self.files.get(at)?.as_ref()?;
        let scope = self.scope(at, file);
        let statics = file.statics.iter().map(|s| self.types.holding(s));
        let named = file.names.iter().map(|chain| {
            let chain: Vec<&str> = chain.iter().map(String::as_str).collect();
            self.types.resolve(&chain, &scope)
        });
        let from = &self.paths[at];
        let used = statics
            .chain(named)
            .flat_map(|files| nearest_all(files, from, &self.paths));
        Some(used.map(|to| (to, BTreeSet::new())).collect())
    }
}

#[cfg(test)]
#[path = "csharp_test.rs"]
mod tests;
