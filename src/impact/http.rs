//! The HTTP level of the radius: routes a change removes or changes, and
//! the client calls elsewhere that ask for them.
//!
//! In a repository of several services the reach of a change often goes
//! through HTTP: one service serves a route, another calls it with a
//! client, and nothing in the code of one names the other. Routes are read
//! from Phoenix routers; calls from the clients `http_clients` knows. Base
//! URLs come from configuration, so a call is matched by the end of the
//! route's path: this level is heuristic, and says so.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::deps::elixir_modules;
use crate::deps::routes::{Declared, declared};
use crate::git::{ChangeKind, FileDiffStat};

use super::http_clients::{self, ClientCall};
use super::narrowing::Narrowing;
use super::reading::graphed_language;
use super::role::{Role, role};

/// What the change does to a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RouteChange {
    /// The router no longer declares it.
    Removed,
    /// Its declaration changed: the module or action behind it, its options.
    Changed,
    /// The action that serves it changed.
    Action,
}

/// A route the change concerns, and the calls that ask for it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChangedRoute {
    pub verb: String,
    pub path: String,
    pub router: PathBuf,
    pub line: usize,
    pub change: RouteChange,
    /// `Module.action`, or the module alone.
    pub serves: String,
    pub callers: Vec<Caller>,
}

/// A client call that asks for a changed route.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Caller {
    pub file: PathBuf,
    pub line: usize,
    pub client: String,
    pub path: String,
    /// The project the call is in, when the repository has projects.
    pub project: Option<String>,
    /// Whether the call asks for the whole path; otherwise for its end,
    /// past a base URL that is not seen.
    pub exact: bool,
    /// Whether a test is named after the file of the call.
    pub tested: bool,
}

/// The HTTP level of a change.
#[derive(Debug, Default, PartialEq)]
pub struct Http {
    pub routes: Vec<ChangedRoute>,
    /// Client calls outside tests whose path is built at run time: they may
    /// ask for any of the routes.
    pub unresolved: Vec<ClientCall>,
    /// Client calls read, outside tests.
    pub calls: usize,
}

/// Whether a source may hold routes or client calls.
pub fn is_read(path: &Path) -> bool {
    matches!(
        graphed_language(path),
        Some("Elixir" | "JavaScript" | "TypeScript" | "JSX" | "Python")
    )
}

fn is_router(path: &Path, text: &str) -> bool {
    graphed_language(path) == Some("Elixir")
        && (text.contains(":router") || text.contains("Phoenix.Router"))
}

/// `Module.action` for a route.
fn serves(route: &Declared) -> String {
    match &route.action {
        Some(action) => format!("{}.{action}", route.module),
        None => route.module.clone(),
    }
}

/// A route the change concerns, before its callers are found.
struct Concerned {
    route: Declared,
    router: PathBuf,
    change: RouteChange,
}

/// The routes a change to a router removes or changes. `head` is the
/// router after the change, if it is still there.
fn router_changes(stat: &FileDiffStat, head: Option<&str>) -> Vec<Concerned> {
    let after = head.unwrap_or_default();
    let before = match stat.kind {
        ChangeKind::Added => String::new(),
        ChangeKind::Deleted => stat.removed.join("\n"),
        _ => stat.before(after),
    };
    let (was, is) = (declared(&before), declared(after));
    let key = |d: &Declared| (d.verb.clone(), d.path.clone());
    let kept: HashSet<_> = is.iter().map(key).collect();
    // Lines the change wrote: where it only deletes, the line that now
    // stands there did not change.
    let touched = &stat.inserted_at;
    let removed = was
        .iter()
        .filter(|d| !kept.contains(&key(d)))
        .map(|d| Concerned {
            route: d.clone(),
            router: stat.path.clone(),
            change: RouteChange::Removed,
        });
    let changed = is.iter().filter_map(|d| {
        let old = was.iter().find(|w| key(w) == key(d))?;
        let differs = old.module != d.module || old.action != d.action;
        (differs || touched.contains(&d.line)).then(|| Concerned {
            route: d.clone(),
            router: stat.path.clone(),
            change: RouteChange::Changed,
        })
    });
    removed.chain(changed).collect()
}

/// The routes of `routers` served by an action the change affects: the
/// functions `narrowing` tells for each changed file, or all of a file's
/// when they are not told.
fn action_changes(
    routers: &[(&Path, Vec<Declared>)],
    narrowing: &[Narrowing],
    text_of: impl Fn(&Path) -> Option<String>,
) -> Vec<Concerned> {
    let mut found = Vec::new();
    for n in narrowing {
        let Some(text) = text_of(&n.file) else {
            continue;
        };
        let modules = elixir_modules(&text);
        let affects = |route: &Declared| {
            modules.contains(&route.module)
                && match (&n.functions, &route.action) {
                    (Some(functions), Some(action)) => functions.contains(action),
                    _ => true,
                }
        };
        for (router, routes) in routers {
            found.extend(routes.iter().filter(|r| affects(r)).map(|r| Concerned {
                route: r.clone(),
                router: router.to_path_buf(),
                change: RouteChange::Action,
            }));
        }
    }
    found
}

