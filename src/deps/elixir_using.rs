//! What an Elixir file takes from other modules of the project without
//! naming it in its own code: what `use` injects, and what an `import`
//! answers.
//!
//! `use Foo, :name` runs `Foo.name/0` by convention (as `__using__` does
//! in the Phoenix web module), and `use Foo` runs `Foo.__using__`. The
//! code of that function, and of the functions it splices in with
//! `unquote(helper())`, is read as if the user had written it: its
//! aliases, imports and references become the user's. The quote is read
//! whole, whichever branch the macro takes.

use std::collections::{BTreeSet, HashMap};

use super::elixir::{self, Aliases, ElixirFile, Reference, references};
use super::elixir_compile::CompileTime;
use super::elixir_directives::{Using, directives, ident_of};
use super::elixir_functions::{Function, functions};
use super::elixir_literals::code_only;
use super::graph::Source;

/// What the modules a file `use`s inject into it.
#[derive(Debug, Default, Clone)]
pub struct Injection {
    /// Aliases declared in the injected code.
    pub aliases: Aliases,
    pub refs: Vec<Reference>,
    /// Modules imported whole by the injected code.
    pub whole: Vec<String>,
    /// Modules the injected code `use`s in turn.
    pub uses: Vec<Using>,
    /// Modules the injected code `use`s, which the user then needs to
    /// compile.
    pub compile: Vec<String>,
    /// What the `use`s written outside `quote` blocks inject, which is
    /// compiled with the user.
    pub compiled: CompileTime,
}

impl Injection {
    fn merge(&mut self, other: &Injection) {
        self.aliases.inherit(&other.aliases);
        self.refs.extend(other.refs.iter().cloned());
        self.whole.extend(other.whole.iter().cloned());
        self.compile.extend(other.compile.iter().cloned());
    }

    fn is_empty(&self) -> bool {
        self.refs.is_empty() && self.whole.is_empty()
    }
}

/// The function a `use` runs.
fn entry(option: Option<&str>) -> &str {
    option.unwrap_or("__using__")
}

/// The calls `unquote(name(` in `body` makes.
fn spliced(body: &str) -> impl Iterator<Item = &str> {
    body.match_indices("unquote(")
        .map(|(at, found)| ident_of(&body[at + found.len()..]))
        .filter(|name| !name.is_empty())
}

/// What `use` of the module defined in `source` injects, `None` when the
/// module defines no function the `use` runs.
pub fn injection(source: &str, option: Option<&str>) -> Option<Injection> {
    let defined = functions(source);
    let code = code_only(source);
    let lines: Vec<&str> = code.lines().collect();
    let body = |f: &Function| lines[f.first_line - 1..f.last_line].join("\n");

    let mut selected: BTreeSet<String> = BTreeSet::new();
    let mut pending = vec![entry(option).to_string()];
    while let Some(name) = pending.pop() {
        let clauses: Vec<String> = defined
            .iter()
            .filter(|f| f.name == name)
            .map(body)
            .collect();
        if selected.insert(name) {
            pending.extend(clauses.iter().flat_map(|b| spliced(b).map(str::to_string)));
        }
    }
    if !defined.iter().any(|f| f.name == entry(option)) {
        return None;
    }
    let injected: String = defined
        .iter()
        .filter(|f| selected.contains(&f.name))
        .map(|f| body(f) + "\n")
        .collect();

    let (mut aliases, _) = Aliases::of(&code);
    let (inner, used) = Aliases::of(&injected);
    aliases.inherit(&inner);
    let found = directives(&used, &aliases);
    let mut refs = references(&used, &aliases);
    refs.extend(found.calls);
    Some(Injection {
        aliases: inner,
        refs,
        whole: found.whole,
        compile: found.uses.iter().map(|u| u.module.clone()).collect(),
        uses: found.uses,
        ..Injection::default()
    })
}

/// How deep `use` is followed through the injected code.
const DEPTH: usize = 3;

/// Where references resolve: the file that defines a module, for a
/// reference made from a file.
pub type Resolve<'a> = &'a dyn Fn(usize, &str) -> Option<usize>;

/// Read again the files that `use` modules of the project, with what those
/// inject, and add the calls each `import` of a module of the project
/// answers.
pub fn complete(parsed: &mut [Option<ElixirFile>], sources: &[Source], resolve: Resolve) {
    let mut injector = Injector {
        sources,
        resolve,
        cache: HashMap::new(),
    };
    for file in 0..parsed.len() {
        let Some(parsed_file) = parsed[file].as_ref() else {
            continue;
        };
        let uses = parsed_file.uses.clone();
        let written = parsed_file.compile.written(&uses);
        let mut injected = injector.inject(file, uses);
        injected.compiled = CompileTime::injected(injector.inject(file, written));
        if !injected.is_empty() {
            let source = &sources[file];
            parsed[file] = Some(elixir::read(&source.text, &source.templates, &injected));
        }
    }
    import_calls(parsed, sources, resolve);
}

/// Reads what `use` injects, each module and function once.
struct Injector<'a> {
    sources: &'a [Source],
    resolve: Resolve<'a>,
    cache: HashMap<(usize, Option<String>), Option<Injection>>,
}

impl Injector<'_> {
    /// What the `uses` of `file` inject into it, through the `use`s of the
    /// injected code too, with a call to the function each `use` runs. A
    /// module of the project that defines no such function is taken whole.
    fn inject(&mut self, file: usize, uses: Vec<Using>) -> Injection {
        let mut injected = Injection::default();
        let mut pending: Vec<(Using, usize)> = uses.into_iter().map(|u| (u, 0)).collect();
        let mut seen = BTreeSet::new();
        while let Some((using, depth)) = pending.pop() {
            let Some(to) = (self.resolve)(file, &using.module) else {
                continue;
            };
            let key = (to, using.option.clone());
            if depth > DEPTH || !seen.insert(key.clone()) {
                continue;
            }
            let sources = self.sources;
            let read = self
                .cache
                .entry(key)
                .or_insert_with(|| injection(&sources[to].text, using.option.as_deref()));
            let Some(found) = read else {
                injected.whole.push(using.module);
                continue;
            };
            injected.merge(found);
            pending.extend(found.uses.iter().map(|u| (u.clone(), depth + 1)));
            let function = entry(using.option.as_deref()).to_string();
            injected.refs.push(Reference {
                module: using.module,
                function: Some(function),
            });
        }
        injected
    }
}

/// Add to each file the bare calls it makes to the functions of the
/// modules of the project it imports whole.
fn import_calls(parsed: &mut [Option<ElixirFile>], sources: &[Source], resolve: Resolve) {
    let mut public: HashMap<usize, BTreeSet<String>> = HashMap::new();
    for (file, parsed) in parsed.iter_mut().enumerate() {
        let Some(parsed) = parsed.as_mut() else {
            continue;
        };
        let mut calls = Vec::new();
        for module in &parsed.whole {
            let Some(to) = resolve(file, module) else {
                continue;
            };
            let offered = public.entry(to).or_insert_with(|| {
                let defined = functions(&sources[to].text).into_iter();
                defined.filter(|f| f.public).map(|f| f.name).collect()
            });
            calls.extend(
                parsed
                    .bare
                    .iter()
                    .filter(|name| offered.contains(*name))
                    .map(|name| Reference {
                        module: module.clone(),
                        function: Some(name.clone()),
                    }),
            );
        }
        parsed.refs.extend(calls);
        parsed.refs.sort();
        parsed.refs.dedup();
    }
}

#[cfg(test)]
#[path = "elixir_using_test.rs"]
mod tests;
