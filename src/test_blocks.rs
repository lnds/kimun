//! Inline test blocks of Rust: the items under `#[cfg(test)]`.
//!
//! An analysis that leaves tests out skips those items and nothing else:
//! the code that follows a test module is production code.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::deps::rust_literals::code_and_strings;
use crate::loc::counter::LineKind;
use crate::loc::language::LanguageSpec;

const ATTRIBUTE: &str = "#[cfg(test)]";

/// Which lines belong to an item under `#[cfg(test)]`, the attribute
/// included. Comments and literals are read apart, so an attribute or a
/// brace inside one counts for nothing.
pub fn test_block_mask(lines: &[String]) -> Vec<bool> {
    let mut mask = vec![false; lines.len()];
    for (_, from, to) in test_items(lines) {
        mask[from..=to].fill(true);
    }
    mask
}

/// Each item under `#[cfg(test)]`: its code without comments and literals,
/// and its first and last lines.
fn test_items(lines: &[String]) -> Vec<(String, usize, usize)> {
    if !lines.iter().any(|line| line.contains(ATTRIBUTE)) {
        return Vec::new();
    }
    let (code, _) = code_and_strings(&lines.join("\n"));
    let code: Vec<&str> = code.split('\n').collect();
    let mut items = Vec::new();
    let mut i = 0;
    while i < code.len() {
        if !code[i].trim_start().starts_with(ATTRIBUTE) {
            i += 1;
            continue;
        }
        let end = item_end(&code, i);
        items.push((code[i..=end].join("\n"), i, end));
        i = end + 1;
    }
    items
}

/// The files of the Rust modules declared under `#[cfg(test)]` without a
/// body (`#[cfg(test)] mod testutil;`), among `files`. Their code is test
/// support, though their names do not say so.
pub fn rust_test_modules(files: &[(PathBuf, &LanguageSpec)]) -> HashSet<PathBuf> {
    let known: HashSet<&Path> = files.iter().map(|(path, _)| path.as_path()).collect();
    files
        .iter()
        .filter(|(_, spec)| spec.name == "Rust")
        .flat_map(|(path, _)| declared_test_modules(path))
        .filter(|candidate| known.contains(candidate.as_path()))
        .collect()
}

/// The files a Rust file may load for the modules it declares under
/// `#[cfg(test)]` without a body: each module in either layout, or where
/// its `#[path]` says.
fn declared_test_modules(path: &Path) -> Vec<PathBuf> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    if !text.contains(ATTRIBUTE) {
        return Vec::new();
    }
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    let mut candidates = Vec::new();
    for (code, from, to) in test_items(&lines) {
        let Some(name) = declared_module(&code) else {
            continue;
        };
        candidates.extend(module_files(path, &name, &lines[from..=to]));
    }
    candidates
}

/// Where the module `name`, declared in `path` by the item of `item_lines`,
/// may live.
fn module_files(path: &Path, name: &str, item_lines: &[String]) -> Vec<PathBuf> {
    if let Some(target) = path_attribute(item_lines) {
        return vec![path.parent().unwrap_or(Path::new("")).join(target)];
    }
    let dir = module_dir(path);
    vec![
        dir.join(format!("{name}.rs")),
        dir.join(name).join("mod.rs"),
    ]
}

/// The name of the module an item declares without a body: `mod name;`,
/// with any visibility.
fn declared_module(code: &str) -> Option<String> {
    let code = code.trim_end();
    let decl = code.strip_suffix(';')?;
    let at = decl.rfind("mod ")?;
    let before = decl[..at].chars().last();
    if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let name = decl[at + 4..].trim();
    let ident = name.chars().all(|c| c.is_alphanumeric() || c == '_');
    (ident && !name.is_empty()).then(|| name.to_string())
}

/// The target of a `#[path = "…"]` attribute among the lines of an item.
fn path_attribute(lines: &[String]) -> Option<String> {
    lines.iter().find_map(|line| {
        let rest = line.trim_start().strip_prefix("#[path")?;
        let start = rest.find('"')? + 1;
        let end = start + rest[start..].find('"')?;
        Some(rest[start..end].to_string())
    })
}

/// The directory where the submodules of a Rust file live: beside a crate
/// root or a `mod.rs`, under a directory named after any other file.
fn module_dir(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or(Path::new(""));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let is_root = matches!(stem, "lib" | "main" | "mod")
        || parent.file_name().is_some_and(|dir| dir == "bin");
    if is_root {
        parent.to_path_buf()
    } else {
        parent.join(stem)
    }
}

/// The last line of the item that starts at `from`: where its braces
/// balance, or where it ends without any (`mod tests;`, a `use`, a field).
fn item_end(code: &[&str], from: usize) -> usize {
    let mut braces = 0usize;
    let mut parens = 0usize;
    let mut where_clause = false;
    for (offset, line) in code[from..].iter().enumerate() {
        let at = from + offset;
        for ch in line.chars() {
            match ch {
                '(' | '[' => parens += 1,
                ')' | ']' => parens = parens.saturating_sub(1),
                '{' => braces += 1,
                // The block around the item closes: it ended a line before.
                '}' if braces == 0 => return at.saturating_sub(1).max(from),
                '}' => {
                    braces -= 1;
                    if braces == 0 {
                        return at;
                    }
                }
                _ => {}
            }
        }
        where_clause |= line.trim_start().starts_with("where");
        if braces == 0 && parens == 0 && closes_item(line, where_clause) {
            return at;
        }
    }
    code.len() - 1
}

/// Whether a line outside braces and parentheses ends an item: with `;`,
/// or with the `,` of a field or variant. The bounds of a `where` clause
/// end in commas too, and the body comes after them.
fn closes_item(line: &str, where_clause: bool) -> bool {
    let line = line.trim_end();
    line.ends_with(';') || (line.ends_with(',') && !where_clause)
}

/// `kinds` with the lines of inline test blocks turned into blanks, so
/// that an analysis of code lines skips them.
pub fn without_test_blocks(lines: &[String], kinds: &[LineKind]) -> Vec<LineKind> {
    let mask = test_block_mask(lines);
    let demote =
        |(kind, in_test): (&LineKind, &bool)| if *in_test { LineKind::Blank } else { *kind };
    kinds.iter().zip(&mask).map(demote).collect()
}

/// `kinds` as an analysis reads them: without the inline test blocks when
/// it leaves tests out. Only Rust keeps tests inside a source file.
pub fn production_kinds(
    lines: &[String],
    kinds: Vec<LineKind>,
    spec: &LanguageSpec,
    exclude_tests: bool,
) -> Vec<LineKind> {
    if exclude_tests && spec.name == "Rust" {
        without_test_blocks(lines, &kinds)
    } else {
        kinds
    }
}

#[cfg(test)]
#[path = "test_blocks_test.rs"]
pub(crate) mod tests;