/// The segments of a path.
fn segments(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

/// Whether a call for `call` may ask for the route at `route`, and whether
/// it asks for all of it. A call matches the end of the route, since its
/// base URL may hold the rest; one of a single segment must match whole.
fn matches(route: &str, call: &str) -> Option<bool> {
    let (route, call) = (segments(route), segments(call));
    if call.len() > route.len() || (call.len() < 2 && call.len() != route.len()) {
        return None;
    }
    let tail = &route[route.len() - call.len()..];
    let param = |s: &str| s.starts_with(':') || s.starts_with('*');
    let pairs = || tail.iter().zip(&call);
    let fit = pairs().all(|(r, c)| param(r) || param(c) || r == c);
    let fixed = pairs().any(|(r, c)| !param(r) && r == c);
    (fit && fixed).then_some(call.len() == route.len())
}

/// Whether a call with `verb` may ask for a route answering `route`.
fn verb_fits(route: &str, verb: Option<&str>) -> bool {
    route == "*" || verb.is_none_or(|v| v == route)
}

/// The name a test file is named after: `client` for `client_test.exs`,
/// `test_client.py` or `client.test.ts`.
fn tested_name(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let stem = stem.split('.').next().unwrap_or(stem);
    let name = stem
        .strip_suffix("_test")
        .or_else(|| stem.strip_suffix("_spec"))
        .or_else(|| stem.strip_prefix("test_"))
        .unwrap_or(stem);
    Some(name.to_string())
}

/// What the HTTP level needs to know of the repository.
pub struct Repository<'a> {
    /// The files `is_read` keeps, after the change.
    pub sources: &'a [(PathBuf, String)],
    /// The project each file belongs to.
    pub project_of: &'a dyn Fn(&Path) -> Option<String>,
}

impl Repository<'_> {
    fn text_of(&self, path: &Path) -> Option<&str> {
        self.sources
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, t)| t.as_str())
    }

    /// The client calls of every file outside tests.
    fn calls(&self) -> Vec<ClientCall> {
        let language = |p: &Path| graphed_language(p).unwrap_or_default();
        self.sources
            .iter()
            .filter(|(path, _)| role(path) == Role::Source)
            .flat_map(|(path, text)| http_clients::calls(path, language(path), text))
            .collect()
    }

    /// The routes the change concerns, each once, the strongest change kept.
    fn concerned(&self, changes: &[FileDiffStat], narrowing: &[Narrowing]) -> Vec<Concerned> {
        let routers: Vec<(&Path, Vec<Declared>)> = self
            .sources
            .iter()
            .filter(|(path, text)| is_router(path, text))
            .map(|(path, text)| (path.as_path(), declared(text)))
            .collect();
        let in_routers = changes.iter().filter(|c| {
            let text = self.text_of(&c.path).unwrap_or_default();
            is_router(&c.path, text)
                || (c.kind == ChangeKind::Deleted && is_router(&c.path, &c.removed.join("\n")))
        });
        let mut all: Vec<Concerned> = in_routers
            .flat_map(|c| router_changes(c, self.text_of(&c.path)))
            .chain(action_changes(&routers, narrowing, |p| {
                self.text_of(p).map(str::to_string)
            }))
            .collect();
        all.sort_by(|a, b| {
            (&a.router, &a.route.path, &a.route.verb, a.change).cmp(&(
                &b.router,
                &b.route.path,
                &b.route.verb,
                b.change,
            ))
        });
        all.dedup_by(|b, a| {
            a.router == b.router && a.route.path == b.route.path && a.route.verb == b.route.verb
        });
        all
    }
}

/// The HTTP level of `changes`: the routes they remove or change, in the
/// routers they touch or behind the actions `narrowing` tells, and the
/// client calls of `repository` that ask for each.
pub fn measure(repository: &Repository, changes: &[FileDiffStat], narrowing: &[Narrowing]) -> Http {
    let concerned = repository.concerned(changes, narrowing);
    if concerned.is_empty() {
        return Http::default();
    }
    let calls = repository.calls();
    let tested: HashSet<String> = repository
        .sources
        .iter()
        .filter(|(path, _)| role(path) == Role::Test)
        .filter_map(|(path, _)| tested_name(path))
        .collect();
    let caller = |call: &ClientCall, route: &Concerned| -> Option<Caller> {
        let path = call.path.as_ref()?;
        if call.file == route.router || !verb_fits(&route.route.verb, call.verb.as_deref()) {
            return None;
        }
        let exact = matches(&route.route.path, path)?;
        Some(Caller {
            file: call.file.clone(),
            line: call.line,
            client: call.client.clone(),
            path: path.clone(),
            project: (repository.project_of)(&call.file),
            exact,
            tested: tested_name(&call.file).is_some_and(|n| tested.contains(&n)),
        })
    };
    let routes = concerned
        .iter()
        .map(|c| {
            let mut callers: Vec<Caller> =
                calls.iter().filter_map(|call| caller(call, c)).collect();
            callers.sort_by(|a, b| (!a.exact, &a.file, a.line).cmp(&(!b.exact, &b.file, b.line)));
            ChangedRoute {
                verb: c.route.verb.clone(),
                path: c.route.path.clone(),
                router: c.router.clone(),
                line: c.route.line,
                change: c.change,
                serves: serves(&c.route),
                callers,
            }
        })
        .collect();
    let unresolved = calls.iter().filter(|c| c.path.is_none()).cloned().collect();
    Http {
        routes,
        unresolved,
        calls: calls.len(),
    }
}

#[cfg(test)]
#[path = "http_test.rs"]
mod tests;
