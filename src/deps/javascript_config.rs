//! What the configs of a JavaScript or TypeScript repository say about
//! imports: the aliases of `tsconfig.json` and `jsconfig.json`, and the
//! packages `package.json` names.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::javascript::{find, line_len, normalize};

/// The aliases that govern the files under a directory.
#[derive(Debug, Default, PartialEq)]
pub struct Config {
    pub dir: PathBuf,
    /// Where non-relative specifiers are looked up, from the repository root.
    pub base_url: Option<PathBuf>,
    /// Each pattern of `paths`, with its targets from the repository root.
    pub paths: Vec<(String, Vec<String>)>,
}

/// A package of the repository, by the name its `package.json` declares.
#[derive(Debug, PartialEq)]
pub struct Package {
    pub dir: PathBuf,
    pub name: String,
    /// Its entry points, as `source`, `module`, `main`, `types` and the
    /// root of `exports` give them.
    pub entries: Vec<String>,
}

/// Whether a file is one these are read from: a `package.json` or a
/// config with aliases, not of an installed package.
pub fn is_manifest(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let installed = path.components().any(|c| c.as_os_str() == "node_modules");
    !installed && (name == "package.json" || is_config(name))
}

/// What the JavaScript manifests of a repository say about imports.
#[derive(Default)]
pub struct Modules {
    /// The aliases of `tsconfig.json` and `jsconfig.json`, by directory.
    pub configs: Vec<Config>,
    /// The packages of the repository.
    pub packages: Vec<Package>,
}

/// What the manifests given as path and text declare.
pub fn read<'t>(manifests: Vec<(&'t Path, &'t str)>) -> Modules {
    let (packages, configs_read): (Vec<_>, Vec<_>) = manifests
        .into_iter()
        .partition(|(path, _)| path.file_name().is_some_and(|n| n == "package.json"));
    Modules {
        configs: configs(configs_read),
        packages: packages
            .into_iter()
            .filter_map(|(p, t)| package(p, t))
            .collect(),
    }
}

/// Whether a file is a config that holds aliases: `tsconfig.json`,
/// `jsconfig.json`, or one of their variants such as `tsconfig.base.json`.
pub fn is_config(name: &str) -> bool {
    (name.starts_with("tsconfig") || name.starts_with("jsconfig")) && name.ends_with(".json")
}

/// The package a `package.json` declares, if it has a name.
pub fn package(path: &Path, text: &str) -> Option<Package> {
    let doc: Value = serde_json::from_str(text).ok()?;
    let name = doc.get("name")?.as_str()?.to_string();
    let mut entries: Vec<String> = ["source", "module", "main", "types", "typings"]
        .iter()
        .filter_map(|field| doc.get(field)?.as_str().map(str::to_string))
        .collect();
    let root = doc.get("exports").map(|e| e.get(".").unwrap_or(e));
    entries.extend(root.into_iter().flat_map(strings));
    Some(Package {
        dir: path.parent().unwrap_or(Path::new("")).to_path_buf(),
        name,
        entries,
    })
}

/// The strings of a value of `exports`, conditions first to last.
fn strings(value: &Value) -> Vec<String> {
    match value {
        Value::String(s) => vec![s.clone()],
        Value::Object(map) => map.values().flat_map(strings).collect(),
        Value::Array(items) => items.iter().flat_map(strings).collect(),
        _ => Vec::new(),
    }
}

/// What one config file declares, before `extends` is followed.
struct Raw {
    extends: Vec<PathBuf>,
    base_url: Option<PathBuf>,
    paths: Option<Vec<(String, Vec<String>)>>,
}

/// The configs of a repository, one per directory that has any, from the
/// config files given as path and text. A file inherits `baseUrl` and
/// `paths` from those it `extends`; `paths` targets are taken from
/// `baseUrl`, or else from the file that declares them. In a directory with
/// several, `tsconfig.json` and `jsconfig.json` come first, and each
/// pattern is taken from the first that declares it.
pub fn configs<'t>(files: impl IntoIterator<Item = (&'t Path, &'t str)>) -> Vec<Config> {
    let raws: BTreeMap<PathBuf, Raw> = files
        .into_iter()
        .filter_map(|(path, text)| Some((path.to_path_buf(), raw(path, text)?)))
        .collect();
    let mut by_dir: BTreeMap<PathBuf, Vec<&Path>> = BTreeMap::new();
    for path in raws.keys() {
        let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
        by_dir.entry(dir).or_default().push(path);
    }
    by_dir
        .into_iter()
        .filter_map(|(dir, paths)| merged(dir, paths, &raws))
        .collect()
}

