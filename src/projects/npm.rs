//! Reader for `package.json`.

use std::path::PathBuf;

use serde_json::Value;

use super::{DepTarget, Manifest, RawDep, Scope};

const SECTIONS: [(&str, Scope); 4] = [
    ("dependencies", Scope::Runtime),
    ("peerDependencies", Scope::Runtime),
    ("optionalDependencies", Scope::Optional),
    ("devDependencies", Scope::Dev),
];

/// How a dependency entry may be local: by an explicit directory, or by
/// carrying the name of another package of the repository. Workspaces link a
/// local package by name whatever the version range, so every other entry is
/// a candidate and is external only if no project has that name.
fn target(name: &str, spec: &str) -> DepTarget {
    match spec
        .strip_prefix("file:")
        .or_else(|| spec.strip_prefix("link:"))
    {
        Some(path) => DepTarget::Path(PathBuf::from(path)),
        None => DepTarget::Name(name.to_string()),
    }
}

/// Read a `package.json`.
pub fn read(source: &str) -> Manifest {
    let Ok(doc) = serde_json::from_str::<Value>(source) else {
        return Manifest::default();
    };

    let mut deps = Vec::new();
    for (section, scope) in SECTIONS {
        let Some(entries) = doc.get(section).and_then(Value::as_object) else {
            continue;
        };
        for (name, spec) in entries {
            if let Some(spec) = spec.as_str() {
                deps.push(RawDep {
                    target: target(name, spec),
                    scope,
                });
            }
        }
    }

    Manifest {
        name: doc.get("name").and_then(Value::as_str).map(str::to_string),
        deps,
        is_workspace_root: doc.get("workspaces").is_some(),
        ..Manifest::default()
    }
}

#[cfg(test)]
#[path = "npm_test.rs"]
mod tests;
