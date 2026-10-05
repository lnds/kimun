//! Readers for `go.mod` and `go.work`.

use std::path::PathBuf;

use super::{DepTarget, Manifest, RawDep, Scope};

/// The entries of the directive `keyword` in a Go module file: those on its
/// own line (`require x v1`) and those of its block (`require ( ... )`).
/// Comments are dropped.
fn entries<'a>(source: &'a str, keyword: &'a str) -> Vec<&'a str> {
    let mut found = Vec::new();
    let mut in_block = false;
    for line in source.lines() {
        let line = line.split("//").next().unwrap_or_default().trim();
        if in_block {
            match line {
                ")" => in_block = false,
                "" => {}
                entry => found.push(entry),
            }
            continue;
        }
        let Some(rest) = line
            .strip_prefix(keyword)
            .filter(|r| r.starts_with([' ', '\t', '(']))
        else {
            continue;
        };
        match rest.trim() {
            "(" => in_block = true,
            entry => found.push(entry),
        }
    }
    found
}

fn dep(target: DepTarget) -> RawDep {
    RawDep {
        target,
        scope: Scope::Runtime,
    }
}

/// The local directory a `replace` points a module at, if it does:
/// `old => ../dir` does, `old => other v1.2.3` does not.
fn replaced_by_directory(entry: &str) -> Option<PathBuf> {
    let (_, new) = entry.split_once("=>")?;
    let target = new.split_whitespace().next()?;
    target
        .starts_with(['.', '/'])
        .then(|| PathBuf::from(target))
}

/// Read a `go.mod`. A required module is local when another module of the
/// repository has its path, which a `go.work` or a `replace` makes resolve
/// there; a `replace` with a directory names it outright.
pub fn read(source: &str) -> Manifest {
    let required = entries(source, "require")
        .into_iter()
        .filter_map(|entry| entry.split_whitespace().next())
        .map(|module| dep(DepTarget::Name(module.to_string())));
    let replaced = entries(source, "replace")
        .into_iter()
        .filter_map(replaced_by_directory)
        .map(|dir| dep(DepTarget::Path(dir)));

    Manifest {
        name: entries(source, "module")
            .first()
            .and_then(|entry| entry.split_whitespace().next())
            .map(|path| path.trim_matches('"').to_string()),
        deps: required.chain(replaced).collect(),
        ..Manifest::default()
    }
}

/// Read a `go.work`: it only says its directory is the root of a workspace.
pub fn read_work(_source: &str) -> Manifest {
    Manifest {
        is_workspace_root: true,
        ..Manifest::default()
    }
}

#[cfg(test)]
#[path = "gomod_test.rs"]
mod tests;
