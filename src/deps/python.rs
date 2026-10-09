//! Python import extraction and resolution.
//!
//! An absolute import names a module under a root of the search path, which
//! the source does not tell; a relative one, a module beside the importer.
//! Both are kept, and the resolver leaves out what the project does not hold.
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::loc::language::languages;
use crate::string_mask::starts_in_string_mask;

/// Extract what each `import` and `from … import` statement names, wherever
/// it is written: at the top, in a function, under `if TYPE_CHECKING:`.
///
/// `import a.b` gives `a.b`. `from a.b import c` gives `a.b:c`, since `c`
/// may be a submodule of `a.b` or a name it defines; `from a.b import *`
/// gives `a.b`. Leading dots are kept: `from . import c` gives `.:c`.
///
/// Statements may run over several lines, in parentheses or with a
/// backslash. A statement written inside a docstring is not one.
pub fn extract(source: &str) -> Vec<String> {
    let Some(spec) = languages().iter().find(|s| s.name == "Python") else {
        return vec![];
    };
    let lines: Vec<String> = source.lines().map(String::from).collect();
    let in_string = starts_in_string_mask(&lines, spec);
    let mut code = lines
        .iter()
        .zip(in_string)
        .filter(|(_, masked)| !masked)
        .map(|(line, _)| without_comment(line));

    let mut imports = Vec::new();
    while let Some(line) = code.next() {
        let line = line.trim_start();
        if keyword(line, "import").is_none() && keyword(line, "from").is_none() {
            continue;
        }
        let mut statement = line.to_string();
        while unfinished(&mut statement) {
            let Some(more) = code.next() else { break };
            statement.push(' ');
            statement.push_str(more);
        }
        for part in statement.split(';') {
            read_statement(part.trim(), &mut imports);
        }
    }
    imports
}

/// An import statement holds no string, so `#` always starts a comment.
fn without_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or(line)
}

/// Whether the statement goes on in the next line: it ends in a backslash,
/// which is dropped, or a parenthesis is still open.
fn unfinished(statement: &mut String) -> bool {
    let written = statement.trim_end();
    if written.ends_with('\\') {
        statement.truncate(written.len() - 1);
        return true;
    }
    statement.matches('(').count() > statement.matches(')').count()
}

/// What follows `word` when `text` starts with it as a whole word.
fn keyword<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(word)?;
    let apart = |c: char| !(c.is_alphanumeric() || c == '_');
    rest.starts_with(apart).then_some(rest)
}

/// Add what one statement imports, if it is an import.
fn read_statement(statement: &str, imports: &mut Vec<String>) {
    if let Some(listed) = keyword(statement, "import") {
        imports.extend(
            names(listed)
                .filter(|name| is_module(name))
                .map(str::to_string),
        );
        return;
    }
    let Some(rest) = keyword(statement, "from") else {
        return;
    };
    let Some((module, rest)) = rest.trim_start().split_once(char::is_whitespace) else {
        return;
    };
    let Some(listed) = keyword(rest.trim_start(), "import") else {
        return;
    };
    let path = module.trim_start_matches('.');
    if !(is_module(path) || path.is_empty() && !module.is_empty()) {
        return;
    }
    for name in names(listed) {
        if name == "*" {
            imports.push(module.to_string());
        } else if is_identifier(name) {
            imports.push(format!("{module}:{name}"));
        }
    }
}

/// The names of a comma-separated list, each without its `as` alias, and
/// without the parentheses around the list.
fn names(listed: &str) -> impl Iterator<Item = &str> {
    let around = |c: char| c.is_whitespace() || c == '(' || c == ')';
    listed
        .split(',')
        .filter_map(move |item| item.trim_matches(around).split_whitespace().next())
}

fn is_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn is_module(path: &str) -> bool {
    path.split('.').all(is_identifier)
}

/// Resolve what `extract` gives to the project file it names, if any.
///
/// A dotted path names `a/b/c.py`, or the package `a/b/c/__init__.py`; a
/// stub (`.pyi`) stands for a module that has no source. Only the deepest
/// module is the one used: the packages above it are not. For `a.b:c` the
/// submodule `a.b.c` comes first, then `a.b` as the module that defines `c`.
///
/// A relative import starts at the importer's package, one level up per
/// extra dot. An absolute one is looked up from each directory above the
/// importer that is not itself a package, nearest first, and from the `src`
/// directory under each: the roots a project is run or installed from. A
/// name found under none of them is external.
pub fn resolve(importer: &Path, import: &str, known: &HashSet<PathBuf>) -> Option<PathBuf> {
    let (module, name) = match import.split_once(':') {
        Some((module, name)) => (module, Some(name)),
        None => (import, None),
    };
    let path = module.trim_start_matches('.');
    let dots = module.len() - path.len();
    let module: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
    let submodule: Option<Vec<&str>> = name.map(|n| module.iter().copied().chain([n]).collect());

    let dir = importer.parent().unwrap_or(Path::new(""));
    let roots: Vec<PathBuf> = match dots {
        0 => roots(dir, known),
        _ => dir
            .ancestors()
            .nth(dots - 1)
            .map(Path::to_path_buf)
            .into_iter()
            .collect(),
    };
    roots.iter().find_map(|root| {
        submodule
            .iter()
            .chain([&module])
            // Only a relative import can name the package it starts at.
            .filter(|segments| dots > 0 || !segments.is_empty())
            .find_map(|segments| module_file(root, segments, known))
    })
}

/// The directories an absolute import written in `dir` may be looked up
/// from, nearest first.
fn roots(dir: &Path, known: &HashSet<PathBuf>) -> Vec<PathBuf> {
    dir.ancestors()
        .filter(|above| module_file(above, &[], known).is_none())
        .flat_map(|above| [above.to_path_buf(), above.join("src")])
        .collect()
}

/// The file of the module `segments` name under `root`: a module of its
/// own, or the `__init__` of a package. No segments name `root` itself.
fn module_file(root: &Path, segments: &[&str], known: &HashSet<PathBuf>) -> Option<PathBuf> {
    let package = segments.iter().fold(root.to_path_buf(), |p, s| p.join(s));
    ["py", "pyi"]
        .iter()
        .flat_map(|ext| {
            let file = segments.split_last().map(|(last, above)| {
                let dir = above.iter().fold(root.to_path_buf(), |p, s| p.join(s));
                dir.join(format!("{last}.{ext}"))
            });
            file.into_iter()
                .chain([package.join(format!("__init__.{ext}"))])
        })
        .find(|path| known.contains(path))
}

#[cfg(test)]
#[path = "python_test.rs"]
mod tests;
