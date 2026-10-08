//! Comments and literals of Elixir source.
//!
//! What a file refers to is read from its code alone, so comments, strings,
//! sigils and character literals are blanked first. The code a string
//! interpolates stays: it runs like any other.

use super::elixir::is_ident;

/// The index past `closer`, looked for from `from`. A string that
/// `interpolates` also honours escapes and skips `#{...}` whole, since the
/// code inside may hold quotes of its own.
fn closing(chars: &[char], from: usize, closer: &[char], interpolates: bool) -> usize {
    let mut i = from;
    while i < chars.len() {
        if interpolates && chars[i] == '\\' {
            i += 2;
        } else if interpolates && chars[i..].starts_with(&['#', '{']) {
            i = interpolation_end(chars, i + 2);
        } else if chars[i..].starts_with(closer) {
            return i + closer.len();
        } else {
            i += 1;
        }
    }
    chars.len()
}

/// The index past the `}` that closes an interpolation opened before `from`.
pub(super) fn interpolation_end(chars: &[char], from: usize) -> usize {
    let mut depth = 1usize;
    let mut i = from;
    while i < chars.len() {
        match chars[i] {
            '"' => {
                i = closing(chars, i + 1, &['"'], true);
                continue;
            }
            '{' => depth += 1,
            '}' if depth == 1 => return i + 1,
            '}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    chars.len()
}

/// The index past the sigil that starts at `i` (`~r/.../`, `~S"""..."""`),
/// or `i` when what follows `~` is not one.
fn sigil_end(chars: &[char], i: usize) -> usize {
    let name_end = i
        + 1
        + chars[i + 1..]
            .iter()
            .take_while(|c| c.is_ascii_alphabetic())
            .count();
    let Some(&open) = chars.get(name_end).filter(|_| name_end > i + 1) else {
        return i;
    };
    // An uppercase sigil takes its content verbatim.
    let interpolates = chars[i + 1].is_ascii_lowercase();
    let heredoc = [open; 3];
    if matches!(open, '"' | '\'') && chars[name_end..].starts_with(&heredoc) {
        return closing(chars, name_end + 3, &heredoc, interpolates);
    }
    let close = match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        '"' | '\'' | '/' | '|' => open,
        _ => return i,
    };
    closing(chars, name_end + 1, &[close], interpolates)
}

/// The index past the comment, string, sigil or character literal that
/// starts at `i`, or `i` when none does.
pub(super) fn literal_end(chars: &[char], i: usize) -> usize {
    let after_ident = i > 0 && is_ident(chars[i - 1]);
    match chars[i..] {
        ['#', ..] => i + chars[i..].iter().take_while(|&&c| c != '\n').count(),
        [q @ ('"' | '\''), b, c, ..] if q == b && q == c => closing(chars, i + 3, &[q; 3], true),
        [q @ ('"' | '\''), ..] => closing(chars, i + 1, &[q], true),
        ['~', ..] => sigil_end(chars, i),
        // `?x` is a character; in `valid?(x)` the mark belongs to the name.
        ['?', '\\', _, ..] if !after_ident => i + 3,
        ['?', _, ..] if !after_ident => i + 2,
        _ => i,
    }
}

/// Blank `chars` into `out`, keeping line breaks.
fn blank_into(chars: &[char], out: &mut String) {
    out.extend(chars.iter().map(|&c| if c == '\n' { '\n' } else { ' ' }));
}

/// Whether the literal `chars` holds interpolations: a string, a charlist
/// or a lowercase sigil. A comment, a character or an uppercase sigil do not.
fn interpolates(chars: &[char]) -> bool {
    match chars {
        ['"' | '\'', ..] => true,
        ['~', name, ..] => name.is_ascii_lowercase(),
        _ => false,
    }
}

/// Blank a literal that interpolates into `out`, but for the code inside
/// each `#{...}`: it runs like any other.
fn blank_around_code(chars: &[char], out: &mut String) {
    let mut i = 0;
    while i < chars.len() {
        let step = if chars[i] == '\\' {
            2
        } else if chars[i..].starts_with(&['#', '{']) {
            let end = interpolation_end(chars, i + 2);
            let closed = usize::from(chars[end - 1] == '}' && end > i + 2);
            blank_into(&chars[i..i + 2], out);
            code_into(&chars[i + 2..end - closed], out);
            blank_into(&chars[end - closed..end], out);
            i = end;
            continue;
        } else {
            1
        };
        let end = (i + step).min(chars.len());
        blank_into(&chars[i..end], out);
        i = end;
    }
}

/// Write `chars` into `out` with comments and literals blanked out.
fn code_into(chars: &[char], out: &mut String) {
    let mut i = 0;
    while i < chars.len() {
        let end = literal_end(chars, i).min(chars.len());
        if end == i {
            out.push(chars[i]);
            i += 1;
        } else if interpolates(&chars[i..end]) {
            blank_around_code(&chars[i..end], out);
            i = end;
        } else {
            blank_into(&chars[i..end], out);
            i = end;
        }
    }
}

/// The source with comments and literals blanked out, but for the code a
/// string interpolates. Line breaks and columns are kept, so indentation
/// still tells how deep a line is.
pub(super) fn code_only(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    code_into(&chars, &mut out);
    out
}
