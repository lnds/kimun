//! What a file is for: code, documentation, infrastructure or data.
//!
//! The language tells most of it; YAML is infrastructure or data by where
//! it sits and what it holds. `.kimun.toml` may declare kinds by glob, and
//! `--kind` chooses which ones a command reads. Without a choice every
//! command reads what it always has: infrastructure written in a language
//! of code (Terraform, Dockerfiles) is measured as code.
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Serialize;

use crate::loc::language::{LanguageSpec, detect};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Code,
    Docs,
    Infra,
    Data,
}

pub const ALL: [Kind; 4] = [Kind::Code, Kind::Docs, Kind::Infra, Kind::Data];

const DOCS: &[&str] = &["Markdown", "reStructuredText", "Text"];
const INFRA: &[&str] = &["Terraform", "Dockerfile"];

/// YAML files that are infrastructure by name: CI pipelines, Compose,
/// Helm charts, Kustomize.
const INFRA_YAML_NAMES: &[&str] = &[
    ".gitlab-ci.yml",
    ".travis.yml",
    "azure-pipelines.yml",
    "bitbucket-pipelines.yml",
    "cloudbuild.yaml",
    "buildspec.yml",
    "Chart.yaml",
    "kustomization.yaml",
    "kustomization.yml",
];

/// Directories whose YAML is infrastructure: GitHub workflows, CircleCI,
/// and the layout of Ansible.
const INFRA_YAML_DIRS: &[&str] = &[
    ".circleci",
    "playbooks",
    "group_vars",
    "host_vars",
    "tasks",
    "handlers",
];

/// How many lines of a YAML file are read to tell what it holds.
const SNIFF_LINES: usize = 64;

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Code => "code",
            Kind::Docs => "docs",
            Kind::Infra => "infra",
            Kind::Data => "data",
        }
    }

    /// The kind of a file on disk: by its language and path, and for YAML
    /// not told by path, by what its first lines hold.
    pub fn of(path: &Path, spec: &LanguageSpec) -> Kind {
        match Self::by_name(path, spec) {
            Kind::Data if spec.name == "YAML" && holds_infra(path) => Kind::Infra,
            kind => kind,
        }
    }

    /// The kind of a path by its language and name alone, for paths that
    /// may no longer exist.
    pub fn by_name(path: &Path, spec: &LanguageSpec) -> Kind {
        if DOCS.contains(&spec.name) {
            Kind::Docs
        } else if INFRA.contains(&spec.name) || (spec.name == "YAML" && is_infra_yaml_path(path)) {
            Kind::Infra
        } else if spec.is_code() {
            Kind::Code
        } else {
            Kind::Data
        }
    }
}

fn is_infra_yaml_path(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let compose = (name.starts_with("docker-compose") || name.starts_with("compose."))
        && (name.ends_with(".yml") || name.ends_with(".yaml"));
    let dirs: Vec<&str> = path
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    let workflow = dirs.windows(2).any(|w| w == [".github", "workflows"]);
    INFRA_YAML_NAMES.contains(&name)
        || compose
        || workflow
        || dirs.iter().any(|d| INFRA_YAML_DIRS.contains(d))
}

/// Whether the first lines of a YAML file are a Kubernetes manifest, a
/// CloudFormation template or an Ansible playbook.
fn holds_infra(path: &Path) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let lines: Vec<String> = BufReader::new(file)
        .lines()
        .take(SNIFF_LINES)
        .map_while(Result::ok)
        .collect();
    yaml_holds_infra(&lines)
}

fn yaml_holds_infra(lines: &[String]) -> bool {
    let top = |key: &str| lines.iter().any(|l| l.starts_with(key));
    let manifest = top("apiVersion:") && top("kind:");
    let first = lines
        .iter()
        .map(|l| l.trim_end())
        .find(|l| !l.is_empty() && !l.starts_with('#') && *l != "---");
    let plays = first.is_some_and(|l| l.starts_with("- "));
    let playbook = plays
        && lines
            .iter()
            .any(|l| l.starts_with("- hosts:") || l.starts_with("  hosts:"));
    manifest || top("AWSTemplateFormatVersion:") || playbook
}

/// Kinds declared in `.kimun.toml`, by glob against the path relative to
/// the analysed directory. The first that matches wins, in the order
/// code, docs, infra, data.
#[derive(Clone, Debug, Default)]
struct Declared {
    sets: Vec<(Kind, GlobSet)>,
}

impl Declared {
    fn new(config: &crate::config::KindsConfig) -> Self {
        let declared = [
            (Kind::Code, &config.code),
            (Kind::Docs, &config.docs),
            (Kind::Infra, &config.infra),
            (Kind::Data, &config.data),
        ];
        let sets = declared
            .into_iter()
            .filter(|(_, globs)| !globs.is_empty())
            .filter_map(|(kind, globs)| glob_set(globs).map(|set| (kind, set)))
            .collect();
        Self { sets }
    }

    fn kind(&self, path: &Path, root: &Path) -> Option<Kind> {
        let relative = path.strip_prefix(root).unwrap_or(path);
        self.sets
            .iter()
            .find(|(_, set)| set.is_match(relative))
            .map(|(kind, _)| *kind)
    }
}

fn glob_set(patterns: &[String]) -> Option<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        match Glob::new(pattern) {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(e) => eprintln!("warning: invalid glob '{pattern}' in [kinds]: {e}"),
        }
    }
    builder.build().ok()
}

/// Which kinds a command reads: those of `--kind`, or else what it always
/// read, with infrastructure apart from code when `[kinds] separate` is set.
#[derive(Clone, Debug, Default)]
pub struct Selection {
    chosen: Option<BTreeSet<Kind>>,
    separate: bool,
    declared: Declared,
}

impl Selection {
    pub fn new(chosen: &[Kind], config: &crate::config::KindsConfig) -> Self {
        Self {
            chosen: (!chosen.is_empty()).then(|| chosen.iter().copied().collect()),
            separate: config.separate,
            declared: Declared::new(config),
        }
    }

    /// The kind of a file on disk under `root`.
    pub fn kind(&self, path: &Path, spec: &LanguageSpec, root: &Path) -> Kind {
        self.declared
            .kind(path, root)
            .unwrap_or_else(|| Kind::of(path, spec))
    }

    /// Whether the metrics of code read a file of this kind and language.
    pub fn measures(&self, kind: Kind, spec: &LanguageSpec) -> bool {
        match (&self.chosen, kind) {
            (Some(chosen), _) => chosen.contains(&kind),
            (None, Kind::Code) => true,
            (None, Kind::Infra) => !self.separate && spec.is_code(),
            (None, _) => false,
        }
    }

    /// Whether the metrics of code read a path, which may no longer exist,
    /// by its name alone. A path of no known language counts as code.
    pub fn measures_path(&self, path: &Path, root: &Path) -> bool {
        let Some(spec) = detect(path) else {
            return self.chosen.as_ref().is_none_or(|c| c.contains(&Kind::Code));
        };
        let kind = self
            .declared
            .kind(path, root)
            .unwrap_or_else(|| Kind::by_name(path, spec));
        self.measures(kind, spec)
    }

    /// Whether the commands that read every source file (line counts, the
    /// dependency graph) read a file of this kind.
    pub fn reads(&self, kind: Kind) -> bool {
        self.chosen.as_ref().is_none_or(|c| c.contains(&kind))
    }
}

#[cfg(test)]
#[path = "kind_test.rs"]
mod tests;
