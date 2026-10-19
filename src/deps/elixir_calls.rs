//! Which functions of other modules each public function of an Elixir file
//! calls, directly or through the functions of its file it reaches: what
//! lets `km impact` follow a change from function to function.

use std::collections::{BTreeMap, BTreeSet};

use super::elixir::{Aliases, Reference, modules_of, references};
use super::elixir_directives::directives;
use super::elixir_functions::{calls_local, functions};
use super::elixir_literals::code_only;

/// For each public function, by name, the calls it makes as
/// `Module.function`, with the full name of the module.
pub type Calls = BTreeMap<String, BTreeSet<String>>;

fn call(reference: &Reference) -> Option<String> {
    Some(format!(
        "{}.{}",
        reference.module,
        reference.function.as_ref()?
    ))
}

/// The calls of the public functions of `source`. A function with several
/// clauses is one; what a `use` injects and what templates call belong to
/// no function and are left out.
pub fn calls(source: &str) -> Calls {
    let (_, code) = modules_of(&code_only(source));
    let (aliases, used) = Aliases::of(&code);
    let lines: Vec<&str> = used.lines().collect();
    let imported = directives(&used, &aliases).calls;
    let functions = functions(source);
    let mut bodies: BTreeMap<&str, String> = BTreeMap::new();
    for f in &functions {
        let span = lines.get(f.first_line - 1..f.last_line.min(lines.len()));
        let body = bodies.entry(f.name.as_str()).or_default();
        body.push_str(&span.unwrap_or_default().join("\n"));
        body.push('\n');
    }
    let direct: BTreeMap<&str, BTreeSet<String>> = bodies
        .iter()
        .map(|(&name, body)| {
            let named = references(body, &aliases);
            let by_import = imported
                .iter()
                .filter(|r| r.function.as_deref().is_some_and(|f| calls_local(body, f)));
            (
                name,
                named.iter().chain(by_import).filter_map(call).collect(),
            )
        })
        .collect();
    let public: BTreeSet<&str> = functions
        .iter()
        .filter(|f| f.public)
        .map(|f| f.name.as_str())
        .collect();
    public
        .into_iter()
        .map(|name| {
            let reached = reached_locally(name, &bodies);
            let calls = reached.into_iter().flat_map(|n| direct[n].iter().cloned());
            (name.to_string(), calls.collect())
        })
        .collect()
}

/// `name` and the functions of the file it reaches through local calls.
fn reached_locally<'a>(name: &'a str, bodies: &BTreeMap<&'a str, String>) -> BTreeSet<&'a str> {
    let mut reached = BTreeSet::from([name]);
    let mut pending = vec![name];
    while let Some(caller) = pending.pop() {
        for &other in bodies.keys() {
            if !reached.contains(other) && calls_local(&bodies[caller], other) {
                reached.insert(other);
                pending.push(other);
            }
        }
    }
    reached
}

#[cfg(test)]
#[path = "elixir_calls_test.rs"]
mod tests;
