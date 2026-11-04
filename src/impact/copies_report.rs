//! The copies of changed values, as text and as JSON.

use serde::Serialize;

use super::copies::Copy;

const TITLE: &str = "Copies of changed values — groups outside the change that hold the old ones";

/// How many copies the text lists.
const SHOWN: usize = 10;

/// What a copy lacks or still holds, in words.
fn behind(copy: &Copy) -> String {
    let mut parts = Vec::new();
    if !copy.missing.is_empty() {
        parts.push(format!("lacks {}", copy.missing.join(", ")));
    }
    if !copy.kept.is_empty() {
        parts.push(format!("still holds {}", copy.kept.join(", ")));
    }
    parts.join("; ")
}

/// The block of the report; empty when no copy was found.
pub fn render(copies: &[Copy]) -> Vec<String> {
    if copies.is_empty() {
        return Vec::new();
    }
    let sep = crate::report_helpers::separator(78);
    let mut lines = vec![TITLE.to_string(), sep.clone()];
    for copy in copies.iter().take(SHOWN) {
        let tie = if copy.mentions {
            ", says it mirrors it"
        } else {
            ""
        };
        lines.push(format!(
            " {}:{}  {} of {} values of {}:{}{tie}",
            copy.file.display(),
            copy.line,
            copy.matched.len(),
            copy.matched.len() + copy.missing.len(),
            copy.of.display(),
            copy.of_line,
        ));
        lines.push(format!("            {}", behind(copy)));
    }
    if copies.len() > SHOWN {
        lines.push(format!(" {} copies ({SHOWN} shown).", copies.len()));
    }
    lines.push(sep);
    lines.push("Matched by their values alone: a hint, not a proof.".to_string());
    lines
}

/// The copies in the JSON report.
#[derive(Serialize)]
pub struct JsonCopies<'a> {
    /// Matching is by literal values: a heuristic.
    heuristic: bool,
    copies: &'a [Copy],
}

impl<'a> From<&'a [Copy]> for JsonCopies<'a> {
    fn from(copies: &'a [Copy]) -> Self {
        JsonCopies {
            heuristic: true,
            copies,
        }
    }
}
