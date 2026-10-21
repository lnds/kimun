//! Terraform: a project is a directory of `.tf` files, a module; a
//! `module` block whose `source` is a local path depends on the module of
//! that directory. Registry, git and other remote sources are external.

use super::{DepTarget, Manifest, RawDep, Scope};
use crate::infra::lines::{self, Format};

/// What a `.tf` file declares: the local modules it calls. The name of the
/// project is that of its directory.
pub fn read(source: &str) -> Manifest {
    let deps = sources(source)
        .into_iter()
        .filter(|s| is_local(s))
        .map(|s| RawDep {
            target: DepTarget::Path(s.into()),
            scope: Scope::Runtime,
        })
        .collect();
    Manifest {
        deps,
        ..Manifest::default()
    }
}

/// Whether a module source is a path of the repository: Terraform reads
/// only `./` and `../` as local.
pub fn is_local(source: &str) -> bool {
    source.starts_with("./") || source.starts_with("../")
}

/// The `source` of each `module` block of a file, as written.
pub fn sources(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut depth = 0usize;
    // The depth a `module` block was opened at, while inside it.
    let mut module: Option<usize> = None;
    for line in lines::split(text, Format::Hcl) {
        let code = line.code.trim();
        if module.is_none() && opens_module(code) {
            module = Some(depth);
        }
        let inside = module.is_some_and(|at| depth == at + 1 || opens_module(code));
        if inside && let Some(source) = source_of(code) {
            found.push(source.to_string());
        }
        depth = braces(code, depth);
        if module.is_some_and(|at| depth <= at) {
            module = None;
        }
    }
    found
}

/// `module "name" {`, the header of a module call.
fn opens_module(code: &str) -> bool {
    code.strip_prefix("module")
        .is_some_and(|rest| rest.starts_with(char::is_whitespace) && rest.contains('{'))
}

/// The quoted value of a `source =` on a line.
fn source_of(code: &str) -> Option<&str> {
    let at = code.match_indices("source").find_map(|(at, _)| {
        let before = code[..at].chars().next_back();
        let starts = !before.is_some_and(|c| c.is_alphanumeric() || c == '_');
        let rest = code[at + "source".len()..].trim_start();
        (starts && rest.starts_with('=')).then_some(rest)
    })?;
    let value = at[1..].trim_start().strip_prefix('"')?;
    value.split_once('"').map(|(inner, _)| inner)
}

/// The depth after a line: braces in strings come in pairs, so counting
/// them all keeps the balance.
fn braces(code: &str, depth: usize) -> usize {
    code.chars().fold(depth, |d, c| match c {
        '{' => d + 1,
        '}' => d.saturating_sub(1),
        _ => d,
    })
}

#[cfg(test)]
#[path = "terraform_test.rs"]
mod tests;
