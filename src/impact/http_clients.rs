//! Calls to HTTP clients, and the path each one asks for.
//!
//! Read lexically, a line at a time: Tesla, Req, Finch and HTTPoison in
//! Elixir, `fetch` and `axios` in JavaScript and TypeScript, `requests` and
//! `httpx` in Python. The path is the literal the call is given, with what
//! is built at run time turned into a parameter. A call whose path is not a
//! literal is kept, without a path: it cannot be matched, and that is said.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// What stands for a segment built at run time.
pub const PARAM: &str = ":param";

/// The verbs a client names its calls by.
const VERBS: [&str; 7] = ["get", "post", "put", "patch", "delete", "head", "options"];

/// Clients called as `Module.verb(` or `module.verb(`, by language family.
const ELIXIR_CLIENTS: [&str; 3] = ["Tesla", "Req", "HTTPoison"];
const SCRIPT_CLIENTS: [&str; 1] = ["axios"];
const PYTHON_CLIENTS: [&str; 2] = ["requests", "httpx"];

/// One call to an HTTP client.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClientCall {
    pub file: PathBuf,
    /// From 1.
    pub line: usize,
    pub client: String,
    /// The verb, when the call tells it.
    pub verb: Option<String>,
    /// The path asked for, normalized: `None` when it is built at run time.
    pub path: Option<String>,
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The text of the literal that opens at the start of `text` (a quote or
/// a backtick), each interpolation or `{placeholder}` turned into `PARAM`.
/// `None` when it is no literal or the line does not close it.
fn literal(text: &str) -> Option<String> {
    let mut chars = text.chars().peekable();
    let quote = chars.next().filter(|c| matches!(c, '"' | '\'' | '`'))?;
    let mut out = String::new();
    while let Some(c) = chars.next() {
        match c {
            c if c == quote => return Some(out),
            '#' | '$' if chars.peek() == Some(&'{') => {}
            '{' => {
                chars.by_ref().find(|&c| c == '}');
                out.push_str(PARAM);
            }
            '\\' => {
                chars.next();
            }
            c => out.push(c),
        }
    }
    None
}

/// A path as it is matched: no scheme or host, no query, and no leading
/// parameters (a base URL written at run time). `None` without a fixed
/// segment.
pub fn normalize(url: &str) -> Option<String> {
    let mut path = url;
    if let Some(at) = path.find("://") {
        let host_and_path = &path[at + 3..];
        path = host_and_path
            .find('/')
            .map_or("", |slash| &host_and_path[slash..]);
    }
    let path = &path[..path.find(['?', '#']).unwrap_or(path.len())];
    let segments: Vec<&str> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .skip_while(|s| s.starts_with(':'))
        .map(|s| if s.contains(PARAM) { PARAM } else { s })
        .collect();
    let fixed = segments.iter().any(|s| !s.starts_with(':'));
    fixed.then(|| format!("/{}", segments.join("/")))
}

/// The path argument of a call whose arguments start at `args`: a `url:`
/// option when there is one, else the first literal.
fn path_argument(args: &str) -> Option<String> {
    let option = args
        .match_indices("url:")
        .find(|(at, _)| !args[..*at].ends_with(is_ident))
        .map(|(at, _)| at);
    let start = match option {
        Some(at) => at + 4 + args[at + 4..].len() - args[at + 4..].trim_start().len(),
        None => args.find(['"', '\'', '`'])?,
    };
    literal(&args[start..])
}

/// The verb a line names besides the call: `method: :post`, `method: "POST"`.
fn method_option(line: &str) -> Option<String> {
    let at = line.find("method:")?;
    let value = line[at + 7..]
        .trim_start()
        .trim_start_matches([':', '"', '\'']);
    let verb: String = value.chars().take_while(|c| c.is_alphabetic()).collect();
    let verb = verb.to_lowercase();
    VERBS.contains(&verb.as_str()).then_some(verb)
}

/// The name of the function called right before the `(` at `open`.
fn callee(line: &str, open: usize) -> &str {
    let before = line[..open].trim_end_matches('!');
    let start = before
        .rfind(|c: char| !is_ident(c) && c != '.')
        .map_or(0, |i| i + 1);
    &before[start..]
}

/// The client and verb a callee names, if it is a call to a client.
fn client_of(callee: &str, language: &str, bare_tesla: bool) -> Option<(String, Option<String>)> {
    let (module, function) = callee.rsplit_once('.').unwrap_or(("", callee));
    let verb = VERBS.contains(&function).then(|| function.to_string());
    let clients: &[&str] = match language {
        "Elixir" | "Elixir Script" => &ELIXIR_CLIENTS,
        "Python" => &PYTHON_CLIENTS,
        _ => &SCRIPT_CLIENTS,
    };
    if clients.contains(&module) && (verb.is_some() || function == "request") {
        return Some((module.to_string(), verb));
    }
    match (module, function) {
        ("Finch", "build") => Some(("Finch".to_string(), None)),
        ("", "fetch") if !language.starts_with("Elixir") && language != "Python" => {
            Some(("fetch".to_string(), None))
        }
        ("", "axios") if !language.starts_with("Elixir") => Some(("axios".to_string(), None)),
        ("", _) if bare_tesla && verb.is_some() => Some(("Tesla".to_string(), verb)),
        _ => None,
    }
}

/// The verb of a `Finch.build(:get, ...)`, given its arguments.
fn finch_verb(args: &str) -> Option<String> {
    let verb: String = args
        .trim_start_matches(['(', ' '])
        .strip_prefix(':')?
        .chars()
        .take_while(|c| c.is_alphabetic())
        .collect();
    VERBS.contains(&verb.as_str()).then_some(verb)
}

/// The calls to HTTP clients a source in `language` makes.
pub fn calls(file: &Path, language: &str, text: &str) -> Vec<ClientCall> {
    let bare_tesla = language.starts_with("Elixir") && text.contains("use Tesla");
    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let code = line.trim_start();
        if code.starts_with(['#', '/', '*']) {
            continue;
        }
        for (open, _) in line.match_indices('(') {
            let Some((client, verb)) = client_of(callee(line, open), language, bare_tesla) else {
                continue;
            };
            let args = &line[open + 1..];
            let verb = verb
                .or_else(|| (client == "Finch").then(|| finch_verb(args)).flatten())
                .or_else(|| method_option(args));
            found.push(ClientCall {
                file: file.to_path_buf(),
                line: number + 1,
                client,
                verb,
                path: path_argument(args).and_then(|p| normalize(&p)),
            });
        }
    }
    found
}

#[cfg(test)]
#[path = "http_clients_test.rs"]
mod tests;
