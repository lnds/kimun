//! JavaScript and TypeScript: the specifiers a source imports, and the file
//! each one names. Relative specifiers resolve from the importer; the rest
//! through the aliases of the nearest `tsconfig.json` or `jsconfig.json`,
//! then through the packages of the repository, by the name their
//! `package.json` declares. Anything else is external.
//!
//! A barrel (`index.ts`) is a file like any other: its importers depend on
//! it, and it on what it re-exports.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use super::c_like::{find, line_len};
use super::javascript_config::{Config, Package};
use super::layout::Layout;

/// What precedes a token, as far as imports are concerned.
#[derive(Clone, Copy)]
enum Before {
    From,
    Import,
    Require,
    Call,
    Dot,
    Other,
}

/// The specifiers of the import forms of a source, in order: `import … from`,
/// `import '…'`, `export … from`, `require('…')` and `import('…')`. Other
/// strings, those in comments and templates that interpolate are left out.
pub fn extract(source: &str) -> Vec<String> {
    let mut specifiers = Vec::new();
    let mut before = Before::Other;
    for token in Tokens::of(source) {
        before = match (token, before) {
            (Token::Str(Some(text)), Before::From | Before::Import | Before::Call) => {
                specifiers.push(text);
                Before::Other
            }
            (Token::Word(_), Before::Dot) => Before::Other,
            (Token::Word("from"), _) => Before::From,
            (Token::Word("import"), _) => Before::Import,
            (Token::Word("require"), _) => Before::Require,
            (Token::Punct(b'('), Before::Import | Before::Require) => Before::Call,
            (Token::Punct(b'.'), _) => Before::Dot,
            _ => Before::Other,
        };
    }
    specifiers
}

/// A token of JavaScript, coarsely: what imports need.
enum Token<'s> {
    Word(&'s str),
    /// A string literal, with its text when it is plain.
    Str(Option<String>),
    Punct(u8),
}

/// The tokens of a source, without whitespace and comments.
struct Tokens<'s> {
    source: &'s str,
    at: usize,
}

impl<'s> Tokens<'s> {
    fn of(source: &'s str) -> Self {
        Self { source, at: 0 }
    }

    fn rest(&self) -> &'s [u8] {
        &self.source.as_bytes()[self.at..]
    }

    /// Skip whitespace and comments.
    fn skip_blank(&mut self) {
        loop {
            let rest = self.rest();
            let skip = match rest {
                [c, ..] if c.is_ascii_whitespace() => 1,
                [b'/', b'/', ..] => line_len(rest),
                [b'/', b'*', ..] => find(rest, b"*/").map_or(rest.len(), |at| at + 2),
                _ => return,
            };
            self.at += skip;
        }
    }

    fn word(&mut self) -> Token<'s> {
        let len = self.rest().iter().take_while(|&&c| is_word(c)).count();
        let word = &self.source[self.at..self.at + len];
        self.at += len;
        Token::Word(word)
    }

    /// A string literal. A quoted one ends with its line; a template that
    /// interpolates is no specifier.
    fn string(&mut self) -> Token<'s> {
        let rest = self.rest();
        let quote = rest[0];
        let mut i = 1;
        while let Some(&c) = rest.get(i) {
            if c == quote {
                let text = &self.source[self.at + 1..self.at + i];
                self.at += i + 1;
                let plain = quote != b'`' || !text.contains("${");
                return Token::Str(plain.then(|| text.to_string()));
            }
            if c == b'\n' && quote != b'`' {
                break;
            }
            i += if c == b'\\' { 2 } else { 1 };
        }
        self.at += i.min(rest.len());
        Token::Str(None)
    }
}

impl<'s> Iterator for Tokens<'s> {
    type Item = Token<'s>;

    fn next(&mut self) -> Option<Token<'s>> {
        self.skip_blank();
        let c = *self.rest().first()?;
        Some(match c {
            b'"' | b'\'' | b'`' => self.string(),
            c if is_word(c) => self.word(),
            c => {
                self.at += 1;
                Token::Punct(c)
            }
        })
    }
}

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

/// The file `specifier` names when `importer` imports it, if it is one of
/// the repository. A query, as in Vite's `./a.svg?raw`, names the same file.
pub fn resolve(importer: &Path, specifier: &str, layout: &Layout) -> Option<PathBuf> {
    let known = &layout.known;
    let specifier = specifier.split('?').next().unwrap_or(specifier);
    if is_relative(specifier) {
        return relative(importer.parent().unwrap_or(Path::new("")), specifier, known);
    }
    let config = governing(importer, &layout.javascript.configs);
    config
        .and_then(|c| through_paths(c, specifier, known))
        .or_else(|| {
            let base = config?.base_url.as_ref()?;
            file_at(&base.join(specifier), known)
        })
        .or_else(|| through_packages(specifier, &layout.javascript.packages, known))
}

/// The file a relative specifier names, from the directory of its importer.
pub fn relative(dir: &Path, specifier: &str, known: &HashSet<PathBuf>) -> Option<PathBuf> {
    file_at(&dir.join(specifier), known)
}