/// The config of a directory, from its config files: `tsconfig.json` and
/// `jsconfig.json` first, each pattern from the first that declares it.
/// None when they declare no aliases.
fn merged(dir: PathBuf, mut paths: Vec<&Path>, raws: &BTreeMap<PathBuf, Raw>) -> Option<Config> {
    paths.sort_by_key(|p| !is_main(p));
    let mut config = Config {
        dir,
        ..Config::default()
    };
    for path in paths {
        let (base_url, paths) = effective(path, raws, 0);
        config.base_url = config.base_url.or(base_url);
        for (pattern, targets) in paths {
            if !config.paths.iter().any(|(p, _)| *p == pattern) {
                config.paths.push((pattern, targets));
            }
        }
    }
    (config.base_url.is_some() || !config.paths.is_empty()).then_some(config)
}

fn is_main(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|n| n == "tsconfig.json" || n == "jsconfig.json")
}

/// How deep `extends` is followed, against cycles.
const MAX_EXTENDS: usize = 8;

/// The `baseUrl` and `paths` a config file ends up with, the latter with
/// their targets from the repository root.
fn effective(
    path: &Path,
    raws: &BTreeMap<PathBuf, Raw>,
    depth: usize,
) -> (Option<PathBuf>, Vec<(String, Vec<String>)>) {
    let Some(raw) = raws.get(path).filter(|_| depth < MAX_EXTENDS) else {
        return (None, Vec::new());
    };
    let dir = path.parent().unwrap_or(Path::new(""));
    let mut base_url = None;
    let mut paths = None;
    for parent in &raw.extends {
        let (b, p) = effective(parent, raws, depth + 1);
        base_url = b.or(base_url);
        if !p.is_empty() {
            paths = Some(p);
        }
    }
    let base_url = raw.base_url.clone().or(base_url);
    let paths = match &raw.paths {
        Some(own) => {
            let from = base_url.clone().unwrap_or_else(|| dir.to_path_buf());
            rooted(own, &from)
        }
        None => paths.unwrap_or_default(),
    };
    (base_url, paths)
}

/// `paths` with each target taken from `from`.
fn rooted(paths: &[(String, Vec<String>)], from: &Path) -> Vec<(String, Vec<String>)> {
    paths
        .iter()
        .map(|(pattern, targets)| {
            let targets = targets
                .iter()
                .filter_map(|t| normalize(&from.join(t)))
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            (pattern.clone(), targets)
        })
        .collect()
}

/// What a config file declares. Configs are JSON with comments and
/// trailing commas.
fn raw(path: &Path, text: &str) -> Option<Raw> {
    let doc: Value = serde_json::from_str(&strict_json(text)).ok()?;
    let dir = path.parent().unwrap_or(Path::new(""));
    let options = doc.get("compilerOptions");
    let extends = match doc.get("extends") {
        Some(Value::String(s)) => vec![s.as_str()],
        Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    let base_url = options
        .and_then(|o| o.get("baseUrl")?.as_str())
        .and_then(|b| normalize(&dir.join(b)));
    let paths = options
        .and_then(|o| o.get("paths")?.as_object())
        .map(|map| {
            map.iter()
                .map(|(pattern, targets)| {
                    let targets = targets.as_array().into_iter().flatten();
                    let targets = targets.filter_map(Value::as_str).map(str::to_string);
                    (pattern.clone(), targets.collect())
                })
                .collect()
        });
    Some(Raw {
        extends: extends.iter().filter_map(|e| extended(dir, e)).collect(),
        base_url,
        paths,
    })
}

/// The file an `extends` names, when it is one of the repository: a
/// relative path, with `.json` added when it has none.
fn extended(dir: &Path, target: &str) -> Option<PathBuf> {
    if !target.starts_with('.') {
        return None;
    }
    let path = normalize(&dir.join(target))?;
    if path.extension().is_some_and(|e| e == "json") {
        Some(path)
    } else {
        let mut name = path.into_os_string();
        name.push(".json");
        Some(PathBuf::from(name))
    }
}

/// JSON with its comments and trailing commas removed, strings untouched.
fn strict_json(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let (len, kept) = piece(&bytes[i..]);
        if kept {
            out.extend_from_slice(&bytes[i..i + len]);
        }
        i += len;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The piece of JSON that `rest` starts with, and whether it is kept: a
/// string, a comment, a comma or any other byte.
fn piece(rest: &[u8]) -> (usize, bool) {
    match rest {
        [b'"', ..] => (quoted_len(rest), true),
        [b'/', b'/', ..] => (line_len(rest), false),
        [b'/', b'*', ..] => (find(rest, b"*/").map_or(rest.len(), |at| at + 2), false),
        [b',', tail @ ..] => {
            let next = tail.iter().find(|c| !c.is_ascii_whitespace());
            (1, !matches!(next, Some(b'}' | b']')))
        }
        _ => (1, true),
    }
}

/// The length of the string literal `rest` starts with, quotes included.
fn quoted_len(rest: &[u8]) -> usize {
    let mut i = 1;
    while i < rest.len() && rest[i] != b'"' {
        i += if rest[i] == b'\\' { 2 } else { 1 };
    }
    (i + 1).min(rest.len())
}

#[cfg(test)]
#[path = "javascript_config_test.rs"]
mod tests;
