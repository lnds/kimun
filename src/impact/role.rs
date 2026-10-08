//! What each file of a graph is to the radius.

use std::path::Path;

use crate::deps::graph::FileGraph;
use crate::walk::{TEST_DIRS, is_test_file};

use super::reading::graphed_language;

/// What a file is to the radius.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Code that ships: it can change, and it can be reached.
    Source,
    /// A test: it protects the sources it refers to.
    Test,
    /// Test support, configuration and scripts. A factory or a case
    /// template refers to every schema; counting it as a test would make
    /// everything look protected.
    Other,
}

pub fn role(path: &Path) -> Role {
    let in_test_dir = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .any(|c| TEST_DIRS.contains(&c));
    let is_script = graphed_language(path) == Some("Elixir Script");
    if is_test_file(path) {
        Role::Test
    } else if in_test_dir || is_script {
        Role::Other
    } else {
        Role::Source
    }
}

/// What each file of `graph` is. A file that holds tests is a test when it
/// sits among them or carries their name: test support otherwise, as
/// `tests/common` is.
pub fn roles_of(graph: &FileGraph) -> Vec<Role> {
    // `src/x/tests.rs`: the tests of a module kept in a file of their own.
    let named_tests = |path: &Path| path.file_stem().is_some_and(|stem| stem == "tests");
    let holding = |file: usize| match role(&graph.files[file]) {
        Role::Other if graph.own_tests[file] => Role::Test,
        Role::Source if graph.own_tests[file] && named_tests(&graph.files[file]) => Role::Test,
        role => role,
    };
    (0..graph.files.len()).map(holding).collect()
}

/// Count a source that holds tests as a test of its own, among the tests
/// `referring` lists for each file.
pub fn own_tests(graph: &FileGraph, roles: &[Role], referring: &mut [Vec<usize>]) {
    for file in (0..roles.len()).filter(|&f| roles[f] == Role::Source && graph.own_tests[f]) {
        referring[file].push(file);
    }
}
