//! A badge of the score for a README: a self-contained SVG in the flat
//! style of shields.io, or the JSON its endpoint badge reads.

use std::error::Error;
use std::path::Path;

use serde::Serialize;

use super::analyzer::{Grade, ProjectScore};

const LABEL: &str = "km score";

/// The colour of a grade, by its letter, as shields.io names it and as
/// its flat badges paint it.
fn colour(grade: Grade) -> (&'static str, &'static str) {
    match grade.as_str().chars().next() {
        Some('A') => ("brightgreen", "#4c1"),
        Some('B') => ("green", "#97ca00"),
        Some('C') => ("yellow", "#dfb317"),
        Some('D') => ("orange", "#fe7d37"),
        _ => ("red", "#e05d44"),
    }
}

/// What the badge says: the grade and the score.
fn message(score: &ProjectScore) -> String {
    format!("{} {:.1}", score.grade, score.score)
}

/// Write the badge of `score` to `path`: the endpoint JSON when the file
/// ends in `.json`, an SVG otherwise.
pub fn write(path: &Path, score: &ProjectScore) -> Result<(), Box<dyn Error>> {
    let json = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"));
    let text = match json {
        true => endpoint(score)?,
        false => svg(score),
    };
    std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    let shown = path.display();
    match json {
        true => eprintln!(
            "Badge written to {shown}. Serve it and use https://img.shields.io/endpoint?url=<its URL>"
        ),
        false => {
            eprintln!(
                "Badge written to {shown}. In a README: [![km score]({shown})](https://kimun.tools)"
            )
        }
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Endpoint<'a> {
    schema_version: u8,
    label: &'a str,
    message: String,
    color: &'a str,
}

/// The JSON of a shields.io endpoint badge.
pub fn endpoint(score: &ProjectScore) -> Result<String, Box<dyn Error>> {
    let badge = Endpoint {
        schema_version: 1,
        label: LABEL,
        message: message(score),
        color: colour(score.grade).0,
    };
    Ok(serde_json::to_string_pretty(&badge)? + "\n")
}

/// The width a text takes in 11px Verdana, about: narrow characters are
/// thinner than the rest.
fn text_width(text: &str) -> usize {
    let tenths: usize = text
        .chars()
        .map(|c| match c {
            'i' | 'l' | '.' | ' ' | '|' => 35,
            '+' | '-' => 75,
            c if c.is_uppercase() || c.is_ascii_digit() => 75,
            _ => 65,
        })
        .sum();
    tenths.div_ceil(10)
}

/// A flat badge as an SVG that needs nothing else to render.
pub fn svg(score: &ProjectScore) -> String {
    let message = message(score);
    let colour = colour(score.grade).1;
    let left = text_width(LABEL) + 10;
    let right = text_width(&message) + 10;
    let width = left + right;
    let (lx, mx) = (left * 5, left * 10 + right * 5);
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="20" role="img" aria-label="{LABEL}: {message}"><title>{LABEL}: {message}</title><linearGradient id="s" x2="0" y2="100%"><stop offset="0" stop-color="#bbb" stop-opacity=".1"/><stop offset="1" stop-opacity=".1"/></linearGradient><clipPath id="r"><rect width="{width}" height="20" rx="3" fill="#fff"/></clipPath><g clip-path="url(#r)"><rect width="{left}" height="20" fill="#555"/><rect x="{left}" width="{right}" height="20" fill="{colour}"/><rect width="{width}" height="20" fill="url(#s)"/></g><g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" font-size="110"><text x="{lx}" y="150" fill="#010101" fill-opacity=".3" transform="scale(.1)">{LABEL}</text><text x="{lx}" y="140" transform="scale(.1)">{LABEL}</text><text x="{mx}" y="150" fill="#010101" fill-opacity=".3" transform="scale(.1)">{message}</text><text x="{mx}" y="140" transform="scale(.1)">{message}</text></g></svg>
"##
    )
}

#[cfg(test)]
#[path = "badge_test.rs"]
mod tests;
