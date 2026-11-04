//! The HTTP level as text and as JSON.

use serde::Serialize;

use super::http::{Caller, ChangedRoute, Http, RouteChange};
use super::http_clients::ClientCall;

const TITLE: &str = "HTTP level — client calls to the routes the change removes or changes";

/// How many unresolved calls the text names.
const NAMED: usize = 5;

fn change_word(change: RouteChange) -> &'static str {
    match change {
        RouteChange::Removed => "removed",
        RouteChange::Changed => "changed",
        RouteChange::Action => "action",
    }
}

/// The line under a route for one of its callers.
fn caller_line(caller: &Caller) -> String {
    let mut about = vec![caller.client.clone()];
    about.extend(caller.project.clone());
    if !caller.exact {
        about.push(format!("asks for {}", caller.path));
    }
    about.push(if caller.tested { "tested" } else { "no test" }.to_string());
    format!(
        "          {}:{}  {}",
        caller.file.display(),
        caller.line,
        about.join(", ")
    )
}

fn route_lines(route: &ChangedRoute) -> Vec<String> {
    let mut lines = vec![format!(
        " {:<8} {} {}  {} ({}:{})",
        change_word(route.change),
        route.verb.to_uppercase(),
        route.path,
        route.serves,
        route.router.display(),
        route.line
    )];
    lines.extend(route.callers.iter().map(caller_line));
    if route.callers.is_empty() {
        lines.push("          no client call asks for it".to_string());
    }
    lines
}

/// The block of the report; empty when no route is concerned.
pub fn render(http: &Http) -> Vec<String> {
    if http.routes.is_empty() {
        return Vec::new();
    }
    let sep = crate::report_helpers::separator(78);
    let mut lines = vec![TITLE.to_string(), sep.clone()];
    lines.extend(http.routes.iter().flat_map(route_lines));
    lines.push(sep);
    lines.push(format!(
        "Matched by method and the end of the path, over {} client calls: a heuristic.",
        http.calls
    ));
    if !http.unresolved.is_empty() {
        let named: Vec<String> = http
            .unresolved
            .iter()
            .take(NAMED)
            .map(|c| format!("{}:{}", c.file.display(), c.line))
            .collect();
        let more = http.unresolved.len().saturating_sub(NAMED);
        let tail = if more > 0 {
            format!(" and {more} more")
        } else {
            String::new()
        };
        lines.push(format!(
            "Unknown: {} calls build their path at run time: {}{tail}",
            http.unresolved.len(),
            named.join(", ")
        ));
    }
    lines
}

/// The HTTP level in the JSON report.
#[derive(Serialize)]
pub struct JsonHttp<'a> {
    heuristic: bool,
    calls: usize,
    routes: &'a [ChangedRoute],
    /// Calls whose path is built at run time: they may ask for any route.
    unresolved: &'a [ClientCall],
}

impl<'a> From<&'a Http> for JsonHttp<'a> {
    fn from(http: &'a Http) -> Self {
        JsonHttp {
            heuristic: true,
            calls: http.calls,
            routes: &http.routes,
            unresolved: &http.unresolved,
        }
    }
}
