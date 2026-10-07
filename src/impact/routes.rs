//! Tests that reach a file through the route it serves.
//!
//! A test of a controller or of a live view asks for a path. The router of
//! its project tells which module serves it, and that makes the test one
//! that refers to the module, though it never names it.

use crate::deps::graph::{FileGraph, Related, Source, shared_depth};
use crate::deps::routes::{requests, routes, serves};

/// A router: its file, and the paths it serves with the file behind each.
struct Router {
    file: usize,
    serves: Vec<(String, usize)>,
}

/// The routers among `sources`, with their routes resolved to files.
fn routers(sources: &[Source], graph: &FileGraph, related: Related) -> Vec<Router> {
    let mentions_router = |text: &str| text.contains(":router") || text.contains("Phoenix.Router");
    sources
        .iter()
        .enumerate()
        .filter(|(_, source)| source.language == "Elixir" && mentions_router(&source.text))
        .map(|(file, source)| Router {
            file,
            serves: routes(&source.text)
                .into_iter()
                .filter_map(|route| {
                    let to = graph.defining(&route.module, file, related)?;
                    Some((route.path, to))
                })
                .collect(),
        })
        .filter(|router| !router.serves.is_empty())
        .collect()
}

/// For each file of `graph`, the tests that request a route it serves. A
/// test asks the routers nearest to it: several projects of a repository
/// serve the same paths.
pub fn requesting(
    sources: &[Source],
    graph: &FileGraph,
    is_test: impl Fn(usize) -> bool,
    related: Related,
) -> Vec<Vec<usize>> {
    let mut requesting = vec![Vec::new(); graph.files.len()];
    let routers = routers(sources, graph, related);
    if routers.is_empty() {
        return requesting;
    }
    for (test, source) in sources.iter().enumerate().filter(|(f, _)| is_test(*f)) {
        let asked = requests(&source.text);
        if asked.is_empty() {
            continue;
        }
        let depth = |router: &Router| shared_depth(&graph.files[router.file], &source.path);
        let nearest = routers.iter().map(depth).max().unwrap_or(0);
        let served = routers
            .iter()
            .filter(|router| depth(router) == nearest)
            .flat_map(|router| &router.serves);
        for (path, to) in served {
            if asked.iter().any(|request| serves(path, request)) {
                requesting[*to].push(test);
            }
        }
    }
    requesting
}
