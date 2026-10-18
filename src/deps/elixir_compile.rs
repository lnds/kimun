//! What an Elixir file needs at compile time: the modules it `use`s and the
//! macros it calls, which the compiler expands into the caller.
//!
//! Code inside a `quote` block is not compiled with the file that writes
//! it: it is injected into the module that calls the macro. An `import` or
//! `require` alone recompiles the file only when the exports of the module
//! change, so neither is a compile-time use by itself. A function called in
//! the body of a module, outside any function, also runs at compile time;
//! that is not told here.

use super::elixir::{Aliases, Reference, references};
use super::elixir_directives::{Using, bare_calls, directives, ident_of};
use super::elixir_literals::code_only;
use super::elixir_using::Injection;

/// What a file may need at compile time, before modules are resolved.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CompileTime {
    /// Modules `use`d, by the file or by what a `use` injects into it.
    pub uses: Vec<String>,
    /// Qualified calls and calls imported by name: those to a macro are
    /// expanded.
    pub calls: Vec<Reference>,
    /// Calls without a module, which a whole `import` may answer.
    pub bare: Vec<String>,
}

impl CompileTime {
    /// What `used` (code with literals blanked and `alias` statements
    /// removed) needs, outside its `quote` blocks.
    pub fn of(used: &str, source: &str, aliases: &Aliases) -> Self {
        let outside = outside_quotes(used);
        let found = directives(&outside, aliases);
        let mut calls = references(&outside, aliases);
        calls.retain(|r| r.function.is_some());
        calls.extend(found.calls);
        calls.sort();
        calls.dedup();
        let mut uses: Vec<String> = found.uses.into_iter().map(|u| u.module).collect();
        uses.sort();
        uses.dedup();
        CompileTime {
            uses,
            calls,
            bare: bare_calls(&outside, source),
        }
    }

    /// What a `use` injects into its user, all of it compiled there.
    pub fn injected(injection: Injection) -> Self {
        let mut calls = injection.refs;
        calls.retain(|r| r.function.is_some());
        CompileTime {
            uses: injection.compile,
            calls,
            bare: Vec::new(),
        }
    }

    pub fn add(&mut self, other: &CompileTime) {
        self.uses.extend(other.uses.iter().cloned());
        self.uses.sort();
        self.uses.dedup();
        self.calls.extend(other.calls.iter().cloned());
        self.calls.sort();
        self.calls.dedup();
    }

    /// Of `uses`, those written outside `quote` blocks.
    pub fn written(&self, uses: &[Using]) -> Vec<Using> {
        let outside = uses.iter().filter(|u| self.uses.contains(&u.module));
        outside.cloned().collect()
    }

    /// The files needed at compile time: those `use`d, and those whose
    /// macros are called, by name or through a whole `import`, given where
    /// modules resolve and the macros of each file.
    pub fn needed<'a>(
        &self,
        whole: &[String],
        resolve: &dyn Fn(&String) -> Option<usize>,
        macros: &dyn Fn(usize) -> &'a [String],
    ) -> Vec<usize> {
        let expands = |to: usize, name: &String| macros(to).contains(name);
        let mut needed: Vec<usize> = self.uses.iter().filter_map(resolve).collect();
        needed.extend(self.calls.iter().filter_map(|call| {
            let to = resolve(&call.module)?;
            expands(to, call.function.as_ref()?).then_some(to)
        }));
        let imported = whole.iter().filter_map(resolve);
        needed.extend(imported.filter(|&to| self.bare.iter().any(|b| expands(to, b))));
        needed
    }
}

/// Whether a trimmed line opens a `quote` block: `quote do`, `quote` with
/// options before its `do`, or either bound to a name.
fn opens_quote(trimmed: &str) -> bool {
    let at = trimmed.find("quote").filter(|&at| {
        let before = trimmed[..at].chars().next_back();
        before.is_none_or(|c| c == ' ' || c == '(')
    });
    at.is_some_and(|at| {
        let rest = &trimmed[at + "quote".len()..];
        (rest == " do" || rest.starts_with(' ') && rest.ends_with(" do")) && !rest.contains("do:")
    })
}

/// `code` with the lines inside `quote` blocks blanked: those indented
/// deeper than the line that opens one, up to its `end`.
fn outside_quotes(code: &str) -> String {
    let mut quote: Option<usize> = None;
    code.split_inclusive('\n')
        .map(|line| {
            let indent = line.len() - line.trim_start().len();
            let inside = quote.is_some_and(|q| indent > q || line.trim().is_empty());
            if inside {
                return "\n";
            }
            quote = opens_quote(line.trim()).then_some(indent);
            line
        })
        .collect()
}

/// The public macros of a file, guards included: a call to one is
/// expanded when the caller compiles.
pub fn macros(source: &str) -> Vec<String> {
    let code = code_only(source);
    let mut found: Vec<String> = code
        .lines()
        .filter_map(|line| {
            let content = line.trim_start();
            let rest = ["defmacro ", "defguard "]
                .iter()
                .find_map(|k| content.strip_prefix(k))?;
            let name = ident_of(rest.trim_start());
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect();
    found.sort();
    found.dedup();
    found
}

#[cfg(test)]
#[path = "elixir_compile_test.rs"]
mod tests;
