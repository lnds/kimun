//! Secrets written into infrastructure code: a literal assigned to a key
//! named as a secret, or a value shaped like a known kind of key.

use super::lines::{Format, Value};

/// The last word of a key that names a secret.
const SECRET_WORDS: [&str; 9] = [
    "password",
    "passwd",
    "pwd",
    "passphrase",
    "secret",
    "token",
    "apikey",
    "credential",
    "credentials",
];

/// The word before `key` that makes it a secret.
const KEY_KINDS: [&str; 7] = [
    "api",
    "access",
    "secret",
    "private",
    "signing",
    "encryption",
    "master",
];

const PASSWORD_WORDS: [&str; 4] = ["password", "passwd", "pwd", "passphrase"];

/// The words of a key, in lower case: split at `_`, `-`, `.` and where
/// camel case starts a word.
fn key_words(key: &str) -> Vec<String> {
    let mut words = vec![String::new()];
    let mut previous_lower = false;
    for c in key.chars() {
        if matches!(c, '_' | '-' | '.') {
            words.push(String::new());
            previous_lower = false;
            continue;
        }
        if c.is_uppercase() && previous_lower {
            words.push(String::new());
        }
        previous_lower = c.is_lowercase() || c.is_ascii_digit();
        if let Some(word) = words.last_mut() {
            word.extend(c.to_lowercase());
        }
    }
    words.retain(|w| !w.is_empty());
    words
}

/// Whether a key names a secret by its last word: `db_password`,
/// `clientSecret`, `AWS_SECRET_ACCESS_KEY`, but not `secret_name`.
pub fn names_secret(key: &str) -> bool {
    match key_words(key).as_slice() {
        [.., last] if SECRET_WORDS.contains(&last.as_str()) => true,
        [.., kind, last] => last == "key" && KEY_KINDS.contains(&kind.as_str()),
        _ => false,
    }
}

/// Whether a key names a password.
pub fn names_password(key: &str) -> bool {
    key_words(key)
        .last()
        .is_some_and(|w| PASSWORD_WORDS.contains(&w.as_str()))
}

/// Whether a value is written as it is, not read from elsewhere: a
/// variable, an interpolation, a reference of a template, a YAML tag or
/// alias, a path, a flag or a number all come from somewhere else.
pub fn is_literal(value: &Value<'_>, format: Format) -> bool {
    let text = value.text.trim();
    if text.is_empty() || (format == Format::Hcl && !value.quoted) {
        return false;
    }
    let read_elsewhere = ["${", "{{", "$(", "%{", "<%", "#{"]
        .iter()
        .any(|m| text.contains(m));
    let starts_aside = text.starts_with(['$', '!', '*', '&', '/', '[', '{', '|', '>', '<', '.']);
    let plain = matches!(
        text.to_ascii_lowercase().as_str(),
        "true" | "false" | "null" | "none" | "~" | "yes" | "no"
    );
    let number = text.chars().all(|c| c.is_ascii_digit() || c == '.');
    !(read_elsewhere || starts_aside || plain || number)
}

/// Prefixes of keys issued by known services, with the characters and the
/// least length of what follows.
const SHAPES: [(&str, &str, usize); 12] = [
    ("AKIA", "an AWS access key", 16),
    ("ASIA", "an AWS access key", 16),
    ("ghp_", "a GitHub token", 36),
    ("gho_", "a GitHub token", 36),
    ("ghs_", "a GitHub token", 36),
    ("ghu_", "a GitHub token", 36),
    ("github_pat_", "a GitHub token", 22),
    ("glpat-", "a GitLab token", 20),
    ("xoxb-", "a Slack token", 10),
    ("xoxp-", "a Slack token", 10),
    ("sk_live_", "a Stripe key", 16),
    ("AIza", "a Google API key", 35),
];

/// What kind of key a line of code holds by its shape, if any.
pub fn shaped(code: &str) -> Option<&'static str> {
    if code.contains("-----BEGIN") && code.contains("PRIVATE KEY") {
        return Some("a private key");
    }
    SHAPES.iter().find_map(|&(prefix, what, least)| {
        code.match_indices(prefix).find_map(|(at, _)| {
            let before = code[..at].chars().next_back();
            let rest = &code[at + prefix.len()..];
            let length = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
                .count();
            let starts_word = !before.is_some_and(|c| c.is_ascii_alphanumeric());
            let aws = !prefix.starts_with('A')
                || prefix == "AIza"
                || rest
                    .chars()
                    .take(least)
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
            (starts_word && aws && length >= least).then_some(what)
        })
    })
}

#[cfg(test)]
#[path = "secrets_test.rs"]
mod tests;
