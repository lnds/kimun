//! `km docs context`: the context files read, and the output of the files,
//! the loads and the repeated lines.

use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;

use chrono::DateTime;
use serde::Serialize;

use super::context::{Load, Repeated, Role};
use super::links_report::table;
use crate::cli::OutputMode;
use crate::git::GitRepo;
use crate::report_helpers;
use crate::walk::WalkConfig;

/// A context file and what is measured of it.
pub struct ContextFile {
    /// As walked, for display.
    pub path: PathBuf,
    pub role: Role,
    pub tokens: usize,
    pub rules: usize,
    pub dead: usize,
    /// Commits and last change, in a git repository.
    pub commits: Option<usize>,
    pub changed: Option<i64>,
}

pub struct ContextReport {
    pub files: Vec<ContextFile>,
    pub loads: Vec<Load>,
    pub repeated: Vec<Repeated>,
}

impl ContextReport {
    /// A file as reports show it, under its own name though it links to
    /// another.
    fn shown(&self, file: usize) -> String {
        let path = &self.files[file].path;
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        match path.parent().map(report_helpers::shown) {
            Some(dir) if !dir.is_empty() && dir != "." => format!("{dir}/{name}"),
            _ => name.into_owned(),
        }
    }

    fn load_files(&self, load: &Load) -> Vec<String> {
        load.files.iter().map(|&f| self.shown(f)).collect()
    }
}

/// The context files of coding agents: their size, rules, repetition,
/// dead references and change.
pub fn run(
    cfg: &WalkConfig<'_>,
    output: OutputMode,
    top: Option<usize>,
    max_tokens: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    let root = super::repository_root(cfg.path)?;
    let repository = super::repository(&root);
    let history = GitRepo::open(&root)
        .ok()
        .and_then(|git| git.commits_by_file().ok());
    let mut files = Vec::new();
    let mut texts = Vec::new();
    let mut roles = Vec::new();
    let mut read = HashSet::new();
    let mut unique = Vec::new();
    for entry in crate::walk::walk(cfg.path, cfg.exclude_tests(), cfg.filter).flatten() {
        let path = entry.path();
        let relative = path.strip_prefix(cfg.path).unwrap_or(path);
        let Some(role) = super::context::role(relative).filter(|_| path.is_file()) else {
            continue;
        };
        let full = path.canonicalize()?;
        // A link to a file already read, such as AGENTS.md to CLAUDE.md,
        // repeats nothing.
        if read.insert(full.clone()) {
            unique.push(files.len());
        }
        let text = std::fs::read_to_string(&full).unwrap_or_default();
        let commits = history
            .as_ref()
            .map(|h| h.get(full.strip_prefix(&root).unwrap_or(&full)));
        files.push(ContextFile {
            path: path.to_path_buf(),
            role,
            tokens: super::context::tokens(&text),
            rules: super::context::rules(&text),
            dead: super::refs::check(&full, &text, &repository).dead.len(),
            commits: commits.map(|c| c.map_or(0, Vec::len)),
            changed: commits.flatten().and_then(|c| c.iter().map(|c| c.1).max()),
        });
        roles.push((relative.to_path_buf(), role, super::context::tokens(&text)));
        texts.push(text);
    }
    let report = ContextReport {
        loads: super::context::loads(&roles),
        repeated: super::context::repeated(
            &unique
                .iter()
                .map(|&i| texts[i].as_str())
                .collect::<Vec<_>>(),
        )
        .into_iter()
        .map(|mut r| {
            r.places.iter_mut().for_each(|p| p.0 = unique[p.0]);
            r
        })
        .collect(),
        files,
    };
    match output {
        OutputMode::Json => print_json(&report)?,
        OutputMode::Short => print_short(&report),
        OutputMode::Terse => println!("{}", report.loads.first().map_or(0, |l| l.tokens)),
        OutputMode::Github | OutputMode::Codeclimate => {
            return Err(crate::cli::ERR_CI_FORMAT_ONLY.into());
        }
        OutputMode::Table => print_report(&report, top),
    }
    let over: Vec<String> = report
        .loads
        .iter()
        .filter(|l| max_tokens.is_some_and(|max| l.tokens > max))
        .map(|l| format!("{} ({} tokens)", l.dir.display(), l.tokens))
        .collect();
    match over.is_empty() {
        true => Ok(()),
        false => Err(format!("loads over --max-tokens: {}", over.join(", ")).into()),
    }
}

