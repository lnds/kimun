//! The sources of a change, read into a graph.

use std::path::{Path, PathBuf};

use crate::deps::graph::{FileGraph, Related, Source};
use crate::deps::heex;
use crate::deps::layout::{Layout, is_manifest};
use crate::loc::language::detect;

/// The language of a file, when it has a graph.
pub fn graphed_language(path: &Path) -> Option<&'static str> {
    detect(path)
        .map(|spec| spec.name)
        .filter(|name| crate::deps::is_supported(name))
}

/// Build the graph of the files given as path and text. Templates go to
/// the file that renders them, and manifests say how packages are named;
/// neither is a file of the graph.
pub fn read(files: Vec<(PathBuf, String)>, related: Related) -> (Vec<Source>, FileGraph) {
    let (manifests, files): (Vec<_>, Vec<_>) =
        files.into_iter().partition(|(path, _)| is_manifest(path));
    let (templates, files): (Vec<_>, Vec<_>) = files
        .into_iter()
        .partition(|(path, _)| heex::is_template(path));
    let mut rendered = heex::by_owner(templates, &files);
    let sources: Vec<Source> = files
        .into_iter()
        .filter_map(|(path, text)| {
            let language = graphed_language(&path)?.to_string();
            let templates = rendered.remove(&path).unwrap_or_default();
            Some(Source {
                path,
                language,
                text,
                templates,
            })
        })
        .collect();
    let layout = Layout {
        known: sources.iter().map(|s| s.path.clone()).collect(),
        ..Layout::default()
    }
    .with_manifests(manifests.iter().map(|(p, t)| (p.as_path(), t.as_str())));
    let graph = FileGraph::build(&sources, &layout, related);
    (sources, graph)
}
