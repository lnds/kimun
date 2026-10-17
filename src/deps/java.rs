//! Java, where a file uses the classes it names. A class is known by its
//! package and its name, and lives in the file of its top-level class.
//!
//! A file sees the classes of its own package and of each import on demand
//! (`import a.b.*`) by their simple name, and each class it imports by
//! itself (`import a.b.C`, or a nested one, `a.b.C.Inner`). A static import
//! uses the class it imports from. Classes of the JDK and of libraries are
//! declared by no file of the repository: external.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::c_like::{Token, tokens};
use super::graph::{Source, shared_depth};
use super::types::{Scope, Types, qualified};

/// What a Java file declares and names.
#[derive(Debug, Default, PartialEq)]
pub struct JavaFile {
    pub package: String,
    /// Its top-level classes, interfaces, enums, records and annotations.
    pub declares: Vec<String>,
    /// What it imports by itself, qualified, statically or not.
    pub imports: Vec<String>,
    /// The packages it imports on demand.
    pub on_demand: Vec<String>,
    /// The chains of names its code writes (`User`, `models.User.find`).
    pub names: Vec<Vec<String>>,
}

const DECLARATIONS: &[&str] = &["class", "interface", "enum", "record"];

/// The chain of names that starts at `tokens[at]`, and where it ends: the
/// words joined by dots. A `.*` that closes it is told apart.
pub fn chain_at(tokens: &[Token], at: usize) -> (Vec<String>, usize, bool) {
    let mut names = Vec::new();
    let mut i = at;
    while let Some(Token::Word(word)) = tokens.get(i) {
        names.push(word.to_string());
        match (tokens.get(i + 1), tokens.get(i + 2)) {
            (Some(Token::Punct(b'.')), Some(Token::Word(_))) => i += 2,
            (Some(Token::Punct(b'.')), Some(Token::Punct(b'*'))) => return (names, i + 3, true),
            _ => return (names, i + 1, false),
        }
    }
    (names, i, false)
}

/// Read a Java source.
pub fn read(source: &str) -> JavaFile {
    let tokens = tokens(source);
    let mut reader = Reader {
        tokens: &tokens,
        at: 0,
        depth: 0,
        file: JavaFile::default(),
    };
    while reader.at < tokens.len() {
        reader.at = reader.step();
    }
    reader.file
}

/// A Java source being read, token by token.
struct Reader<'t> {
    tokens: &'t [Token<'t>],
    at: usize,
    /// How many braces are open: declarations and imports are read at 0.
    depth: usize,
    file: JavaFile,
}

impl Reader<'_> {
    /// Read what starts at the current token; where the next starts.
    fn step(&mut self) -> usize {
        let after_dot = self.at > 0 && self.tokens[self.at - 1] == Token::Punct(b'.');
        match self.tokens[self.at] {
            Token::Punct(b'{') => self.depth += 1,
            Token::Punct(b'}') => self.depth = self.depth.saturating_sub(1),
            Token::Word(_) if after_dot => {}
            Token::Word(word) if self.depth == 0 => return self.top_level(word),
            Token::Word(_) => return self.name(),
            Token::Punct(_) => {}
        }
        self.at + 1
    }

    /// A word outside every brace: a package, an import, a declaration or
    /// a name.
    fn top_level(&mut self, word: &str) -> usize {
        match word {
            "package" => {
                let (names, end, _) = chain_at(self.tokens, self.at + 1);
                self.file.package = names.join(".");
                end
            }
            "import" => self.import(),
            word if DECLARATIONS.contains(&word) => self.declaration(),
            _ => self.name(),
        }
    }

    fn import(&mut self) -> usize {
        let first = self.at + 1;
        let is_static = self.tokens.get(first) == Some(&Token::Word("static"));
        let (names, end, on_demand) = chain_at(self.tokens, first + usize::from(is_static));
        let imported = names.join(".");
        match (on_demand, is_static) {
            (true, false) => self.file.on_demand.push(imported),
            _ => self.file.imports.push(imported),
        }
        end
    }

    /// `class Name`, and the like: the name it declares.
    fn declaration(&mut self) -> usize {
        match self.tokens.get(self.at + 1) {
            Some(Token::Word(name)) => {
                self.file.declares.push(name.to_string());
                self.at + 2
            }
            _ => self.at + 1,
        }
    }

    /// The chain of names that starts here.
    fn name(&mut self) -> usize {
        let (names, end, _) = chain_at(self.tokens, self.at);
        self.file.names.push(names);
        end
    }
}

/// Those of `files` nearest to `from`: a type declared in two copies of a
/// tree (a flavor, a generated mirror) is the one of the tree it is used
/// from, while the parts of a type split among files stay together.
pub fn nearest_all(files: &[usize], from: &Path, paths: &[PathBuf]) -> Vec<usize> {
    let depth = |&f: &usize| shared_depth(&paths[f], from);
    let deepest = files.iter().map(depth).max().unwrap_or(0);
    files
        .iter()
        .copied()
        .filter(|f| depth(f) == deepest)
        .collect()
}

/// The Java files of a graph, read, and the classes they declare.
pub struct Classes {
    paths: Vec<PathBuf>,
    files: Vec<Option<JavaFile>>,
    types: Types,
}

impl Classes {
    pub fn of(sources: &[Source]) -> Self {
        let files: Vec<Option<JavaFile>> = sources
            .iter()
            .map(|s| (s.language == "Java").then(|| read(&s.text)))
            .collect();
        let mut types = Types::default();
        for (at, file) in files.iter().enumerate() {
            for (class, java) in file
                .iter()
                .flat_map(|f| f.declares.iter().map(move |c| (c, f)))
            {
                types.declare(qualified(&java.package, class), at);
            }
        }
        let paths = sources.iter().map(|s| s.path.clone()).collect();
        Classes {
            paths,
            files,
            types,
        }
    }

    /// The files the Java file at `at` uses. `None` when it is not Java.
    pub fn uses(&self, at: usize) -> Option<BTreeMap<usize, BTreeSet<String>>> {
        let file = self.files.get(at)?.as_ref()?;
        let mut scope = Scope {
            whole: vec![file.package.clone()],
            ..Scope::default()
        };
        scope.whole.extend(file.on_demand.iter().cloned());
        for import in &file.imports {
            let simple = import.rsplit('.').next().unwrap_or(import);
            scope.aliases.insert(simple.to_string(), import.clone());
        }
        let imported = file.imports.iter().map(|i| self.types.holding(i));
        let named = file.names.iter().map(|chain| {
            let chain: Vec<&str> = chain.iter().map(String::as_str).collect();
            self.types.resolve(&chain, &scope)
        });
        let from = &self.paths[at];
        let used = imported
            .chain(named)
            .flat_map(|files| nearest_all(files, from, &self.paths));
        Some(used.map(|to| (to, BTreeSet::new())).collect())
    }
}

#[cfg(test)]
#[path = "java_test.rs"]
mod tests;