fn is_relative(specifier: &str) -> bool {
    specifier.starts_with("./") || specifier.starts_with("../") || specifier == "."
}

/// The config of the deepest directory above `importer`.
fn governing<'a>(importer: &Path, configs: &'a [Config]) -> Option<&'a Config> {
    configs
        .iter()
        .filter(|c| importer.starts_with(&c.dir))
        .max_by_key(|c| c.dir.components().count())
}

/// Through `paths`: an exact pattern wins, then the one whose part before
/// `*` is longest. Its targets are tried in order.
fn through_paths(config: &Config, specifier: &str, known: &HashSet<PathBuf>) -> Option<PathBuf> {
    let (star, targets) = config
        .paths
        .iter()
        .filter_map(|(pattern, targets)| Some((matches(pattern, specifier)?, targets)))
        .max_by_key(|(star, _)| star.map_or(usize::MAX, |(prefix, _)| prefix))?;
    targets.iter().find_map(|target| {
        let path = match star {
            Some((_, captured)) => target.replacen('*', captured, 1),
            None => target.clone(),
        };
        file_at(Path::new(&path), known)
    })
}

/// How `specifier` matches `pattern`: `None` when it does not, the length of
/// the prefix and what `*` takes when the pattern has one.
fn matches<'s>(pattern: &str, specifier: &'s str) -> Option<Option<(usize, &'s str)>> {
    match pattern.split_once('*') {
        None => (pattern == specifier).then_some(None),
        Some((prefix, suffix)) => {
            let rest = specifier.strip_prefix(prefix)?;
            let captured = rest.strip_suffix(suffix)?;
            Some(Some((prefix.len(), captured)))
        }
    }
}

/// Through a package of the repository: the package by its name, then the
/// file the rest of the specifier names in it, or else its entry point.
fn through_packages(
    specifier: &str,
    packages: &[Package],
    known: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let (name, rest) = package_name(specifier);
    let package = packages.iter().find(|p| p.name == name)?;
    let dir = &package.dir;
    if !rest.is_empty() {
        return file_at(&dir.join(rest), known)
            .or_else(|| file_at(&dir.join("src").join(rest), known));
    }
    package
        .entries
        .iter()
        .find_map(|entry| {
            file_at(&dir.join(entry), known)
                .or_else(|| file_at(&dir.join(as_source(entry)?), known))
        })
        .or_else(|| file_at(&dir.join("src"), known))
        .or_else(|| file_at(dir, known))
}

/// A package name and the rest of a specifier: `@scope/pkg/a` is `@scope/pkg`
/// and `a`.
fn package_name(specifier: &str) -> (&str, &str) {
    let segments = if specifier.starts_with('@') { 2 } else { 1 };
    let cut = specifier
        .match_indices('/')
        .nth(segments - 1)
        .map_or(specifier.len(), |(at, _)| at);
    (&specifier[..cut], specifier.get(cut + 1..).unwrap_or(""))
}

/// An entry point under a build directory, as its source: `dist/index.js`
/// is `src/index.js`, which `file_at` reads as `src/index.ts` too.
fn as_source(entry: &str) -> Option<PathBuf> {
    let path = Path::new(entry.strip_prefix("./").unwrap_or(entry));
    let mut parts = path.components();
    let first = parts.next()?.as_os_str().to_str()?;
    ["dist", "lib", "build", "out"]
        .contains(&first)
        .then(|| Path::new("src").join(parts.as_path()))
}

const EXTENSIONS: [&str; 8] = ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

/// The file a path names: as it is, as its TypeScript source when it names
/// a compiled `.js`, with an extension added, or as the index of a
/// directory.
fn file_at(path: &Path, known: &HashSet<PathBuf>) -> Option<PathBuf> {
    let path = normalize(path)?;
    if known.contains(&path) {
        return Some(path);
    }
    let sources: &[&str] = match path.extension().and_then(|e| e.to_str()) {
        Some("js") => &["ts", "tsx"],
        Some("jsx") => &["tsx"],
        Some("mjs") => &["mts"],
        Some("cjs") => &["cts"],
        _ => &[],
    };
    let source = sources.iter().map(|ext| path.with_extension(ext));
    let with_ext = EXTENSIONS.iter().map(|ext| {
        let mut name = path.clone().into_os_string();
        name.push(format!(".{ext}"));
        PathBuf::from(name)
    });
    let index = EXTENSIONS
        .iter()
        .map(|ext| path.join(format!("index.{ext}")));
    source
        .chain(with_ext)
        .chain(index)
        .find(|p| known.contains(p))
}

/// A path with `.` and `..` resolved, without touching the filesystem.
/// `None` when it leaves the repository.
pub(super) fn normalize(path: &Path) -> Option<PathBuf> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::CurDir => {}
            other => parts.push(other),
        }
    }
    Some(parts.iter().collect())
}

#[cfg(test)]
#[path = "javascript_test.rs"]
mod tests;
