//! What a change removes, as text: a line of the summary, and the files
//! that still use it among the warnings.

use std::collections::BTreeMap;
use std::path::Path;

use super::removed::Removed;
use super::structural_report::names;

/// What the change removes, for the summary.
pub fn summary(removed: &[Removed]) -> Option<String> {
    let gone: Vec<String> = removed
        .iter()
        .map(|r| match &r.function {
            Some(function) => format!("{function} ({})", r.file.display()),
            None => format!("{} (deleted)", r.file.display()),
        })
        .collect();
    (!gone.is_empty()).then(|| format!(" Removed: {}", names(gone.iter(), 5)))
}

/// The files that still use what the change removes, each with what.
pub fn warning(removed: &[Removed]) -> Vec<String> {
    let mut users: BTreeMap<&Path, Vec<String>> = BTreeMap::new();
    for removed in removed {
        for caller in &removed.callers {
            let what = match caller.calls.is_empty() {
                false => caller.calls.join(", "),
                true => removed.file.display().to_string(),
            };
            users.entry(&caller.file).or_default().push(what);
        }
    }
    if users.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![match users.len() {
        1 => "1 file still uses what the change removes, and will break:".to_string(),
        n => format!("{n} files still use what the change removes, and will break:"),
    }];
    lines.extend(
        users
            .iter()
            .take(5)
            .map(|(file, what)| format!("  {} ({})", file.display(), what.join(", "))),
    );
    if users.len() > 5 {
        lines.push(format!("  and {} more", users.len() - 5));
    }
    lines
}
