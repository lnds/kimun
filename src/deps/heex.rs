//! HEEx templates: the Elixir in them, and the module each belongs to.
//!
//! A template names the components it renders, so it is where a view uses
//! them. It has no module of its own: what it refers to is a use by the
//! module that renders it.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const EXTENSION: &str = ".html.heex";

pub fn is_template(path: &Path) -> bool {
    path.to_str().is_some_and(|p| p.ends_with(EXTENSION))
}

/// Where the module that renders the template at `path` may be, likeliest
/// first: beside it under the same name (`page/index.html.heex` and
/// `page/index.ex`), or beside its directory (`page_html/home.html.heex`
/// and `page_html.ex`).
pub fn owners(path: &Path) -> Vec<PathBuf> {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Vec::new();
    };
    let Some(dir) = path.parent() else {
        return Vec::new();
    };
    let beside = name
        .strip_suffix(EXTENSION)
        .map(|stem| dir.join(format!("{stem}.ex")));
    let above = dir.file_name().and_then(|d| d.to_str()).map(|d| {
        dir.parent()
            .unwrap_or(Path::new(""))
            .join(format!("{d}.ex"))
    });
    beside.into_iter().chain(above).collect()
}

/// The templates each of `sources` renders, by the path of the source. A
/// template whose module is not among them is dropped.
pub fn by_owner(
    templates: Vec<(PathBuf, String)>,
    sources: &[(PathBuf, String)],
) -> HashMap<PathBuf, Vec<String>> {
    let paths: HashSet<&Path> = sources.iter().map(|(path, _)| path.as_path()).collect();
    let mut rendered: HashMap<PathBuf, Vec<String>> = HashMap::new();
    for (path, text) in templates {
        let owner = owners(&path)
            .into_iter()
            .find(|owner| paths.contains(owner.as_path()));
        if let Some(owner) = owner {
            rendered.entry(owner).or_default().push(text);
        }
    }
    rendered
}

/// The `~H` templates written in `source`, delimiters included.
pub(super) fn embedded(source: &str) -> Vec<String> {
    let chars: Vec<char> = source.chars().collect();
    let mut templates = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let end = super::elixir::literal_end(&chars, i).min(chars.len());
        if end == i {
            i += 1;
            continue;
        }
        if chars[i..].starts_with(&['~', 'H']) {
            templates.push(chars[i + 2..end].iter().collect());
        }
        i = end;
    }
    templates
}

/// The index of `closer` at or after `from`, or the end.
fn find(chars: &[char], from: usize, closer: &[char]) -> usize {
    (from..chars.len())
        .find(|&i| chars[i..].starts_with(closer))
        .unwrap_or(chars.len())
}

/// The Elixir of a template, one piece a line: what is between `{` and `}`,
/// between `<%` and `%>`, and the names of the components called as tags
/// (`<Card.header`). Markup and text are left out: a word of prose must not
/// read as a module.
pub fn code(template: &str) -> String {
    let chars: Vec<char> = template.chars().collect();
    let mut out = String::new();
    let mut keep = |piece: &[char]| {
        out.extend(piece);
        out.push('\n');
    };
    let mut i = 0;
    while i < chars.len() {
        match chars[i..] {
            ['<', '!', '-', '-', ..] => i = find(&chars, i, &['-', '-', '>']),
            ['<', '%', '#', ..] | ['<', '%', '!', ..] => i = find(&chars, i, &['%', '>']),
            ['<', '%', ..] => {
                let end = find(&chars, i, &['%', '>']);
                let start = i + 2 + usize::from(chars.get(i + 2) == Some(&'='));
                keep(&chars[start.min(end)..end]);
                i = end;
            }
            ['<', c, ..] if c.is_ascii_uppercase() => {
                let len = chars[i + 1..]
                    .iter()
                    .take_while(|c| super::elixir::is_ident(**c) || **c == '.')
                    .count();
                keep(&chars[i + 1..i + 1 + len]);
                i += len;
            }
            ['{', ..] => {
                let end = super::elixir::interpolation_end(&chars, i + 1);
                keep(&chars[i + 1..end.saturating_sub(1).max(i + 1)]);
                i = end;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

#[cfg(test)]
#[path = "heex_test.rs"]
mod tests;
