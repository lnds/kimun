//! Reader for `pyproject.toml`.
//!
//! A requirement names a package and nothing else, so a project of the
//! repository is recognized by its name. Where it lives is said apart, by
//! the tool that installs it: `[tool.uv.sources]`, or a `path` in the
//! dependency tables of Poetry.

use std::collections::HashMap;
use std::path::PathBuf;

use toml::{Table, Value};

use super::{DepTarget, Ecosystem, Manifest, RawDep, Scope};

/// Files a `path` may point at instead of a project.
const ARCHIVES: [&str; 3] = [".whl", ".tar.gz", ".zip"];

/// The value at the end of a chain of tables.
fn at<'a>(doc: &'a Table, keys: &[&str]) -> Option<&'a Value> {
    let (first, rest) = keys.split_first()?;
    rest.iter()
        .try_fold(doc.get(*first)?, |value, key| value.get(*key))
}

/// The package a requirement such as `core[extra]>=1.0; python_version>"3"`
/// names. An option of pip (`-e ../core`) names none.
fn requirement_name(requirement: &str) -> Option<&str> {
    let requirement = requirement.trim_start();
    let end = requirement
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
        .unwrap_or(requirement.len());
    let name = &requirement[..end];
    name.starts_with(|c: char| c.is_ascii_alphanumeric())
        .then_some(name)
}

/// The directory a dependency entry points at. An entry is a table, or
/// several of them when the source depends on the platform.
fn directory(spec: &Value) -> Option<PathBuf> {
    match spec {
        Value::Table(table) => table
            .get("path")?
            .as_str()
            .filter(|path| !ARCHIVES.iter().any(|ext| path.ends_with(ext)))
            .map(PathBuf::from),
        Value::Array(specs) => specs.iter().find_map(directory),
        _ => None,
    }
}

/// What a manifest declares, before names are matched with directories.
#[derive(Default)]
struct Declared {
    deps: Vec<(String, Scope)>,
    /// Package name, as names compare, → its directory.
    directories: HashMap<String, PathBuf>,
}

impl Declared {
    /// A list of requirements. An entry that is not a string includes
    /// another group, which is read on its own.
    fn requirements(&mut self, list: Option<&Value>, scope: Scope) {
        let names = list
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(requirement_name);
        self.deps
            .extend(names.map(|name| (name.to_string(), scope)));
    }

    /// A table of named lists of requirements.
    fn groups(&mut self, table: Option<&Value>, scope: Scope) {
        for list in table.and_then(Value::as_table).into_iter().flatten() {
            self.requirements(Some(list.1), scope);
        }
    }

    /// A table of where packages are found, by name.
    fn sources(&mut self, table: Option<&Value>) {
        for (name, spec) in table.and_then(Value::as_table).into_iter().flatten() {
            if let Some(dir) = directory(spec) {
                self.directories.insert(Ecosystem::Python.key(name), dir);
            }
        }
    }

    /// A dependency table of Poetry, where an entry is both the requirement
    /// and its source. `python` is the interpreter, not a package.
    fn poetry(&mut self, table: Option<&Value>, scope: Scope) {
        self.sources(table);
        for (name, spec) in table.and_then(Value::as_table).into_iter().flatten() {
            if name == "python" {
                continue;
            }
            let optional = spec.get("optional").and_then(Value::as_bool) == Some(true);
            let scope = if optional {
                scope.max(Scope::Optional)
            } else {
                scope
            };
            self.deps.push((name.clone(), scope));
        }
    }
}

