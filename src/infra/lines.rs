//! Lines of infrastructure code as the smells read them: the code apart
//! from the comment, by format, and the assignments the code makes.

/// How a file of infrastructure is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Terraform and other HCL: `#`, `//` and `/* */` comments, heredocs.
    Hcl,
    /// A Dockerfile: a comment is a whole line that starts with `#`.
    Dockerfile,
    /// YAML: `#` after a space or at the start, outside quotes.
    Yaml,
    /// Any other language declared infrastructure: `#` comments.
    Other,
}

impl Format {
    pub fn of(language: &str) -> Format {
        match language {
            "Terraform" => Format::Hcl,
            "Dockerfile" => Format::Dockerfile,
            "YAML" => Format::Yaml,
            _ => Format::Other,
        }
    }
}

/// A line split into its code and its comment, numbered from 1.
#[derive(Debug, PartialEq)]
pub struct Line<'a> {
    pub number: usize,
    pub code: &'a str,
    pub comment: &'a str,
}

/// The lines of a text, each split into code and comment.
pub fn split(text: &str, format: Format) -> Vec<Line<'_>> {
    let mut state = State::Code;
    text.lines()
        .enumerate()
        .map(|(i, line)| {
            let (code, comment) = match format {
                Format::Hcl => hcl(line, &mut state),
                Format::Dockerfile if line.trim_start().starts_with('#') => ("", line),
                Format::Dockerfile => (line, ""),
                Format::Yaml | Format::Other => hash_comment(line, format == Format::Yaml),
            };
            Line {
                number: i + 1,
                code,
                comment,
            }
        })
        .collect()
}

/// Where a line of HCL starts.
enum State {
    Code,
    /// Inside `/* */`.
    Block,
    /// Inside a heredoc, until the line that holds only its marker.
    Heredoc(String),
}

fn hcl<'a>(line: &'a str, state: &mut State) -> (&'a str, &'a str) {
    match std::mem::replace(state, State::Code) {
        State::Heredoc(marker) => {
            if line.trim() != marker {
                *state = State::Heredoc(marker);
            }
            (line, "")
        }
        State::Block => match line.find("*/") {
            Some(end) => (line[end + 2..].trim_start(), &line[..end + 2]),
            None => {
                *state = State::Block;
                ("", line)
            }
        },
        State::Code => {
            let (code, comment, next) = hcl_code(line);
            *state = next;
            (code, comment)
        }
    }
}

/// A line of HCL that starts in code: its code, its comment and where the
/// next line starts.
fn hcl_code(line: &str) -> (&str, &str, State) {
    let opens = |rest: &str| ["#", "//", "/*", "<<"].iter().any(|m| rest.starts_with(m));
    let Some(at) = outside_quotes(line, opens) else {
        return (line, "", State::Code);
    };
    let rest = &line[at..];
    if let Some(marker) = rest.strip_prefix("<<") {
        return (line, "", heredoc(marker));
    }
    let next = match rest.starts_with("/*") && !rest.contains("*/") {
        true => State::Block,
        false => State::Code,
    };
    (&line[..at], rest, next)
}

/// The first place outside double quotes where `starts` holds.
fn outside_quotes(line: &str, starts: impl Fn(&str) -> bool) -> Option<usize> {
    let mut quoted = false;
    let mut escaped = false;
    for (at, c) in line.char_indices() {
        if !quoted && starts(&line[at..]) {
            return Some(at);
        }
        quoted ^= c == '"' && !escaped;
        escaped = c == '\\' && !escaped;
    }
    None
}

/// The state a heredoc opens: `<<EOF` and `<<-EOF` end at a line that
/// holds only `EOF`.
fn heredoc(marker: &str) -> State {
    let marker = marker.trim_start_matches('-').trim();
    match !marker.is_empty() && marker.chars().all(|c| c.is_alphanumeric() || c == '_') {
        true => State::Heredoc(marker.to_string()),
        false => State::Code,
    }
}

/// `#` comments outside quotes, when at the start or after a space. In
/// YAML a quote opens a string only where a value starts, so an apostrophe
/// inside a plain scalar opens nothing.
fn hash_comment(line: &str, yaml: bool) -> (&str, &str) {
    let mut quote: Option<char> = None;
    let mut previous = ' ';
    for (at, c) in line.char_indices() {
        if let Some(open) = quote {
            quote = quote.filter(|_| c != open || previous == '\\');
        } else if c == '#' && previous.is_whitespace() {
            return (&line[..at], &line[at..]);
        } else if matches!(c, '"' | '\'') && (!yaml || opens_value(&line[..at])) {
            quote = Some(c);
        }
        previous = c;
    }
    (line, "")
}

fn opens_value(before: &str) -> bool {
    before
        .trim_end()
        .chars()
        .next_back()
        .is_none_or(|c| matches!(c, ':' | '[' | '{' | ',' | '-' | '='))
}

/// The value of an assignment as written.
#[derive(Debug, PartialEq)]
pub struct Value<'a> {
    /// Without its quotes.
    pub text: &'a str,
    pub quoted: bool,
}

/// The assignments of a line of code: `key: value` in YAML, `key = value`
/// in HCL, `KEY=value` in shell and `ENV`/`ARG` of a Dockerfile.
pub fn assignments(code: &str) -> Vec<(&str, Value<'_>)> {
    let trimmed = code.trim();
    let instruction = trimmed
        .split_once(char::is_whitespace)
        .filter(|(word, _)| matches!(word.to_ascii_uppercase().as_str(), "ENV" | "ARG"));
    match instruction {
        Some((_, rest)) if !rest.contains('=') => rest
            .trim()
            .split_once(char::is_whitespace)
            .map(|(key, value)| (key, value_of(value.trim())))
            .into_iter()
            .collect(),
        Some((_, rest)) => rest
            .split_whitespace()
            .filter_map(|pair| pair.split_once('='))
            .map(|(key, value)| (key, value_of(value)))
            .collect(),
        None => assignment(trimmed).into_iter().collect(),
    }
}

fn assignment(code: &str) -> Option<(&str, Value<'_>)> {
    let bytes = code.as_bytes();
    let at = (0..bytes.len()).find(|&i| match bytes[i] {
        b'=' => {
            let before = i.checked_sub(1).map(|j| bytes[j]);
            let after = bytes.get(i + 1);
            !matches!(before, Some(b'=' | b'!' | b'<' | b'>'))
                && !matches!(after, Some(b'=' | b'>'))
        }
        b':' => bytes.get(i + 1).is_none_or(|b| *b == b' '),
        _ => false,
    })?;
    let before = code[..at].trim_end();
    let unquoted = before.trim_end_matches(['"', '\'']);
    let start = unquoted
        .rfind(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '-' | '.')))
        .map_or(0, |i| i + 1);
    let key = &unquoted[start..];
    match key.is_empty() {
        true => None,
        false => Some((key, value_of(code[at + 1..].trim()))),
    }
}

fn value_of(text: &str) -> Value<'_> {
    let text = text.trim_end_matches([',', ';']).trim();
    for quote in ['"', '\''] {
        if let Some(inner) = text.strip_prefix(quote).and_then(|t| t.strip_suffix(quote)) {
            return Value {
                text: inner,
                quoted: true,
            };
        }
    }
    Value {
        text,
        quoted: false,
    }
}

/// The words of a text in lower case: runs of letters and digits.
pub fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
}

#[cfg(test)]
#[path = "lines_test.rs"]
mod tests;
