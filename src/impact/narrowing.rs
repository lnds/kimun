//! What a change comes to in each changed file: the functions it affects,
//! or why every use of the file counts.

use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;

use crate::deps::graph::{Narrowed, Unnarrowed};
use crate::git::{ChangeKind, FileDiffStat};

/// What a change comes to in one changed file: the public functions it
/// affects, or why they cannot be told and every use of the file counts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Narrowing {
    pub file: PathBuf,
    pub functions: Option<Vec<String>>,
    /// Why not, when `functions` is `None`.
    pub reason: Option<String>,
    /// The module attributes the change touches that functions read, each
    /// with those functions: why they are among `functions`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<AttributeRead>,
}

/// A module attribute a change touches, and the functions that read it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AttributeRead {
    pub name: String,
    pub read_by: Vec<String>,
}

/// Why the change to `source` cannot be narrowed, in words.
fn reason(stat: &FileDiffStat, why: &Unnarrowed) -> String {
    match stat.kind {
        ChangeKind::Added => return "new file".to_string(),
        ChangeKind::Deleted => return "deleted".to_string(),
        _ => {}
    }
    match why {
        Unnarrowed::Language => "its language does not tell functions".to_string(),
        Unnarrowed::NoLines => "no changed line is known".to_string(),
        Unnarrowed::Outside(line) => format!("line {line} is outside every function"),
        Unnarrowed::CompileTime(line, name) => {
            format!("line {line} sets {name}, read when the module compiles")
        }
        Unnarrowed::Unread(line, name) => {
            format!("line {line} sets {name}, which no function reads")
        }
        Unnarrowed::Routes(line) => {
            format!("line {line} declares routes, read when the router compiles")
        }
    }
}

impl Narrowing {
    /// What the change described by `stat` comes to in `file`.
    pub fn of(file: &Path, stat: &FileDiffStat, told: &Result<Narrowed, Unnarrowed>) -> Self {
        match told {
            Ok(narrowed) => Narrowing {
                file: file.to_path_buf(),
                functions: Some(narrowed.functions.iter().cloned().collect()),
                reason: None,
                attributes: narrowed
                    .attributes
                    .iter()
                    .map(|(name, by)| AttributeRead {
                        name: name.clone(),
                        read_by: by.iter().cloned().collect(),
                    })
                    .collect(),
            },
            Err(why) => Narrowing {
                file: file.to_path_buf(),
                functions: None,
                reason: Some(reason(stat, why)),
                attributes: Vec::new(),
            },
        }
    }
}
