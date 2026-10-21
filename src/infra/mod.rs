//! Measures of infrastructure code, the files of kind `infra`.
//!
//! `km infra smells` finds the security smells of Terraform, Dockerfiles,
//! CI pipelines, Compose files, Kubernetes manifests and Ansible
//! playbooks, whatever `--kind` says.

mod help;
pub mod lines;
mod report;
pub mod secrets;
pub mod smells;

pub use help::SMELLS as SMELLS_HELP;
pub use smells::Smell;

use std::cmp::Reverse;
use std::error::Error;
use std::path::PathBuf;

use crate::cli::{InfraCommands, OutputMode};
use crate::kind::Kind;
use crate::walk::WalkConfig;
use lines::Format;
use smells::Found;

/// The smells of one file.
pub struct FileSmells {
    pub path: PathBuf,
    pub language: &'static str,
    pub found: Vec<Found>,
}

/// What `km infra smells` was given besides the walk and the output.
pub struct SmellsOptions<'a> {
    pub top: Option<usize>,
    /// Keep only the files changed since this ref.
    pub since_ref: Option<&'a str>,
    /// Fail when one of these smells is found.
    pub fail_on: &'a [Smell],
}

/// The security smells of the infrastructure files of the walk.
pub fn run(
    cfg: &WalkConfig<'_>,
    output: OutputMode,
    options: &SmellsOptions<'_>,
) -> Result<(), Box<dyn Error>> {
    let changed = match options.since_ref {
        Some(since) => Some((
            crate::docs::repository_root(cfg.path)?,
            crate::docs::changed_since(cfg.path, since)?,
        )),
        None => None,
    };
    let mut files = Vec::new();
    let mut read = 0;
    for (path, spec) in cfg.source_files() {
        if let Some((root, changed)) = &changed {
            let full = path.canonicalize()?;
            if !changed.contains(full.strip_prefix(root).unwrap_or(&full)) {
                continue;
            }
        }
        let text = match std::fs::read(&path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(err) => {
                eprintln!("warning: {}: {err}", path.display());
                continue;
            }
        };
        read += 1;
        let found = smells::scan(&text, Format::of(spec.name));
        if !found.is_empty() {
            files.push(FileSmells {
                path,
                language: spec.name,
                found,
            });
        }
    }
    files.sort_by(|a, b| {
        Reverse(a.found.len())
            .cmp(&Reverse(b.found.len()))
            .then_with(|| a.path.cmp(&b.path))
    });
    match output {
        OutputMode::Json => report::print_json(&files, options.top)?,
        OutputMode::Github => report::print_github(&files),
        OutputMode::Codeclimate => report::print_codeclimate(&files)?,
        OutputMode::Short => report::print_short(&files, read),
        OutputMode::Terse => println!("{}", report::total(&files)),
        OutputMode::Table => report::print_report(&files, read, options.top),
    }
    let failing: Vec<&str> = options
        .fail_on
        .iter()
        .filter(|smell| {
            files
                .iter()
                .any(|f| f.found.iter().any(|s| s.smell == **smell))
        })
        .map(|smell| smell.title())
        .collect();
    match failing.is_empty() {
        true => Ok(()),
        false => Err(format!("infrastructure smells found: {}", failing.join(", ")).into()),
    }
}

/// Run a `km infra` subcommand.
pub fn dispatch(command: InfraCommands) {
    match command {
        InfraCommands::Smells {
            common,
            top,
            since_ref,
            fail_on,
        } => {
            let options = SmellsOptions {
                top,
                since_ref: since_ref.as_deref(),
                fail_on: &fail_on,
            };
            crate::on_kind::run(Kind::Infra, common, |cfg, output| {
                run(cfg, output, &options)
            })
        }
    }
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
