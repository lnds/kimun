//! Kaikai import extraction and resolution.
//!
//! An import is a top-level `import a.b.c` line whose dotted path names a
//! file under a package root rather than under the importing file.
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::loc::counter::{LineKind, classify_reader};
use crate::loc::language::languages;
use crate::string_mask::multi_line_string_mask;

/// Extract the dotted module path of each `import` line
/// (`import a.b.c`, `import a.b as x`, `import a.b.{f, g}`).
///
/// The line classifier and the multi-line string mask are reused so that an
/// `import` written inside a `#[doc(...)]` block or a triple-quoted string is
/// not taken for a real one.
pub fn extract(source: &str) -> Vec<String> {
    let Some(spec) = languages().iter().find(|s| s.name == "Kaikai") else {
        return vec![];
    };
    let normalized = source.replace("\r\n", "\n");
    let lines: Vec<String> = normalized.lines().map(String::from).collect();
    let kinds = classify_reader(normalized.as_bytes(), spec);
    let in_string = multi_line_string_mask(&lines, spec);

    lines
        .iter()
        .zip(kinds)
        .zip(in_string)
        .filter(|((_, kind), masked)| *kind == LineKind::Code && !masked)
        .flat_map(|((line, _), _)| statements(line).filter_map(import_path))
        .map(str::to_string)
        .collect()
}

/// The statements of a line of code: before its comment, separated by `;`.
fn statements(line: &str) -> impl Iterator<Item = &str> {
    line.split('#').next().unwrap_or(line).split(';')
}

/// The module path of an `import` statement, without its alias or its
/// selective list: `import a.b.{f, g}` → `a.b`.
fn import_path(statement: &str) -> Option<&str> {
    let rest = statement.trim_start().strip_prefix("import")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
        .unwrap_or(rest.len());
    // The selective form leaves the dot that precedes `{`.
    let path = rest[..end].trim_end_matches('.');
    // An empty path splits into a single empty segment, so this rejects it too.
    path.split('.').all(|seg| !seg.is_empty()).then_some(path)
}

/// Resolve a module path to the project files it names.
///
/// `import a.b.c` names `a/b/c.kai` under a package root, not under the
/// importing file's directory. The source does not say where that root is, so
/// every ancestor directory of the importer is tried, nearest first.
///
/// When no ancestor holds that file, the import may name a package directory
/// `a/b/c/` instead, whose directory the compiler searches: the import names
/// `a/b/c/c.kai`, the file that carries the package's name. A package
/// without one resolves to all the `.kai` files directly inside it. Only a
/// directory with a `kai.toml` manifest counts: a plain directory that
/// happens to share its name with an external module (`import loop` next to
/// `loop/`) is not one.
///
/// Each file is a module of its own, and the files of a package import each
/// other as any other module, so they have no edges besides their imports.
pub fn resolve(importer: &Path, module: &str, file_set: &HashSet<PathBuf>) -> Vec<PathBuf> {
    let dir = importer.parent().unwrap_or(Path::new(""));
    let rel = module.replace('.', "/");
    let as_file = dir
        .ancestors()
        .map(|root| root.join(format!("{rel}.kai")))
        .find(|file| file_set.contains(file));
    if let Some(file) = as_file {
        return vec![file];
    }
    let last = module.rsplit('.').next().unwrap_or(module);
    let packages = dir.ancestors().map(|root| root.join(&rel));
    packages
        .filter(|package| file_set.contains(&package.join("kai.toml")))
        .map(|package| package_files(&package, last, importer, file_set))
        .find(|files| !files.is_empty())
        .unwrap_or_default()
}

/// The files an import of the package directory `package` names, besides
/// the importer: the one named `last`, or else every one inside.
fn package_files(
    package: &Path,
    last: &str,
    importer: &Path,
    file_set: &HashSet<PathBuf>,
) -> Vec<PathBuf> {
    let named = package.join(format!("{last}.kai"));
    if file_set.contains(&named) {
        return [named].into_iter().filter(|f| f != importer).collect();
    }
    let mut members: Vec<PathBuf> = file_set
        .iter()
        .filter(|p| p.parent() == Some(package) && p.extension().is_some_and(|e| e == "kai"))
        .filter(|p| p.as_path() != importer)
        .cloned()
        .collect();
    members.sort();
    members
}

#[cfg(test)]
#[path = "kaikai_test.rs"]
mod tests;