/// Read a `pyproject.toml`. One that only configures tools declares no
/// project.
pub fn read(source: &str) -> Manifest {
    let Ok(doc) = source.parse::<Table>() else {
        return Manifest::default();
    };

    let mut declared = Declared::default();
    declared.requirements(at(&doc, &["project", "dependencies"]), Scope::Runtime);
    declared.groups(
        at(&doc, &["project", "optional-dependencies"]),
        Scope::Optional,
    );
    declared.groups(at(&doc, &["dependency-groups"]), Scope::Dev);
    declared.requirements(at(&doc, &["tool", "uv", "dev-dependencies"]), Scope::Dev);
    declared.requirements(at(&doc, &["build-system", "requires"]), Scope::Build);
    declared.sources(at(&doc, &["tool", "uv", "sources"]));

    declared.poetry(
        at(&doc, &["tool", "poetry", "dependencies"]),
        Scope::Runtime,
    );
    declared.poetry(
        at(&doc, &["tool", "poetry", "dev-dependencies"]),
        Scope::Dev,
    );
    let groups = at(&doc, &["tool", "poetry", "group"]).and_then(Value::as_table);
    for group in groups.into_iter().flatten() {
        declared.poetry(group.1.get("dependencies"), Scope::Dev);
    }

    let deps = declared
        .deps
        .iter()
        .map(|(name, scope)| RawDep {
            target: match declared.directories.get(&Ecosystem::Python.key(name)) {
                Some(dir) => DepTarget::Path(dir.clone()),
                None => DepTarget::Name(name.clone()),
            },
            scope: *scope,
        })
        .collect();

    Manifest {
        name: at(&doc, &["project", "name"])
            .or_else(|| at(&doc, &["tool", "poetry", "name"]))
            .and_then(Value::as_str)
            .map(str::to_string),
        deps,
        is_workspace_root: at(&doc, &["tool", "uv", "workspace"]).is_some(),
        ..Manifest::default()
    }
}

/// The strings of a value that is one string or a list of them.
fn strings(value: Option<&Value>) -> Vec<&str> {
    match value {
        Some(Value::String(one)) => vec![one.as_str()],
        Some(Value::Array(many)) => many.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    }
}

/// The directories, relative to the manifest, a `pyproject.toml` says its
/// packages are imported from: where the build backend finds them, and what
/// pytest adds to the search path.
pub fn import_roots(source: &str) -> Vec<PathBuf> {
    let Ok(doc) = source.parse::<Table>() else {
        return Vec::new();
    };
    let tool = |keys: &[&str]| at(&doc, &[&["tool"], keys].concat());

    let mut roots = strings(tool(&["setuptools", "packages", "find", "where"]));
    roots.extend(strings(tool(&["setuptools", "package-dir", ""])));
    roots.extend(strings(tool(&["pytest", "ini_options", "pythonpath"])));
    roots.extend(strings(tool(&["maturin", "python-source"])));
    roots.extend(strings(tool(&["pdm", "build", "package-dir"])));
    roots.extend(strings(tool(&["uv", "build-backend", "module-root"])));
    let included = tool(&["poetry", "packages"]).and_then(Value::as_array);
    roots.extend(
        included
            .into_iter()
            .flatten()
            .filter_map(|package| package.get("from")?.as_str()),
    );
    let mut roots: Vec<PathBuf> = roots.into_iter().map(PathBuf::from).collect();

    // Hatch lists the roots, or the packages themselves, for a target or
    // for every one.
    for build in [
        &["hatch", "build"][..],
        &["hatch", "build", "targets", "wheel"],
    ] {
        let key = |name: &str| tool(&[build, &[name]].concat());
        match key("sources") {
            Some(Value::Table(renamed)) => roots.extend(renamed.keys().map(PathBuf::from)),
            listed => roots.extend(strings(listed).into_iter().map(PathBuf::from)),
        }
        let packages = strings(key("packages"));
        roots.extend(
            packages
                .into_iter()
                .filter_map(|package| Some(PathBuf::from(package).parent()?.to_path_buf())),
        );
    }

    let mut seen = Vec::new();
    for root in roots {
        if !seen.contains(&root) {
            seen.push(root);
        }
    }
    seen
}

#[cfg(test)]
#[path = "pyproject_test.rs"]
mod tests;
