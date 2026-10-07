//! A GitHub pull request as the change to measure, through the GitHub CLI.
//!
//! `gh` is run as a program: kimun links no GitHub client and stores no
//! token. The pull request is measured from its own commits when this
//! repository has them, and from its patch otherwise.

use std::error::Error;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use serde::Deserialize;

/// The GitHub CLI, as found in `PATH`.
pub const GH: &str = "gh";

#[derive(Debug, Deserialize)]
struct Commit {
    oid: String,
}

/// What `gh pr view` says about the commits of a pull request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrInfo {
    base_ref_oid: String,
    head_ref_oid: String,
    /// The commit that merged it, once merged.
    merge_commit: Option<Commit>,
}

/// How the changes of a pull request can be obtained.
#[derive(Debug, PartialEq)]
pub enum Plan {
    /// From two commits of this repository: its base and its head.
    Refs { since: String, until: String },
    /// From the patch GitHub serves, read against the tree of the commit
    /// that merged it, with the history ending at the commit `before` it.
    Merged { merge: String, before: String },
    /// From the patch GitHub serves, read against the working tree, with
    /// the history ending at `base` when this repository has that commit.
    Patch { base: Option<String> },
}

/// Choose how to measure a pull request, given which commits are local.
///
/// With its base and its head here, the two are compared. Once merged by a
/// squash or a rebase the head is usually gone, and nothing local delimits
/// the change: the base GitHub reports may be many merges behind the commit
/// that merged it, and comparing those two would count every pull request
/// merged in between. The patch GitHub serves is the change exactly; it is
/// read against the tree right after the merge, which holds it.
pub fn plan(
    info: &PrInfo,
    is_local: impl Fn(&str) -> bool,
    first_parent: impl Fn(&str) -> Option<String>,
) -> Plan {
    let base = info.base_ref_oid.as_str();
    if is_local(base) && is_local(&info.head_ref_oid) {
        return Plan::Refs {
            since: base.to_string(),
            until: info.head_ref_oid.clone(),
        };
    }
    let merged = info.merge_commit.as_ref().map(|c| c.oid.as_str());
    match merged.and_then(|merge| Some((merge, first_parent(merge)?))) {
        Some((merge, before)) => Plan::Merged {
            merge: merge.to_string(),
            before,
        },
        None => Plan::Patch {
            base: is_local(base).then(|| base.to_string()),
        },
    }
}

/// Run `program` with `args` in `dir` and return what it printed.
fn run(program: &str, dir: &Path, args: &[&str]) -> Result<Vec<u8>, Box<dyn Error>> {
    let output = Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => format!(
                "--pr requires the GitHub CLI (`{program}`) installed and authenticated: \
                 https://cli.github.com"
            ),
            _ => format!("cannot run `{program}`: {e}"),
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("`{program} {}` failed: {}", args.join(" "), stderr.trim()).into());
    }
    Ok(output.stdout)
}

/// The commits of pull request `number` of the repository at `dir`.
pub fn info(program: &str, dir: &Path, number: u64) -> Result<PrInfo, Box<dyn Error>> {
    let number = number.to_string();
    let fields = "baseRefOid,headRefOid,mergeCommit";
    let json = run(program, dir, &["pr", "view", &number, "--json", fields])?;
    Ok(serde_json::from_slice(&json)
        .map_err(|e| format!("unexpected answer from `{program} pr view`: {e}"))?)
}

/// The patch of pull request `number`, in git format.
pub fn patch(program: &str, dir: &Path, number: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    run(
        program,
        dir,
        &["pr", "diff", &number.to_string(), "--color=never"],
    )
}

#[cfg(test)]
#[path = "pr_test.rs"]
mod tests;