fn date(time: Option<i64>) -> String {
    time.and_then(|t| DateTime::from_timestamp(t, 0))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn count(value: Option<usize>) -> String {
    value.map_or_else(|| "-".to_string(), |v| v.to_string())
}

pub fn print_report(report: &ContextReport, top: Option<usize>) {
    if report.files.is_empty() {
        println!("No context files of coding agents found.");
        return;
    }
    let limit = |total| report_helpers::shown_count(OutputMode::Table, top, total);
    let files: Vec<[String; 7]> = report.files[..limit(report.files.len())]
        .iter()
        .enumerate()
        .map(|(i, f)| {
            [
                report.shown(i),
                f.role.name().to_string(),
                f.tokens.to_string(),
                f.rules.to_string(),
                f.dead.to_string(),
                count(f.commits),
                date(f.changed),
            ]
        })
        .collect();
    table(
        "Agent context — files loaded into a coding agent's context (tokens ≈ characters / 4)",
        [
            "File", "Role", "Tokens", "Rules", "Dead", "Commits", "Changed",
        ],
        &files,
    );
    let loads: Vec<[String; 4]> = report.loads[..limit(report.loads.len())]
        .iter()
        .map(|l| {
            let dir = match l.dir.as_os_str().is_empty() {
                true => ".".to_string(),
                false => l.dir.display().to_string(),
            };
            [
                dir,
                l.family.to_string(),
                l.tokens.to_string(),
                report.load_files(l).join(", "),
            ]
        })
        .collect();
    table(
        "Loads — the instructions an agent working in a directory reads together",
        ["Directory", "Agent", "Tokens", "Files"],
        &loads,
    );
    let repeated: Vec<[String; 2]> = report.repeated[..limit(report.repeated.len())]
        .iter()
        .map(|r| {
            let text: String = r.text.chars().take(60).collect();
            let mut files: Vec<String> = r.places.iter().map(|p| report.shown(p.0)).collect();
            files.dedup();
            let more = match files.len() > 2 {
                true => format!(" (+{})", files.len() - 2),
                false => String::new(),
            };
            files.truncate(2);
            [text, files.join(", ") + &more]
        })
        .collect();
    table(
        "Repeated — lines found in more than one context file",
        ["Line", "Files"],
        &repeated,
    );
}

#[derive(Serialize)]
struct JsonFile {
    path: String,
    role: Role,
    tokens: usize,
    rules: usize,
    dead_references: usize,
    commits: Option<usize>,
    changed: Option<String>,
}

#[derive(Serialize)]
struct JsonLoad {
    directory: String,
    agent: &'static str,
    tokens: usize,
    files: Vec<String>,
}

#[derive(Serialize)]
struct JsonPlace {
    file: String,
    line: usize,
}

#[derive(Serialize)]
struct JsonRepeated {
    text: String,
    places: Vec<JsonPlace>,
}

#[derive(Serialize)]
struct JsonContext {
    files: Vec<JsonFile>,
    loads: Vec<JsonLoad>,
    repeated: Vec<JsonRepeated>,
}

pub fn print_json(report: &ContextReport) -> Result<(), Box<dyn Error>> {
    let json = JsonContext {
        files: report
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| JsonFile {
                path: report.shown(i),
                role: f.role,
                tokens: f.tokens,
                rules: f.rules,
                dead_references: f.dead,
                commits: f.commits,
                changed: f.changed.map(|t| date(Some(t))),
            })
            .collect(),
        loads: report
            .loads
            .iter()
            .map(|l| JsonLoad {
                directory: l.dir.display().to_string(),
                agent: l.family,
                tokens: l.tokens,
                files: report.load_files(l),
            })
            .collect(),
        repeated: report
            .repeated
            .iter()
            .map(|r| JsonRepeated {
                text: r.text.clone(),
                places: r
                    .places
                    .iter()
                    .map(|&(file, line)| JsonPlace {
                        file: report.shown(file),
                        line,
                    })
                    .collect(),
            })
            .collect(),
    };
    report_helpers::print_json_stdout(&json)
}

pub fn print_short(report: &ContextReport) {
    println!(
        "context files:{} tokens:{} max_load:{} rules:{} repeated:{} dead:{}",
        report.files.len(),
        report.files.iter().map(|f| f.tokens).sum::<usize>(),
        report.loads.first().map_or(0, |l| l.tokens),
        report.files.iter().map(|f| f.rules).sum::<usize>(),
        report.repeated.len(),
        report.files.iter().map(|f| f.dead).sum::<usize>(),
    );
}
