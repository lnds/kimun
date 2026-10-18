//! The languages read over the whole graph: what a file of theirs uses
//! depends on more than where it is.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use super::c_family;
use super::csharp;
use super::golang;
use super::graph::{Source, Use};
use super::java;
use super::layout::Layout;
use super::python_uses;
use super::ruby_files;

/// The readers of those languages, each over the sources of a graph.
pub struct Apart<'s> {
    layout: &'s Layout,
    by_path: &'s HashMap<&'s Path, usize>,
    python: python_uses::Modules,
    go: golang::Packages,
    c: c_family::Files<'s>,
    java: java::Classes,
    csharp: csharp::Namespaces,
    ruby: ruby_files::Files,
}

impl<'s> Apart<'s> {
    /// The readers of `sources`, given the layout of the graph and the
    /// index of each source by path.
    pub fn of(
        sources: &'s [Source],
        layout: &'s Layout,
        by_path: &'s HashMap<&'s Path, usize>,
    ) -> Self {
        Apart {
            layout,
            by_path,
            python: python_uses::Modules::of(sources),
            go: golang::Packages::of(sources, layout),
            c: c_family::Files::of(sources),
            java: java::Classes::of(sources),
            csharp: csharp::Namespaces::of(sources),
            ruby: ruby_files::Files::of(sources),
        }
    }

    /// The files `source` uses, when its language is read here. Python
    /// tells the names taken too, a Go import leads to every file of a
    /// package, a Java or C# file uses the types it names, and a Ruby file the
    /// files it requires and the constants it names.
    pub fn uses(&self, source: &Source) -> Option<BTreeMap<usize, BTreeSet<String>>> {
        self.python
            .uses(source, self.layout, self.by_path)
            .or_else(|| self.go.uses(source))
            .or_else(|| self.c.uses(source))
            .or_else(|| {
                let at = *self.by_path.get(source.path.as_path())?;
                self.java
                    .uses(at)
                    .or_else(|| self.csharp.uses(at))
                    .or_else(|| self.ruby.uses(at))
            })
    }

    /// References left out because several files define what they name
    /// and none is nearer, among the files whose uses were asked.
    pub fn ambiguous(&self) -> usize {
        self.ruby.ambiguous()
    }

    /// For each source, the files it uses without naming them, given the
    /// uses it names: the sources behind the headers it includes.
    pub fn implied(&self, sources: &[Source], uses: &[Vec<Use>]) -> Vec<Vec<Use>> {
        sources
            .iter()
            .zip(uses)
            .enumerate()
            .map(|(file, (source, named))| {
                let written: BTreeSet<usize> = named.iter().map(|u| u.to).collect();
                let implied = self.c.implied(source, named).into_iter();
                implied
                    .filter(|to| *to != file && !written.contains(to))
                    .collect::<BTreeSet<usize>>()
                    .into_iter()
                    .map(Use::plain)
                    .collect()
            })
            .collect()
    }
}
