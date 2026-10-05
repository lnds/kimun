//! Reader for `Cargo.toml`.

use std::path::PathBuf;

use toml::{Table, Value};

use super::{DepTarget, Manifest, RawDep, Scope};

const SECTIONS: [(&str, Scope); 3] = [
    ("dependencies", Scope::Runtime),
    ("dev-dependencies", Scope::Dev),
    ("build-dependencies", Scope::Build),
];

/// The local target of one dependency entry, if it has one. A plain version
/// string or a git source names an external crate.
fn target(name: &str, spec: &Value) -> Option<DepTarget> {
    let spec = spec.as_table()?;
    if let Some(path) = spec.get("path").and_then(Value::as_str) {
        return Some(DepTarget::Path(PathBuf::from(path)));
    }
    spec.get("workspace")
        .and_then(Value::as_bool)
        .filter(|&inherited| inherited)
        .map(|_| DepTarget::Workspace(name.to_string()))
}

/// The local dependencies declared in the three sections of `table`.
fn deps_of(table: &Table) -> Vec<RawDep> {
    let mut deps = Vec::new();
    for (section, scope) in SECTIONS {
        let Some(entries) = table.get(section).and_then(Value::as_table) else {
            continue;
        };
        for (name, spec) in entries {
            let Some(target) = target(name, spec) else {
                continue;
            };
            let optional = spec.get("optional").and_then(Value::as_bool) == Some(true);
            deps.push(RawDep {
                target,
                scope: if optional {
                    scope.max(Scope::Optional)
                } else {
                    scope
                },
            });
        }
    }
    deps
}

/// Read a `Cargo.toml`. A manifest without `[package]` is a virtual
/// workspace root and declares no project.
pub fn read(source: &str) -> Manifest {
    let Ok(doc) = source.parse::<Table>() else {
        return Manifest::default();
    };

    let mut deps = deps_of(&doc);
    // `[target.'cfg(...)'.dependencies]` and its siblings.
    for platform in doc
        .get("target")
        .and_then(Value::as_table)
        .into_iter()
        .flat_map(|targets| targets.values())
        .filter_map(Value::as_table)
    {
        deps.extend(deps_of(platform));
    }

    let workspace = doc.get("workspace").and_then(Value::as_table);
    let workspace_paths = workspace
        .and_then(|w| w.get("dependencies"))
        .and_then(Value::as_table)
        .into_iter()
        .flatten()
        .filter_map(|(name, spec)| {
            let path = spec.get("path")?.as_str()?;
            Some((name.clone(), PathBuf::from(path)))
        })
        .collect();

    Manifest {
        name: doc
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(Value::as_str)
            .map(str::to_string),
        deps,
        workspace_paths,
        is_workspace_root: workspace.is_some(),
        unread: 0,
    }
}

#[cfg(test)]
#[path = "cargo_test.rs"]
mod tests;
