//! Files that steer coding agents: `CLAUDE.md`, `AGENTS.md`, rules of
//! Cursor and Copilot, skills, agents and commands. They are loaded into
//! the model's context, so their size, the rules they pile up and what
//! they repeat have a cost.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

/// What a context file is to an agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Read for every task under its directory: `CLAUDE.md`, `AGENTS.md`.
    Instructions,
    /// Rules of an editor: `.cursorrules`, `.cursor/rules/`, Copilot.
    Rules,
    /// A skill, read when its task comes up.
    Skill,
    /// The definition of an agent.
    Agent,
    /// A command, read when it is run.
    Command,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Instructions => "instructions",
            Role::Rules => "rules",
            Role::Skill => "skill",
            Role::Agent => "agent",
            Role::Command => "command",
        }
    }
}

const INSTRUCTIONS: [&str; 4] = ["CLAUDE.md", "CLAUDE.local.md", "AGENTS.md", "GEMINI.md"];
const RULES: [&str; 3] = [".cursorrules", ".windsurfrules", "copilot-instructions.md"];

/// The role of a file, by its path relative to the walk; none for a file
/// that is no context of an agent.
pub fn role(path: &Path) -> Option<Role> {
    let name = path.file_name()?.to_str()?;
    let dirs: Vec<&str> = path
        .parent()
        .into_iter()
        .flat_map(Path::iter)
        .filter_map(|d| d.to_str())
        .collect();
    let under = |a: &str, b: &str| dirs.windows(2).any(|w| w == [a, b]);
    let markdown = name.ends_with(".md") || name.ends_with(".mdc");
    if INSTRUCTIONS.contains(&name) {
        Some(Role::Instructions)
    } else if RULES.contains(&name)
        || (markdown && (under(".cursor", "rules") || under(".github", "instructions")))
    {
        Some(Role::Rules)
    } else if !markdown {
        None
    } else if under(".claude", "skills") || name == "SKILL.md" {
        Some(Role::Skill)
    } else if under(".claude", "agents") {
        Some(Role::Agent)
    } else if under(".claude", "commands") {
        Some(Role::Command)
    } else {
        None
    }
}

/// Approximate tokens of a text: a token is about four characters of
/// English prose.
pub fn tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

/// Words that ask for something, in English and Spanish.
const ASKING: [&str; 19] = [
    "must",
    "never",
    "always",
    "do not",
    "don't",
    "should",
    "required",
    "forbidden",
    "prohibited",
    "important",
    "debe",
    "debes",
    "nunca",
    "siempre",
    "no uses",
    "no hagas",
    "prohibido",
    "obligatorio",
    "importante",
];

/// The lines of prose of a text, numbered from 1: outside fenced code.
fn prose(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut fenced = false;
    text.lines().enumerate().filter_map(move |(i, line)| {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            return None;
        }
        (!fenced).then_some((i + 1, line))
    })
}

/// Lines of prose that ask for something: a rough count of the rules.
pub fn rules(text: &str) -> usize {
    prose(text)
        .filter(|(_, line)| {
            let line = line.to_lowercase();
            ASKING.iter().any(|word| has_word(&line, word))
        })
        .count()
}

fn has_word(line: &str, word: &str) -> bool {
    line.match_indices(word).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + word.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// Lines shorter than this, once normalized, are too common to count as
/// repeated.
const MIN_REPEATED: usize = 30;

/// A line of prose as compared: without list and quote markers, emphasis,
/// case and repeated spaces.
fn normalized(line: &str) -> String {
    let trimmed = line
        .trim()
        .trim_start_matches(['-', '*', '+', '>', '#'])
        .trim_start_matches(|c: char| c.is_ascii_digit())
        .trim_start_matches(['.', ')'])
        .trim();
    let words: Vec<String> = trimmed
        .split_whitespace()
        .map(|w| w.trim_matches(['*', '_', '`']).to_lowercase())
        .collect();
    words.join(" ")
}

/// A line found in several context files.
#[derive(Debug, PartialEq, Serialize)]
pub struct Repeated {
    pub text: String,
    /// The files and lines where it is, by index of the files given.
    pub places: Vec<(usize, usize)>,
}

/// The lines of prose found in two or more of `texts`, most repeated
/// first.
pub fn repeated(texts: &[&str]) -> Vec<Repeated> {
    let mut seen: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    for (file, text) in texts.iter().enumerate() {
        for (number, line) in prose(text) {
            let line = normalized(line);
            if line.chars().count() >= MIN_REPEATED {
                seen.entry(line).or_default().push((file, number));
            }
        }
    }
    let mut found: Vec<Repeated> = seen
        .into_iter()
        .filter(|(_, places)| places.iter().map(|p| p.0).collect::<BTreeSet<_>>().len() > 1)
        .map(|(text, places)| Repeated { text, places })
        .collect();
    found.sort_by(|a, b| {
        b.places
            .len()
            .cmp(&a.places.len())
            .then_with(|| a.text.cmp(&b.text))
    });
    found
}

/// The agents whose instructions a file is: Claude reads `CLAUDE.md`,
/// Codex and others `AGENTS.md`, Gemini `GEMINI.md`.
fn family(path: &Path) -> &'static str {
    match path.file_name().and_then(|n| n.to_str()) {
        Some("AGENTS.md") => "agents",
        Some("GEMINI.md") => "gemini",
        _ => "claude",
    }
}

/// What an agent working in a directory reads together: the instructions
/// of its family from the root of the walk down to it.
#[derive(Debug, PartialEq)]
pub struct Load {
    pub dir: PathBuf,
    /// `claude`, `agents` or `gemini`.
    pub family: &'static str,
    /// Indexes of the files, from the root down.
    pub files: Vec<usize>,
    pub tokens: usize,
}

/// The loads of every directory that holds instructions, for each family,
/// largest first. `files` are the paths relative to the walk, with their
/// role and tokens.
pub fn loads(files: &[(PathBuf, Role, usize)]) -> Vec<Load> {
    let instructions: Vec<(usize, &Path, &'static str)> = files
        .iter()
        .enumerate()
        .filter(|(_, (_, role, _))| *role == Role::Instructions)
        .map(|(i, (path, _, _))| (i, path.parent().unwrap_or(Path::new("")), family(path)))
        .collect();
    let places: BTreeSet<(&Path, &'static str)> = instructions
        .iter()
        .map(|(_, dir, fam)| (*dir, *fam))
        .collect();
    let mut loads: Vec<Load> = places
        .into_iter()
        .map(|(dir, fam)| {
            let mut read: Vec<usize> = instructions
                .iter()
                .filter(|(_, above, f)| *f == fam && dir.starts_with(above))
                .map(|(i, _, _)| *i)
                .collect();
            read.sort_by_key(|&i| (files[i].0.components().count(), i));
            Load {
                dir: dir.to_path_buf(),
                family: fam,
                tokens: read.iter().map(|&i| files[i].2).sum(),
                files: read,
            }
        })
        .collect();
    loads.sort_by(|a, b| {
        b.tokens
            .cmp(&a.tokens)
            .then_with(|| a.dir.cmp(&b.dir))
            .then_with(|| a.family.cmp(b.family))
    });
    loads
}

#[cfg(test)]
#[path = "context_test.rs"]
mod tests;
