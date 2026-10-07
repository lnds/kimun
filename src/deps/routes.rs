//! Routes of a Phoenix router, and the requests a test makes.
//!
//! A controller or a live view is tested through its route: the test asks
//! for a path and never names the module. The router is what relates the
//! two. Both are read lexically, one statement a line, as `mix format`
//! leaves them.

use super::elixir::{code_only, interpolation_end, is_ident, module_name};

/// A path a router serves, and the module it hands the request to.
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    pub path: String,
    pub module: String,
}

/// Macros that route a path to a module.
const VERBS: &[&str] = &[
    "live", "get", "post", "put", "patch", "delete", "head", "options",
];

/// Calls a test requests a path with.
const REQUESTS: &[&str] = &[
    "live", "get", "post", "put", "patch", "delete", "head", "options", "visit",
];

/// What stands for a segment written at run time: `#{id}` in a request.
const ANY: &str = ":_";

/// The first string literal of `line`, without its quotes.
fn first_string(line: &str) -> Option<&str> {
    let rest = &line[line.find('"')? + 1..];
    Some(&rest[..rest.find('"')?])
}

/// The first module named in `code`, a line with literals blanked.
fn first_module(code: &str) -> Option<&str> {
    let mut before = ' ';
    for (at, c) in code.char_indices() {
        let starts = !is_ident(before) && before != ':' && before != '.';
        let name = module_name(&code[at..]);
        if starts && !name.is_empty() {
            return Some(name);
        }
        before = c;
    }
    None
}

/// The first word of a line of code.
fn keyword(content: &str) -> &str {
    &content[..content.find(|c| !is_ident(c)).unwrap_or(content.len())]
}

/// `parts` as one path: `/api` and `/orders/:id` make `/api/orders/:id`.
fn join<'a>(parts: impl Iterator<Item = &'a str>) -> String {
    let segments: Vec<&str> = parts
        .flat_map(|part| part.split('/'))
        .filter(|s| !s.is_empty())
        .collect();
    format!("/{}", segments.join("/"))
}

/// A `scope` open at some point of a router.
struct Scope<'a> {
    indent: usize,
    path: &'a str,
    alias: Option<&'a str>,
}

/// The routes a router declares. Each `scope` adds its path and its alias
/// to what is inside it; nesting is told by indentation.
pub fn routes(source: &str) -> Vec<Route> {
    let code = code_only(source);
    let mut scopes: Vec<Scope> = Vec::new();
    let mut routes = Vec::new();

    for (raw, line) in source.lines().zip(code.lines()) {
        let content = line.trim_start();
        if content.is_empty() {
            continue;
        }
        let indent = line.len() - content.len();
        while scopes.last().is_some_and(|scope| scope.indent >= indent) {
            scopes.pop();
        }
        let word = keyword(content);
        let after = &content[word.len()..];
        if word == "scope" {
            scopes.push(Scope {
                indent,
                path: first_string(raw).unwrap_or_default(),
                alias: first_module(after),
            });
        } else if VERBS.contains(&word) || word == "resources" {
            let (Some(path), Some(module)) = (first_string(raw), first_module(after)) else {
                continue;
            };
            let aliases = scopes.iter().filter_map(|scope| scope.alias);
            let module = aliases.chain([module]).collect::<Vec<_>>().join(".");
            let under = |tail: &str| join(scopes.iter().map(|s| s.path).chain([path, tail]));
            // What `resources` serves besides the collection itself.
            let tails: &[&str] = match word {
                "resources" => &["", "new", ":id", ":id/edit"],
                _ => &[""],
            };
            routes.extend(tails.iter().map(|tail| Route {
                path: under(tail),
                module: module.clone(),
            }));
        }
    }
    routes
}

/// The path literal at the start of `text`, which follows its opening
/// quote: up to the closing one, with every `#{...}` turned into `ANY`.
fn path_literal(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut path = String::new();
    let mut i = 0;
    while i < chars.len() && chars[i] != '"' {
        if chars[i..].starts_with(&['#', '{']) {
            path.push_str(ANY);
            i = interpolation_end(&chars, i + 2);
        } else {
            path.push(chars[i]);
            i += 1;
        }
    }
    path
}

/// The paths a test requests: the first path literal of each call that
/// makes a request, as `get(conn, ~p"/orders/#{order}")`. Other literals
/// are left out: the path a redirect is asserted to lead to is not asked for.
pub fn requests(source: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in source.lines() {
        for (at, _) in line.match_indices('(') {
            let before = &line[..at];
            let name = &before[before.rfind(|c| !is_ident(c)).map_or(0, |i| i + 1)..];
            if !REQUESTS.contains(&name) {
                continue;
            }
            let arguments = &line[at..];
            if let Some(quote) = arguments.find("\"/") {
                paths.push(path_literal(&arguments[quote + 1..]));
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// The segments of a path, without its query or fragment.
fn segments(path: &str) -> Vec<&str> {
    let end = path.find(['?', '#']).unwrap_or(path.len());
    path[..end].split('/').filter(|s| !s.is_empty()).collect()
}

/// Whether a request for `request` is one the route at `route` serves:
/// segment by segment, a parameter of the route or a segment the request
/// writes at run time standing for any one.
pub fn serves(route: &str, request: &str) -> bool {
    let (route, request) = (segments(route), segments(request));
    let glob = route.iter().position(|s| s.starts_with('*'));
    let fixed = glob.unwrap_or(route.len());
    let long_enough = match glob {
        Some(_) => request.len() >= fixed,
        None => request.len() == fixed,
    };
    long_enough
        && route[..fixed]
            .iter()
            .zip(&request)
            .all(|(r, q)| r.starts_with(':') || q.contains(ANY) || r == q)
}

#[cfg(test)]
#[path = "routes_test.rs"]
mod tests;
